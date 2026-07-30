use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, ItemValue, TagItem};

use crate::error::Result;
use crate::models::track::TrackMetadata;

pub fn write_metadata(path: &str, metadata: &TrackMetadata) -> Result<()> {
    let mut tagged_file = Probe::open(path)?.read()?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(t) => t,
        None => {
            let tag_type = tagged_file.primary_tag_type();
            tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
            tagged_file.primary_tag_mut().unwrap()
        }
    };

    set_or_remove_str(tag, &ItemKey::TrackTitle, metadata.title.as_deref());
    set_or_remove_str(tag, &ItemKey::TrackArtist, metadata.artist.as_deref());
    set_or_remove_str(tag, &ItemKey::AlbumArtist, metadata.album_artist.as_deref());
    set_or_remove_str(tag, &ItemKey::AlbumTitle, metadata.album.as_deref());

    if let Some(n) = metadata.track_number {
        tag.set_track(n);
    } else {
        tag.remove_track();
    }
    if let Some(n) = metadata.total_tracks {
        tag.set_track_total(n);
    } else {
        tag.remove_track_total();
    }
    if let Some(n) = metadata.disc_number {
        tag.set_disk(n);
    } else {
        tag.remove_disk();
    }
    if let Some(n) = metadata.total_discs {
        tag.set_disk_total(n);
    } else {
        tag.remove_disk_total();
    }

    if let Some(y) = metadata.year {
        tag.set_year(y as u32);
    } else {
        tag.remove_year();
    }

    set_or_remove_str(tag, &ItemKey::RecordingDate, metadata.date.as_deref());

    if let Some(genres) = &metadata.genre {
        let genre_str = genres.join("; ");
        set_or_remove_str(tag, &ItemKey::Genre, Some(&genre_str));
    } else {
        tag.remove_key(&ItemKey::Genre);
    }

    if let Some(composers) = &metadata.composer {
        tag.remove_key(&ItemKey::Composer);
        for c in composers {
            tag.push(TagItem::new(
                ItemKey::Composer,
                ItemValue::Text(c.clone()),
            ));
        }
    } else {
        tag.remove_key(&ItemKey::Composer);
    }

    set_or_remove_str(tag, &ItemKey::Comment, metadata.comment.as_deref());
    set_or_remove_str(tag, &ItemKey::Lyrics, metadata.lyrics.as_deref());
    set_or_remove_str(tag, &ItemKey::Isrc, metadata.isrc.as_deref());
    set_or_remove_str(
        tag,
        &ItemKey::MusicBrainzRecordingId,
        metadata.musicbrainz_track_id.as_deref(),
    );
    set_or_remove_str(
        tag,
        &ItemKey::MusicBrainzReleaseId,
        metadata.musicbrainz_release_id.as_deref(),
    );
    set_or_remove_str(
        tag,
        &ItemKey::MusicBrainzArtistId,
        metadata.musicbrainz_artist_id.as_deref(),
    );
    set_or_remove_str(
        tag,
        &ItemKey::MusicBrainzReleaseGroupId,
        metadata.musicbrainz_release_group_id.as_deref(),
    );

    tag.save_to_path(path, lofty::config::WriteOptions::default())?;

    Ok(())
}

fn set_or_remove_str(tag: &mut lofty::tag::Tag, key: &ItemKey, value: Option<&str>) {
    tag.remove_key(key);
    if let Some(v) = value {
        if !v.is_empty() {
            tag.push(TagItem::new(key.clone(), ItemValue::Text(v.to_string())));
        }
    }
}
