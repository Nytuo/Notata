use serde::Deserialize;
use tokio::process::Command;

use super::Chapter;

#[derive(Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    chapters: Vec<FfprobeChapter>,
}

#[derive(Deserialize)]
struct FfprobeChapter {
    start_time: String,
    end_time: String,
    #[serde(default)]
    tags: FfprobeChapterTags,
}

#[derive(Deserialize, Default)]
struct FfprobeChapterTags {
    title: Option<String>,
}

fn seconds_to_ms(s: &str) -> Option<u64> {
    s.trim().parse::<f64>().ok().map(|secs| (secs * 1000.0).round() as u64)
}

pub async fn read_chapters(ffprobe_path: &str, path: &str) -> Option<Vec<Chapter>> {
    let output = Command::new(ffprobe_path)
        .args(["-v", "error", "-show_chapters", "-of", "json", path])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;

    let mut chapters: Vec<Chapter> = parsed
        .chapters
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            Some(Chapter {
                id: format!("chp{}", i + 1),
                title: c.tags.title.clone().unwrap_or_default(),
                start_ms: seconds_to_ms(&c.start_time)?,
                end_ms: seconds_to_ms(&c.end_time)?,
            })
        })
        .collect();

    if chapters.is_empty() {
        return None;
    }

    chapters.sort_by_key(|c| c.start_ms);
    Some(chapters)
}
