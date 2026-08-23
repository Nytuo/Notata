use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, ItemValue, Tag, TagItem};

use crate::error::Result;

use super::Chapter;

fn format_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn parse_timestamp(s: &str) -> Option<u64> {
    let s = s.trim();

    if let Ok(total_seconds) = s.parse::<f64>() {
        if total_seconds.is_finite() && total_seconds >= 0.0 {
            return Some((total_seconds * 1000.0).round() as u64);
        }
    }

    let (hms, millis) = match s.split_once('.') {
        Some((hms, millis)) => (hms, millis.parse().ok()?),
        None => (s, 0u64),
    };

    let parts: Vec<&str> = hms.split(':').collect();
    let (hours, minutes, seconds): (u64, u64, u64) = match parts.as_slice() {
        [h, m, s] => (h.parse().ok()?, m.parse().ok()?, s.parse().ok()?),
        [m, s] => (0, m.parse().ok()?, s.parse().ok()?),
        _ => return None,
    };
    Some(hours * 3_600_000 + minutes * 60_000 + seconds * 1000 + millis)
}

fn chapter_key(index: usize) -> String {
    format!("CHAPTER{index:03}")
}

fn chapter_name_key(index: usize) -> String {
    format!("CHAPTER{index:03}NAME")
}

fn parse_chapter_key(raw_key: &str) -> Option<(u64, bool)> {
    let upper = raw_key.to_ascii_uppercase();
    let rest = upper.strip_prefix("CHAPTER")?;
    let (digits, is_name) = match rest.strip_suffix("NAME") {
        Some(d) => (d, true),
        None => (rest, false),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: u64 = digits.parse().ok()?;
    Some((index, is_name))
}

pub fn read_chapters(path: &str) -> Result<Vec<Chapter>> {
    let tagged_file = Probe::open(path)?.read()?;
    let Some(tag) = tagged_file.primary_tag() else {
        return Ok(Vec::new());
    };

    let mut starts: std::collections::BTreeMap<u64, u64> = std::collections::BTreeMap::new();
    let mut names: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();

    for item in tag.items() {
        let ItemKey::Unknown(raw_key) = item.key() else {
            continue;
        };
        let Some((index, is_name)) = parse_chapter_key(raw_key) else {
            continue;
        };
        let Some(text) = item.value().text() else {
            continue;
        };
        if is_name {
            names.insert(index, text.to_string());
        } else if let Some(start_ms) = parse_timestamp(text) {
            starts.insert(index, start_ms);
        }
    }

    let mut chapters: Vec<Chapter> = starts
        .into_iter()
        .map(|(index, start_ms)| Chapter {
            id: format!("chp{index}"),
            title: names.get(&index).cloned().unwrap_or_default(),
            start_ms,
            end_ms: 0,
        })
        .collect();
    chapters.sort_by_key(|c| c.start_ms);

    for i in 0..chapters.len().saturating_sub(1) {
        chapters[i].end_ms = chapters[i + 1].start_ms;
    }

    Ok(chapters)
}

pub fn write_chapters(path: &str, chapters: &[Chapter]) -> Result<()> {
    let mut tagged_file = Probe::open(path)?.read()?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(t) => t,
        None => {
            let tag_type = tagged_file.primary_tag_type();
            tagged_file.insert_tag(Tag::new(tag_type));
            tagged_file.primary_tag_mut().unwrap()
        }
    };

    let stale_keys: Vec<ItemKey> = tag
        .items()
        .filter_map(|item| match item.key() {
            ItemKey::Unknown(k) if k.to_ascii_uppercase().starts_with("CHAPTER") => {
                Some(item.key().clone())
            }
            _ => None,
        })
        .collect();
    for key in stale_keys {
        tag.remove_key(&key);
    }

    let mut sorted: Vec<&Chapter> = chapters.iter().collect();
    sorted.sort_by_key(|c| c.start_ms);

    for (i, chapter) in sorted.iter().enumerate() {
        let index = i + 1;
        tag.push_unchecked(TagItem::new(
            ItemKey::Unknown(chapter_key(index)),
            ItemValue::Text(format_timestamp(chapter.start_ms)),
        ));
        tag.push_unchecked(TagItem::new(
            ItemKey::Unknown(chapter_name_key(index)),
            ItemValue::Text(chapter.title.clone()),
        ));
    }

    tag.save_to_path(path, lofty::config::WriteOptions::default())?;
    Ok(())
}
