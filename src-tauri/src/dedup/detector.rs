use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::dedup::cache::FingerprintCache;
use crate::dedup::fingerprint;
use crate::dedup::{DuplicateFile, DuplicateGroup, DuplicateMode, DuplicateReport, ResolveOutcome};
use crate::error::Result;
use crate::models::media_file::MediaFile;
use crate::models::track::TrackMetadata;

/// Strip the noise that stops otherwise-identical recordings from matching:
/// case, punctuation, a leading article, and parenthetical suffixes such as
/// "(Remastered 2011)" or "[Live]".
pub fn normalize(value: &str) -> String {
    let mut s = value.to_lowercase();

    // Drop bracketed suffixes.
    while let Some(open) = s.rfind(['(', '[']) {
        let close = match s[open..].find([')', ']']) {
            Some(offset) => open + offset,
            None => break,
        };
        if close + 1 >= s.len() {
            s.replace_range(open..=close, "");
            s = s.trim().to_string();
        } else {
            break;
        }
    }

    let s = s.trim_start_matches("the ").to_string();

    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// SHA-256 of the whole file, streamed so large files do not land in memory.
pub(crate) fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{:02x}", b)).collect())
}

fn to_duplicate_file(
    file: &MediaFile,
    metadata: Option<&TrackMetadata>,
    score: f64,
) -> DuplicateFile {
    DuplicateFile {
        path: file.path.clone(),
        file_name: file.file_name.clone(),
        file_size: file.file_size,
        duration_ms: file.duration_ms,
        bitrate_kbps: file.bitrate_kbps,
        format: file.audio_format.as_ref().map(|f| f.as_str().to_string()),
        title: metadata.and_then(|m| m.title.clone()),
        artist: metadata.and_then(|m| m.artist.clone()),
        album: metadata.and_then(|m| m.album.clone()),
        score,
        recommended_keep: false,
    }
}

/// Prefer the highest bitrate, breaking ties on file size.
fn mark_recommended(files: &mut [DuplicateFile]) {
    let best = files
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.bitrate_kbps
                .unwrap_or(0)
                .cmp(&b.bitrate_kbps.unwrap_or(0))
                .then(a.file_size.cmp(&b.file_size))
        })
        .map(|(i, _)| i);

    if let Some(i) = best {
        files[i].recommended_keep = true;
    }
}

fn finish_group(
    id: String,
    mode: DuplicateMode,
    reason: String,
    mut files: Vec<DuplicateFile>,
) -> DuplicateGroup {
    mark_recommended(&mut files);

    // Everything except the kept copy is reclaimable.
    let kept = files
        .iter()
        .find(|f| f.recommended_keep)
        .map(|f| f.file_size)
        .unwrap_or(0);
    let wasted_bytes = files.iter().map(|f| f.file_size).sum::<u64>() - kept;

    DuplicateGroup {
        id,
        mode,
        reason,
        files,
        wasted_bytes,
    }
}

/// Byte-identical duplicates.
///
/// Files are bucketed by size first so the expensive hashing only runs where a
/// duplicate is actually possible.
pub fn find_exact_duplicates(files: &[MediaFile]) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<&MediaFile>> = HashMap::new();
    for file in files {
        by_size.entry(file.file_size).or_default().push(file);
    }

    let mut groups = Vec::new();

    for (size, candidates) in by_size {
        if candidates.len() < 2 || size == 0 {
            continue;
        }

        let mut by_hash: HashMap<String, Vec<&MediaFile>> = HashMap::new();
        for file in candidates {
            match hash_file(Path::new(&file.path)) {
                Ok(hash) => by_hash.entry(hash).or_default().push(file),
                Err(e) => log::warn!("Could not hash {}: {}", file.path, e),
            }
        }

        for (hash, matched) in by_hash {
            if matched.len() < 2 {
                continue;
            }
            let entries = matched
                .iter()
                .map(|f| to_duplicate_file(f, None, 1.0))
                .collect();
            groups.push(finish_group(
                format!("exact-{}", &hash[..12]),
                DuplicateMode::Exact,
                format!("{} identical files ({} bytes each)", matched.len(), size),
                entries,
            ));
        }
    }

    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    groups
}

/// Same recording across different encodings, matched on tags.
///
/// Grouping keys on the normalized title so the comparison stays linear; the
/// artist is then checked with Jaro-Winkler inside each bucket.
pub fn find_fuzzy_duplicates(
    files: &[MediaFile],
    metadata: &HashMap<String, TrackMetadata>,
    threshold: f64,
) -> Vec<DuplicateGroup> {
    let mut buckets: HashMap<String, Vec<&MediaFile>> = HashMap::new();

    for file in files {
        let Some(meta) = metadata.get(&file.path) else {
            continue;
        };
        let Some(title) = meta.title.as_ref().filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let key = normalize(title);
        if key.is_empty() {
            continue;
        }
        buckets.entry(key).or_default().push(file);
    }

    let mut groups = Vec::new();

    for (key, candidates) in buckets {
        if candidates.len() < 2 {
            continue;
        }

        // Within a title bucket, cluster by artist similarity. A bucket can
        // split into several clusters, so each one needs its own index to
        // keep group ids unique (they were colliding when two clusters in
        // the same bucket happened to end up the same size).
        let mut cluster_index = 0usize;
        let mut unassigned: Vec<&MediaFile> = candidates;
        while unassigned.len() > 1 {
            let reference = unassigned.remove(0);
            let ref_artist = metadata
                .get(&reference.path)
                .and_then(|m| m.artist.clone())
                .map(|a| normalize(&a))
                .unwrap_or_default();

            let mut cluster = vec![(reference, 1.0f64)];
            let mut leftover = Vec::new();

            for candidate in unassigned.drain(..) {
                let cand_artist = metadata
                    .get(&candidate.path)
                    .and_then(|m| m.artist.clone())
                    .map(|a| normalize(&a))
                    .unwrap_or_default();

                let score = if ref_artist.is_empty() && cand_artist.is_empty() {
                    // No artist on either side; the title match is all we have.
                    threshold
                } else {
                    strsim::jaro_winkler(&ref_artist, &cand_artist)
                };

                if score >= threshold {
                    cluster.push((candidate, score));
                } else {
                    leftover.push(candidate);
                }
            }

            unassigned = leftover;

            if cluster.len() < 2 {
                continue;
            }

            let entries = cluster
                .iter()
                .map(|(f, score)| to_duplicate_file(f, metadata.get(&f.path), *score))
                .collect();

            groups.push(finish_group(
                format!("fuzzy-{}-{}", key.replace(' ', "-"), cluster_index),
                DuplicateMode::Fuzzy,
                format!("{} files tagged as \"{}\"", cluster.len(), key),
                entries,
            ));
            cluster_index += 1;
        }
    }

    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    groups
}

/// Resolves a file's fingerprint, checking three layers in order: this
/// run's in-memory cache (keyed by path, so a file touched twice in one
/// scan is never rehashed), the on-disk cache (keyed by SHA-256 of the
/// whole file, so an unchanged file skips the expensive decode+fingerprint
/// step across runs), and finally a fresh decode.
fn fingerprint_cached(
    file: &MediaFile,
    session_cache: &mut HashMap<String, Vec<u32>>,
    disk_cache: &mut FingerprintCache,
    warnings: &mut Vec<String>,
) -> Option<Vec<u32>> {
    if let Some(fp) = session_cache.get(&file.path) {
        return Some(fp.clone());
    }

    let path = Path::new(&file.path);
    let hash = match hash_file(path) {
        Ok(h) => h,
        Err(e) => {
            let msg = format!("Could not hash {}: {}", file.path, e);
            log::warn!("{}", msg);
            warnings.push(msg);
            return None;
        }
    };

    if let Some(fp) = disk_cache.get(&hash) {
        session_cache.insert(file.path.clone(), fp.clone());
        return Some(fp.clone());
    }

    match fingerprint::fingerprint_file(path) {
        Ok(fp) => {
            disk_cache.insert(hash, fp.clone());
            session_cache.insert(file.path.clone(), fp.clone());
            Some(fp)
        }
        Err(e) => {
            let msg = format!("Could not fingerprint {}: {}", file.path, e);
            log::warn!("{}", msg);
            warnings.push(msg);
            None
        }
    }
}

/// Same recording by audio content, regardless of tags or encoding.
///
/// Files are bucketed by duration first (allowing a few seconds of slack for
/// different trimming/padding between rips) so fingerprinting - the
/// expensive step, since it decodes the file - only runs where a duplicate
/// is actually plausible. Within a bucket, fingerprints are compared
/// pairwise via Chromaprint-style matching.
pub fn find_acoustic_duplicates(
    files: &[MediaFile],
    threshold: f64,
    disk_cache: &mut FingerprintCache,
    mut on_progress: impl FnMut(usize, usize),
) -> (Vec<DuplicateGroup>, Vec<String>) {
    const DURATION_TOLERANCE_MS: i64 = 5_000;

    let mut warnings = Vec::new();

    let mut candidates: Vec<&MediaFile> = Vec::new();
    for file in files {
        if file.duration_ms.is_some() {
            candidates.push(file);
        } else {
            warnings.push(format!(
                "Skipped {} — no duration indexed, rescan the library to fix this",
                file.path
            ));
        }
    }
    candidates.sort_by_key(|f| f.duration_ms.unwrap_or(0));
    let total = candidates.len();
    on_progress(0, total);

    // Fingerprints are only computed once per file, lazily, since decoding
    // is the expensive part.
    let mut fingerprints: HashMap<String, Vec<u32>> = HashMap::new();

    let mut visited = vec![false; candidates.len()];
    let mut groups = Vec::new();

    for i in 0..candidates.len() {
        if visited[i] {
            continue;
        }
        let ref_file = candidates[i];
        let Some(ref_fp) = fingerprint_cached(ref_file, &mut fingerprints, disk_cache, &mut warnings) else {
            on_progress(fingerprints.len(), total);
            continue;
        };
        on_progress(fingerprints.len(), total);
        let ref_dur = ref_file.duration_ms.unwrap_or(0) as i64;

        let mut cluster = vec![(ref_file, 1.0f64)];

        for j in (i + 1)..candidates.len() {
            if visited[j] {
                continue;
            }
            let cand = candidates[j];
            let cand_dur = cand.duration_ms.unwrap_or(0) as i64;
            if cand_dur - ref_dur > DURATION_TOLERANCE_MS {
                // Sorted by duration, so nothing further out can match either.
                break;
            }

            let Some(cand_fp) = fingerprint_cached(cand, &mut fingerprints, disk_cache, &mut warnings) else {
                on_progress(fingerprints.len(), total);
                continue;
            };
            on_progress(fingerprints.len(), total);
            let score = fingerprint::similarity(&ref_fp, &cand_fp);
            if score >= threshold {
                cluster.push((cand, score));
                visited[j] = true;
            }
        }

        visited[i] = true;

        if cluster.len() < 2 {
            continue;
        }

        let entries = cluster
            .iter()
            .map(|(f, score)| to_duplicate_file(f, None, *score))
            .collect();

        groups.push(finish_group(
            format!("acoustic-{}-{}", ref_file.id, cluster.len()),
            DuplicateMode::Acoustic,
            format!("{} files sound like the same recording", cluster.len()),
            entries,
        ));
    }

    on_progress(total, total);
    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    (groups, warnings)
}

pub fn build_report(groups: Vec<DuplicateGroup>, warnings: Vec<String>) -> DuplicateReport {
    let total_files = groups.iter().map(|g| g.files.len()).sum();
    let reclaimable_bytes = groups.iter().map(|g| g.wasted_bytes).sum();

    DuplicateReport {
        total_groups: groups.len(),
        total_files,
        reclaimable_bytes,
        groups,
        warnings,
    }
}

/// Move the given files into a quarantine folder rather than unlinking them,
/// so a mistaken resolve can be undone from the file manager.
pub fn quarantine_files(paths: &[String], trash_dir: &Path) -> Result<Vec<ResolveOutcome>> {
    std::fs::create_dir_all(trash_dir)?;

    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let batch_dir = trash_dir.join(stamp);
    std::fs::create_dir_all(&batch_dir)?;

    let mut outcomes = Vec::new();

    for path in paths {
        let source = Path::new(path);
        let file_name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        // Two duplicates share a name by definition; disambiguate on collision.
        let mut target = batch_dir.join(&file_name);
        let mut counter = 1;
        while target.exists() {
            target = batch_dir.join(format!("{}-{}", counter, file_name));
            counter += 1;
        }

        match std::fs::rename(source, &target) {
            Ok(()) => outcomes.push(ResolveOutcome {
                path: path.clone(),
                success: true,
                moved_to: Some(target.to_string_lossy().to_string()),
                error: None,
            }),
            Err(e) => outcomes.push(ResolveOutcome {
                path: path.clone(),
                success: false,
                moved_to: None,
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_articles_punctuation_and_suffixes() {
        assert_eq!(normalize("The Beatles"), "beatles");
        assert_eq!(normalize("Let It Be (Remastered 2009)"), "let it be");
        assert_eq!(normalize("Song [Live]"), "song");
        assert_eq!(normalize("A.C.  -  D.C."), "a c d c");
    }

    #[test]
    fn normalize_keeps_inner_parentheses() {
        // Only a trailing qualifier should be dropped, not a mid-title one.
        assert_eq!(
            normalize("Everything (I Do) I Do It For You"),
            "everything (i do) i do it for you"
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { ' ' })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    fn media_file(path: &str, size: u64, bitrate: Option<u32>) -> MediaFile {
        MediaFile {
            id: path.to_string(),
            path: path.to_string(),
            file_name: path.rsplit('/').next().unwrap_or(path).to_string(),
            parent_dir: "/m".to_string(),
            media_type: crate::models::media_file::MediaType::Audio,
            audio_format: None,
            file_size: size,
            modified_at: 0,
            scanned_at: 0,
            has_cover_art: false,
            duration_ms: None,
            bitrate_kbps: bitrate,
            sample_rate_hz: None,
            channels: None,
            first_seen_at: 0,
            last_modified_by_app: None,
            is_new: false,
        }
    }

    fn track(title: &str, artist: &str) -> TrackMetadata {
        TrackMetadata {
            title: Some(title.to_string()),
            artist: Some(artist.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fuzzy_groups_same_song_across_encodings() {
        let files = vec![
            media_file("/m/a.mp3", 3_000_000, Some(192)),
            media_file("/m/b.flac", 30_000_000, Some(1000)),
        ];
        let mut metadata = HashMap::new();
        metadata.insert("/m/a.mp3".to_string(), track("Let It Be (Remastered)", "The Beatles"));
        metadata.insert("/m/b.flac".to_string(), track("Let It Be", "Beatles"));

        let groups = find_fuzzy_duplicates(&files, &metadata, 0.85);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        // The FLAC has the higher bitrate, so it is the one to keep.
        let kept = groups[0].files.iter().find(|f| f.recommended_keep).unwrap();
        assert_eq!(kept.path, "/m/b.flac");
        assert_eq!(groups[0].wasted_bytes, 3_000_000);
    }

    #[test]
    fn fuzzy_keeps_different_artists_apart() {
        let files = vec![
            media_file("/m/a.mp3", 1000, Some(192)),
            media_file("/m/b.mp3", 1000, Some(192)),
        ];
        let mut metadata = HashMap::new();
        metadata.insert("/m/a.mp3".to_string(), track("Hurt", "Nine Inch Nails"));
        metadata.insert("/m/b.mp3".to_string(), track("Hurt", "Johnny Cash"));

        assert!(find_fuzzy_duplicates(&files, &metadata, 0.85).is_empty());
    }

    #[test]
    fn fuzzy_ignores_files_without_a_title() {
        let files = vec![
            media_file("/m/a.mp3", 1000, None),
            media_file("/m/b.mp3", 1000, None),
        ];
        let metadata = HashMap::new();

        assert!(find_fuzzy_duplicates(&files, &metadata, 0.85).is_empty());
    }
}
