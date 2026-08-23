use crate::error::{NotataError, Result};

use super::Chapter;

const UNITS_PER_MS: u64 = 10_000;

struct BoxInfo {
    fourcc: [u8; 4],
    start: usize,
    header_len: usize,
    total_len: usize,
}

fn read_box_header(data: &[u8], offset: usize) -> Option<BoxInfo> {
    if offset + 8 > data.len() {
        return None;
    }
    let size32 = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    let mut fourcc = [0u8; 4];
    fourcc.copy_from_slice(&data[offset + 4..offset + 8]);

    let (header_len, total_len) = if size32 == 1 {
        if offset + 16 > data.len() {
            return None;
        }
        let large = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().unwrap());
        (16, large as usize)
    } else if size32 == 0 {
        (8, data.len() - offset)
    } else {
        (8, size32)
    };

    if total_len < header_len || offset + total_len > data.len() {
        return None;
    }

    Some(BoxInfo {
        fourcc,
        start: offset,
        header_len,
        total_len,
    })
}

fn iter_boxes(data: &[u8]) -> Vec<BoxInfo> {
    let mut boxes = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        match read_box_header(data, offset) {
            Some(b) => {
                let next = b.start + b.total_len;
                boxes.push(b);
                offset = next;
            }
            None => break,
        }
    }
    boxes
}

fn find_box<'a>(boxes: &'a [BoxInfo], fourcc: &[u8; 4]) -> Option<&'a BoxInfo> {
    boxes.iter().find(|b| &b.fourcc == fourcc)
}

fn build_box(fourcc: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(fourcc);
    out.extend_from_slice(body);
    out
}

fn parse_chpl(body: &[u8]) -> Vec<Chapter> {
    if body.len() < 9 {
        return Vec::new();
    }
    let count = body[8] as usize;
    let mut offset = 9;
    let mut chapters = Vec::with_capacity(count);

    for i in 0..count {
        if offset + 9 > body.len() {
            break;
        }
        let start_units = u64::from_be_bytes(body[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let title_len = body[offset] as usize;
        offset += 1;
        if offset + title_len > body.len() {
            break;
        }
        let title = String::from_utf8_lossy(&body[offset..offset + title_len]).into_owned();
        offset += title_len;

        chapters.push(Chapter {
            id: format!("chp{}", i + 1),
            title,
            start_ms: start_units / UNITS_PER_MS,
            end_ms: 0,
        });
    }

    chapters
}

fn build_chpl(chapters: &[Chapter]) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(1u8);
    body.extend_from_slice(&[0u8; 3]);
    body.extend_from_slice(&[0u8; 4]);
    body.push(chapters.len().min(255) as u8);

    for chapter in chapters.iter().take(255) {
        let start_units = chapter.start_ms * UNITS_PER_MS;
        body.extend_from_slice(&start_units.to_be_bytes());

        let mut title_bytes = chapter.title.as_bytes();
        if title_bytes.len() > 255 {
            let mut cut = 255;
            while cut > 0 && !chapter.title.is_char_boundary(cut) {
                cut -= 1;
            }
            title_bytes = &title_bytes[..cut];
        }
        body.push(title_bytes.len() as u8);
        body.extend_from_slice(title_bytes);
    }

    build_box(b"chpl", &body)
}

fn patch_chunk_offsets(data: &mut [u8], delta: i64) {
    const CONTAINERS: &[&[u8; 4]] = &[b"trak", b"mdia", b"minf", b"stbl", b"dinf", b"edts", b"udta"];

    let boxes = iter_boxes(data);
    for b in boxes {
        let body_start = b.start + b.header_len;
        let body_end = b.start + b.total_len;

        if &b.fourcc == b"stco" && body_end - body_start >= 8 {
            let count =
                u32::from_be_bytes(data[body_start + 4..body_start + 8].try_into().unwrap())
                    as usize;
            let mut offset = body_start + 8;
            for _ in 0..count {
                if offset + 4 > body_end {
                    break;
                }
                let value = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
                let patched = (value as i64 + delta).max(0) as u32;
                data[offset..offset + 4].copy_from_slice(&patched.to_be_bytes());
                offset += 4;
            }
        } else if &b.fourcc == b"co64" && body_end - body_start >= 8 {
            let count =
                u32::from_be_bytes(data[body_start + 4..body_start + 8].try_into().unwrap())
                    as usize;
            let mut offset = body_start + 8;
            for _ in 0..count {
                if offset + 8 > body_end {
                    break;
                }
                let value = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
                let patched = (value as i64 + delta).max(0) as u64;
                data[offset..offset + 8].copy_from_slice(&patched.to_be_bytes());
                offset += 8;
            }
        } else if CONTAINERS.contains(&&b.fourcc) {
            patch_chunk_offsets(&mut data[body_start..body_end], delta);
        }
    }
}

pub fn read_chapters(path: &str) -> Result<Vec<Chapter>> {
    let data = std::fs::read(path)?;
    let top = iter_boxes(&data);
    let Some(moov) = find_box(&top, b"moov") else {
        return Ok(Vec::new());
    };
    let moov_body = &data[moov.start + moov.header_len..moov.start + moov.total_len];
    let moov_children = iter_boxes(moov_body);

    if let Some(udta) = find_box(&moov_children, b"udta") {
        let udta_body = &moov_body[udta.start + udta.header_len..udta.start + udta.total_len];
        let udta_children = iter_boxes(udta_body);
        if let Some(chpl) = find_box(&udta_children, b"chpl") {
            let chpl_body = &udta_body[chpl.start + chpl.header_len..chpl.start + chpl.total_len];
            let mut chapters = parse_chpl(chpl_body);
            chapters.sort_by_key(|c| c.start_ms);
            for i in 0..chapters.len().saturating_sub(1) {
                chapters[i].end_ms = chapters[i + 1].start_ms;
            }
            if !chapters.is_empty() {
                return Ok(chapters);
            }
        }
    }

    Ok(qt::read_chapters(&data, moov_body, &moov_children).unwrap_or_default())
}

pub fn write_chapters(path: &str, chapters: &[Chapter]) -> Result<()> {
    let data = std::fs::read(path)?;
    let top = iter_boxes(&data);
    let moov = find_box(&top, b"moov")
        .ok_or_else(|| NotataError::Custom("File has no moov atom".to_string()))?;
    let moov_body = &data[moov.start + moov.header_len..moov.start + moov.total_len];
    let moov_children = iter_boxes(moov_body);

    let mut sorted: Vec<&Chapter> = chapters.iter().collect();
    sorted.sort_by_key(|c| c.start_ms);
    let sorted_owned: Vec<Chapter> = sorted.into_iter().cloned().collect();
    let new_chpl = build_chpl(&sorted_owned);

    let new_udta_body = match find_box(&moov_children, b"udta") {
        Some(udta) => {
            let udta_body = &moov_body[udta.start + udta.header_len..udta.start + udta.total_len];
            let udta_children = iter_boxes(udta_body);
            let mut body = Vec::new();
            for child in &udta_children {
                if &child.fourcc != b"chpl" {
                    body.extend_from_slice(
                        &udta_body[child.start..child.start + child.total_len],
                    );
                }
            }
            body.extend_from_slice(&new_chpl);
            body
        }
        None => new_chpl,
    };
    let new_udta = build_box(b"udta", &new_udta_body);

    let mut new_moov_body = Vec::new();
    for child in &moov_children {
        if &child.fourcc != b"udta" {
            new_moov_body.extend_from_slice(
                &moov_body[child.start..child.start + child.total_len],
            );
        }
    }
    new_moov_body.extend_from_slice(&new_udta);

    let delta = new_moov_body.len() as i64 + 8 - moov.total_len as i64;

    let mdat_starts_after_moov = top
        .iter()
        .filter(|b| &b.fourcc == b"mdat")
        .any(|b| b.start > moov.start);
    if delta != 0 && mdat_starts_after_moov {
        patch_chunk_offsets(&mut new_moov_body, delta);
    }

    let new_moov = build_box(b"moov", &new_moov_body);

    let mut out = Vec::with_capacity(data.len() + delta.unsigned_abs() as usize);
    out.extend_from_slice(&data[..moov.start]);
    out.extend_from_slice(&new_moov);
    out.extend_from_slice(&data[moov.start + moov.total_len..]);

    let tmp_path = format!("{path}.chaptmp");
    std::fs::write(&tmp_path, &out)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

mod qt {
    use super::{find_box, iter_boxes, BoxInfo};
    use crate::chapters::Chapter;

    fn be_u32(data: &[u8], offset: usize) -> Option<u32> {
        data.get(offset..offset + 4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
    }

    fn be_u64(data: &[u8], offset: usize) -> Option<u64> {
        data.get(offset..offset + 8)
            .map(|b| u64::from_be_bytes(b.try_into().unwrap()))
    }

    fn body_of<'a>(container: &'a [u8], b: &BoxInfo) -> &'a [u8] {
        &container[b.start + b.header_len..b.start + b.total_len]
    }

    fn track_id_of(trak_body: &[u8]) -> Option<u32> {
        let children = iter_boxes(trak_body);
        let tkhd = find_box(&children, b"tkhd")?;
        let body = body_of(trak_body, tkhd);
        let version = *body.first()?;
        let offset = if version == 1 { 20 } else { 12 };
        be_u32(body, offset)
    }

    fn chapter_track_ref(trak_body: &[u8]) -> Option<u32> {
        let children = iter_boxes(trak_body);
        let tref = find_box(&children, b"tref")?;
        let tref_body = body_of(trak_body, tref);
        let tref_children = iter_boxes(tref_body);
        let chap = find_box(&tref_children, b"chap")?;
        let chap_body = body_of(tref_body, chap);
        be_u32(chap_body, 0)
    }

    fn mdia_timescale(mdia_body: &[u8]) -> Option<u32> {
        let children = iter_boxes(mdia_body);
        let mdhd = find_box(&children, b"mdhd")?;
        let body = body_of(mdia_body, mdhd);
        let version = *body.first()?;
        let offset = if version == 1 { 20 } else { 12 };
        be_u32(body, offset)
    }

    fn stts_starts(stts_body: &[u8]) -> Vec<u64> {
        let mut starts = Vec::new();
        let Some(entry_count) = be_u32(stts_body, 4) else {
            return starts;
        };
        let mut offset = 8usize;
        let mut time = 0u64;
        for _ in 0..entry_count {
            let Some(count) = be_u32(stts_body, offset) else { break };
            let Some(delta) = be_u32(stts_body, offset + 4) else { break };
            offset += 8;
            for _ in 0..count {
                starts.push(time);
                time += delta as u64;
            }
        }
        starts
    }

    fn stsz_sizes(stsz_body: &[u8]) -> Vec<u32> {
        let Some(sample_size) = be_u32(stsz_body, 4) else {
            return Vec::new();
        };
        let Some(sample_count) = be_u32(stsz_body, 8) else {
            return Vec::new();
        };
        if sample_size != 0 {
            return vec![sample_size; sample_count as usize];
        }
        let mut sizes = Vec::with_capacity(sample_count as usize);
        for i in 0..sample_count {
            match be_u32(stsz_body, 12 + (i as usize) * 4) {
                Some(size) => sizes.push(size),
                None => break,
            }
        }
        sizes
    }

    fn stsc_entries(stsc_body: &[u8]) -> Vec<(u32, u32)> {
        let Some(entry_count) = be_u32(stsc_body, 4) else {
            return Vec::new();
        };
        let mut entries = Vec::with_capacity(entry_count as usize);
        for i in 0..entry_count {
            let base = 8 + (i as usize) * 12;
            let (Some(first_chunk), Some(samples_per_chunk)) =
                (be_u32(stsc_body, base), be_u32(stsc_body, base + 4))
            else {
                break;
            };
            entries.push((first_chunk, samples_per_chunk));
        }
        entries
    }

    fn chunk_offsets(stbl_children: &[BoxInfo], stbl_body: &[u8]) -> Vec<u64> {
        if let Some(stco) = find_box(stbl_children, b"stco") {
            let body = body_of(stbl_body, stco);
            let Some(count) = be_u32(body, 4) else { return Vec::new() };
            (0..count)
                .filter_map(|i| be_u32(body, 8 + (i as usize) * 4).map(u64::from))
                .collect()
        } else if let Some(co64) = find_box(stbl_children, b"co64") {
            let body = body_of(stbl_body, co64);
            let Some(count) = be_u32(body, 4) else { return Vec::new() };
            (0..count)
                .filter_map(|i| be_u64(body, 8 + (i as usize) * 8))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn sample_offsets(stsc: &[(u32, u32)], chunk_offsets: &[u64], sizes: &[u32]) -> Vec<u64> {
        let mut offsets = Vec::with_capacity(sizes.len());
        let mut sample_index = 0usize;
        for (entry_idx, &(first_chunk, samples_per_chunk)) in stsc.iter().enumerate() {
            let next_first_chunk = stsc
                .get(entry_idx + 1)
                .map(|(fc, _)| *fc)
                .unwrap_or(chunk_offsets.len() as u32 + 1);
            for chunk_num in first_chunk..next_first_chunk {
                let Some(&chunk_offset) = chunk_offsets.get((chunk_num - 1) as usize) else {
                    return offsets;
                };
                let mut running = chunk_offset;
                for _ in 0..samples_per_chunk {
                    if sample_index >= sizes.len() {
                        return offsets;
                    }
                    offsets.push(running);
                    running += sizes[sample_index] as u64;
                    sample_index += 1;
                }
            }
        }
        offsets
    }

    fn read_sample_text(data: &[u8], offset: u64, size: u32) -> String {
        let start = offset as usize;
        let end = start + size as usize;
        let Some(sample) = data.get(start..end) else {
            return String::new();
        };
        if sample.len() < 2 {
            return String::new();
        }
        let text_len = u16::from_be_bytes([sample[0], sample[1]]) as usize;
        let text_bytes = sample.get(2..2 + text_len).unwrap_or(&[]);
        String::from_utf8_lossy(text_bytes).into_owned()
    }

    pub fn read_chapters(
        data: &[u8],
        moov_body: &[u8],
        moov_children: &[BoxInfo],
    ) -> Option<Vec<Chapter>> {
        let trak_boxes: Vec<&BoxInfo> = moov_children.iter().filter(|b| &b.fourcc == b"trak").collect();

        let chapter_track_id = trak_boxes
            .iter()
            .find_map(|trak| chapter_track_ref(body_of(moov_body, trak)))?;

        let chapter_trak = trak_boxes
            .iter()
            .find(|trak| track_id_of(body_of(moov_body, trak)) == Some(chapter_track_id))?;
        let trak_body = body_of(moov_body, chapter_trak);

        let trak_children = iter_boxes(trak_body);
        let mdia = find_box(&trak_children, b"mdia")?;
        let mdia_body = body_of(trak_body, mdia);
        let timescale = mdia_timescale(mdia_body)?;
        if timescale == 0 {
            return None;
        }

        let mdia_children = iter_boxes(mdia_body);
        let minf = find_box(&mdia_children, b"minf")?;
        let minf_body = body_of(mdia_body, minf);
        let minf_children = iter_boxes(minf_body);
        let stbl = find_box(&minf_children, b"stbl")?;
        let stbl_body = body_of(minf_body, stbl);
        let stbl_children = iter_boxes(stbl_body);

        let stts = find_box(&stbl_children, b"stts")?;
        let starts = stts_starts(body_of(stbl_body, stts));

        let stsz = find_box(&stbl_children, b"stsz")?;
        let sizes = stsz_sizes(body_of(stbl_body, stsz));

        let stsc = find_box(&stbl_children, b"stsc")?;
        let stsc_entries = stsc_entries(body_of(stbl_body, stsc));

        let offsets_of_chunks = chunk_offsets(&stbl_children, stbl_body);
        let sample_offsets = sample_offsets(&stsc_entries, &offsets_of_chunks, &sizes);

        let count = starts.len().min(sizes.len()).min(sample_offsets.len());
        if count == 0 {
            return None;
        }

        let mut chapters: Vec<Chapter> = (0..count)
            .map(|i| {
                let title = read_sample_text(data, sample_offsets[i], sizes[i]);
                Chapter {
                    id: format!("chp{}", i + 1),
                    title,
                    start_ms: starts[i] * 1000 / timescale as u64,
                    end_ms: 0,
                }
            })
            .collect();

        chapters.sort_by_key(|c| c.start_ms);
        for i in 0..chapters.len().saturating_sub(1) {
            chapters[i].end_ms = chapters[i + 1].start_ms;
        }

        Some(chapters)
    }
}
