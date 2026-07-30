pub mod fields;
pub mod presets;
pub mod template;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{NotataError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlanEntry {
    pub source_path: String,
    pub target_path: String,
    /// Path relative to the base directory, as produced by the template.
    pub relative_target: String,
    /// False when the name is already correct — nothing to do.
    pub changed: bool,
    /// Set when this entry cannot be applied safely.
    pub conflict: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlan {
    pub entries: Vec<RenamePlanEntry>,
    pub total: usize,
    pub changed: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameOutcome {
    pub source_path: String,
    pub target_path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Compute the new path for one file without touching disk.
fn plan_entry(
    tokens: &[template::Token],
    source_path: &str,
    fields: &HashMap<String, String>,
    base_dir: &Path,
) -> RenamePlanEntry {
    let source = Path::new(source_path);
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let outcome = template::render_detailed(tokens, fields);
    let rendered = outcome.text.clone();

    let relative_target = match &extension {
        Some(ext) => format!("{}.{}", rendered, ext),
        None => rendered.clone(),
    };

    let target = base_dir.join(&relative_target);
    let target_path = target.to_string_lossy().to_string();

    // Guard both shapes of failure: nothing rendered at all, and the case
    // where only the template's literal punctuation survived because no
    // field resolved — that is what produced names like "() ()".
    let conflict = if rendered.is_empty() {
        Some("Template produced an empty name — this file has no metadata to rename from".to_string())
    } else if outcome.is_unresolved() {
        Some(format!(
            "No metadata found for this file — the template resolved to \"{}\"",
            rendered
        ))
    } else {
        None
    };

    RenamePlanEntry {
        changed: target_path != source_path,
        source_path: source_path.to_string(),
        target_path,
        relative_target,
        conflict,
    }
}

/// Build a full rename plan, flagging collisions before anything is moved.
///
/// `base_dir` is the directory the rendered relative paths hang off. When
/// `None`, each file's own parent directory is used.
pub fn build_plan(
    template_str: &str,
    files: &[(String, HashMap<String, String>)],
    base_dir: Option<&str>,
) -> Result<RenamePlan> {
    let tokens = template::parse(template_str).map_err(NotataError::Custom)?;

    let mut entries = Vec::new();
    for (path, field_values) in files {
        let source = Path::new(path);
        let base = match base_dir {
            Some(b) => PathBuf::from(b),
            None => source.parent().map(PathBuf::from).unwrap_or_default(),
        };
        entries.push(plan_entry(&tokens, path, field_values, &base));
    }

    flag_conflicts(&mut entries);

    let changed = entries.iter().filter(|e| e.changed).count();
    let conflicts = entries.iter().filter(|e| e.conflict.is_some()).count();

    Ok(RenamePlan {
        total: entries.len(),
        changed,
        conflicts,
        entries,
    })
}

/// Build a plan straight from paths, reading each file's metadata with the
/// reader that matches its type.
pub fn build_plan_for_paths(
    template_str: &str,
    paths: &[String],
    base_dir: Option<&str>,
) -> Result<RenamePlan> {
    let files: Vec<(String, HashMap<String, String>)> = paths
        .iter()
        .map(|p| (p.clone(), fields::fields_for_path(p)))
        .collect();

    build_plan(template_str, &files, base_dir)
}

/// Mark entries whose targets collide with each other or with unrelated files
/// already on disk.
fn flag_conflicts(entries: &mut [RenamePlanEntry]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let sources: HashSet<String> = entries.iter().map(|e| e.source_path.clone()).collect();

    for i in 0..entries.len() {
        if entries[i].conflict.is_some() || !entries[i].changed {
            continue;
        }

        let target = entries[i].target_path.clone();

        if let Some(&first) = seen.get(&target) {
            entries[i].conflict = Some(format!(
                "Target collides with {}",
                entries[first].source_path
            ));
            continue;
        }
        seen.insert(target.clone(), i);

        // A target that already exists is only safe if it belongs to another
        // file in this same plan, which will itself be moved out of the way.
        if Path::new(&target).exists() && !sources.contains(&target) {
            entries[i].conflict = Some("A file already exists at the target path".to_string());
        }
    }
}

/// Execute a plan. Entries that are unchanged or flagged are skipped.
pub fn apply_plan(entries: &[RenamePlanEntry]) -> Vec<RenameOutcome> {
    let mut outcomes = Vec::new();

    for entry in entries {
        if !entry.changed || entry.conflict.is_some() {
            continue;
        }

        let target = Path::new(&entry.target_path);

        if let Some(parent) = target.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                outcomes.push(RenameOutcome {
                    source_path: entry.source_path.clone(),
                    target_path: entry.target_path.clone(),
                    success: false,
                    error: Some(format!("Could not create directory: {}", e)),
                });
                continue;
            }
        }

        // Re-check at apply time: the plan may have been built minutes ago.
        if target.exists() {
            outcomes.push(RenameOutcome {
                source_path: entry.source_path.clone(),
                target_path: entry.target_path.clone(),
                success: false,
                error: Some("A file already exists at the target path".to_string()),
            });
            continue;
        }

        match std::fs::rename(&entry.source_path, target) {
            Ok(()) => outcomes.push(RenameOutcome {
                source_path: entry.source_path.clone(),
                target_path: entry.target_path.clone(),
                success: true,
                error: None,
            }),
            Err(e) => outcomes.push(RenameOutcome {
                source_path: entry.source_path.clone(),
                target_path: entry.target_path.clone(),
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn track(title: &str, artist: &str, album: &str, n: u32) -> HashMap<String, String> {
        fields(&[
            ("title", title),
            ("artist", artist),
            ("albumartist", artist),
            ("album", album),
            ("track", &n.to_string()),
            ("year", "1994"),
        ])
    }

    #[test]
    fn builds_paths_from_template() {
        let files = vec![(
            "/music/01.flac".to_string(),
            track("Bakesale", "Sebadoh", "Bakesale", 3),
        )];
        let plan = build_plan(
            "{albumartist}/{album}[ ({year})]/{track:02} - {title}",
            &files,
            Some("/library"),
        )
        .unwrap();

        assert_eq!(
            plan.entries[0].relative_target,
            "Sebadoh/Bakesale (1994)/03 - Bakesale.flac"
        );
        assert!(plan.entries[0].changed);
        assert_eq!(plan.conflicts, 0);
    }

    #[test]
    fn builds_paths_for_a_movie() {
        let files = vec![(
            "/movies/blade.mkv".to_string(),
            fields(&[("title", "Blade Runner"), ("year", "1982")]),
        )];
        let plan = build_plan(
            "{title}[ ({year})]/{title}[ ({year})]",
            &files,
            Some("/library"),
        )
        .unwrap();

        assert_eq!(
            plan.entries[0].relative_target,
            "Blade Runner (1982)/Blade Runner (1982).mkv"
        );
        assert!(plan.entries[0].conflict.is_none());
    }

    #[test]
    fn flags_a_file_whose_template_resolved_to_punctuation_only() {
        // The reported failure: a video with no readable metadata rendered
        // "() ()" from the literals and was offered as a valid rename.
        let files = vec![("/movies/unknown.mkv".to_string(), HashMap::new())];
        let plan = build_plan(
            "{title} ({year})/{title} ({year})",
            &files,
            Some("/library"),
        )
        .unwrap();

        assert_eq!(plan.conflicts, 1);
        let conflict = plan.entries[0].conflict.as_ref().unwrap();
        assert!(
            conflict.contains("No metadata"),
            "unexpected message: {}",
            conflict
        );
    }

    #[test]
    fn optional_groups_keep_partial_metadata_usable() {
        // A known title but no year should still rename cleanly.
        let files = vec![(
            "/movies/x.mkv".to_string(),
            fields(&[("title", "Solaris")]),
        )];
        let plan = build_plan("{title}[ ({year})]", &files, Some("/library")).unwrap();

        assert_eq!(plan.entries[0].relative_target, "Solaris.mkv");
        assert!(plan.entries[0].conflict.is_none());
    }

    #[test]
    fn flags_two_files_targeting_the_same_path() {
        let files = vec![
            ("/music/a.flac".to_string(), track("Same", "A", "X", 1)),
            ("/music/b.flac".to_string(), track("Same", "A", "X", 1)),
        ];
        let plan = build_plan("{artist}/{title}", &files, Some("/library")).unwrap();

        assert!(plan.entries[0].conflict.is_none());
        assert!(plan.entries[1].conflict.is_some());
        assert_eq!(plan.conflicts, 1);
    }

    #[test]
    fn flags_templates_that_resolve_to_nothing() {
        let files = vec![("/music/a.flac".to_string(), HashMap::new())];
        let plan = build_plan("{artist}/{title}", &files, Some("/library")).unwrap();

        assert!(plan.entries[0].conflict.is_some());
    }

    #[test]
    fn unchanged_names_are_not_counted_as_changes() {
        let files = vec![(
            "/library/A/Song.flac".to_string(),
            track("Song", "A", "X", 1),
        )];
        let plan = build_plan("{artist}/{title}", &files, Some("/library")).unwrap();

        assert!(!plan.entries[0].changed);
        assert_eq!(plan.changed, 0);
    }

    #[test]
    fn rejects_invalid_templates() {
        let files = vec![("/music/a.flac".to_string(), HashMap::new())];
        assert!(build_plan("{unclosed", &files, None).is_err());
    }
}
