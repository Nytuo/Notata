use crate::error::Result;
use crate::models::album::CoverArtData;

pub async fn download_image(url: &str) -> Result<CoverArtData> {
    let client = reqwest::Client::builder()
        .user_agent("Notata/0.1.0")
        .build()?;

    let response = client.get(url).send().await?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = response.bytes().await?;
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

    Ok(CoverArtData {
        data,
        mime_type: content_type,
        art_type: crate::models::album::CoverArtType::Front,
        width: None,
        height: None,
    })
}

pub fn embed_cover_art_in_file(
    path: &str,
    image_data: &[u8],
    mime_type: &str,
) -> Result<()> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::picture::{Picture, PictureType, MimeType};

    let mut tagged_file = Probe::open(path)?.read()?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(t) => t,
        None => {
            let tag_type = tagged_file.primary_tag_type();
            tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
            tagged_file.primary_tag_mut().unwrap()
        }
    };

    let lofty_mime = match mime_type {
        "image/png" => MimeType::Png,
        "image/bmp" => MimeType::Bmp,
        "image/gif" => MimeType::Gif,
        "image/tiff" => MimeType::Tiff,
        _ => MimeType::Jpeg,
    };

    let picture = Picture::new_unchecked(PictureType::CoverFront, Some(lofty_mime), None, image_data.to_vec());

    tag.push_picture(picture);
    tag.save_to_path(path, lofty::config::WriteOptions::default())?;

    Ok(())
}

pub fn remove_cover_art_from_file(path: &str) -> Result<()> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let mut tagged_file = Probe::open(path)?.read()?;

    if let Some(tag) = tagged_file.primary_tag_mut() {
        tag.remove_picture_type(lofty::picture::PictureType::CoverFront);
        tag.save_to_path(path, lofty::config::WriteOptions::default())?;
    }

    Ok(())
}
