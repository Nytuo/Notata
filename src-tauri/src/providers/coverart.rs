use serde::Deserialize;

use crate::error::{NotataError, Result};
use crate::models::album::{ArtSource, CoverArt, CoverArtType};

const CAA_BASE_URL: &str = "https://coverartarchive.org";

#[derive(Deserialize)]
struct CaaResponse {
    images: Vec<CaaImage>,
}

#[derive(Deserialize)]
struct CaaImage {
    id: u64,
    image: String,
    thumbnails: CaaThumbnails,
    front: bool,
    back: bool,
    #[serde(default)]
    types: Vec<String>,
}

#[derive(Deserialize)]
struct CaaThumbnails {
    #[serde(rename = "250", default)]
    small: Option<String>,
    #[serde(rename = "500", default)]
    large: Option<String>,
    #[serde(rename = "1200", default)]
    xl: Option<String>,
}

pub async fn fetch_cover_art_archive(release_id: &str) -> Result<Vec<CoverArt>> {
    let url = format!("{}/release/{}", CAA_BASE_URL, release_id);

    let client = reqwest::Client::builder()
        .user_agent("Notata/0.1.0")
        .build()
        .map_err(|e| NotataError::Http(e))?;

    let response = client.get(&url).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !response.status().is_success() {
        return Err(NotataError::Provider {
            provider: "coverartarchive".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let caa: CaaResponse = response.json().await?;

    Ok(caa
        .images
        .into_iter()
        .map(|img| {
            let art_type = if img.front {
                CoverArtType::Front
            } else if img.back {
                CoverArtType::Back
            } else if img.types.iter().any(|t| t == "Disc") {
                CoverArtType::Disc
            } else if img.types.iter().any(|t| t == "Booklet") {
                CoverArtType::Booklet
            } else {
                CoverArtType::Other(
                    img.types.first().cloned().unwrap_or_else(|| "Other".to_string()),
                )
            };

            let best_url = img
                .thumbnails
                .xl
                .or(img.thumbnails.large)
                .unwrap_or(img.image);

            CoverArt {
                id: img.id.to_string(),
                art_type,
                source: ArtSource::CoverArtArchive,
                mime_type: "image/jpeg".to_string(),
                width: None,
                height: None,
                data_path: None,
                url: Some(best_url),
            }
        })
        .collect())
}
