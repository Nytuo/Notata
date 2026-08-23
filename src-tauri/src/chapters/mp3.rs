use id3::frame::{Chapter as Id3Chapter, TableOfContents};
use id3::{Frame, Tag, TagLike, Version};

use crate::error::Result;

use super::Chapter;

pub fn read_chapters(path: &str) -> Result<Vec<Chapter>> {
    let tag = match id3::no_tag_ok(Tag::read_from_path(path))? {
        Some(tag) => tag,
        None => return Ok(Vec::new()),
    };

    let mut chapters: Vec<Chapter> = tag
        .chapters()
        .map(|c| Chapter {
            id: c.element_id.clone(),
            title: c
                .frames
                .iter()
                .find(|f| f.id() == "TIT2")
                .and_then(|f| f.content().text())
                .unwrap_or("")
                .to_string(),
            start_ms: c.start_time as u64,
            end_ms: c.end_time as u64,
        })
        .collect();

    chapters.sort_by_key(|c| c.start_ms);
    Ok(chapters)
}

pub fn write_chapters(path: &str, chapters: &[Chapter]) -> Result<()> {
    let mut tag = id3::no_tag_ok(Tag::read_from_path(path))?.unwrap_or_default();

    tag.remove_all_chapters();
    tag.remove_all_tables_of_contents();

    let mut sorted: Vec<&Chapter> = chapters.iter().collect();
    sorted.sort_by_key(|c| c.start_ms);

    let element_ids: Vec<String> = sorted
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if c.id.is_empty() {
                format!("chp{i}")
            } else {
                c.id.clone()
            }
        })
        .collect();

    for (chapter, element_id) in sorted.iter().zip(element_ids.iter()) {
        tag.add_frame(Id3Chapter {
            element_id: element_id.clone(),
            start_time: chapter.start_ms as u32,
            end_time: chapter.end_ms as u32,
            start_offset: 0xffffffff,
            end_offset: 0xffffffff,
            frames: vec![Frame::text("TIT2", chapter.title.clone())],
        });
    }

    if !element_ids.is_empty() {
        tag.add_frame(TableOfContents {
            element_id: "toc".to_string(),
            top_level: true,
            ordered: true,
            elements: element_ids,
            frames: Vec::new(),
        });
    }

    tag.write_to_path(path, Version::Id3v24)?;
    Ok(())
}
