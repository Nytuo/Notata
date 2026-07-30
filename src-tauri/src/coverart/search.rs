use serde::Deserialize;

use crate::error::{NotataError, Result};
use crate::models::album::{ArtSource, CoverArt, CoverArtType};

#[derive(Deserialize)]
struct ItunesResponse {
    results: Vec<ItunesResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItunesResult {
    collection_id: Option<u64>,
    collection_name: Option<String>,
    artist_name: Option<String>,
    artwork_url100: Option<String>,
}

pub async fn search_itunes_cover_art(query: &str) -> Result<Vec<CoverArt>> {
    let url = format!(
        "https://itunes.apple.com/search?term={}&entity=album&limit=10",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::builder()
        .user_agent("Notata/0.1.0")
        .build()?;

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(NotataError::Provider {
            provider: "itunes".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let data: ItunesResponse = response.json().await?;

    Ok(data
        .results
        .into_iter()
        .filter_map(|r| {
            let artwork_url = r.artwork_url100.as_ref()?;
            let hi_res = artwork_url.replace("100x100bb", "1200x1200bb");
            let label = format!(
                "{} — {}",
                r.artist_name.as_deref().unwrap_or("Unknown"),
                r.collection_name.as_deref().unwrap_or("Unknown")
            );
            Some(CoverArt {
                id: format!("itunes-{}", r.collection_id.unwrap_or(0)),
                art_type: CoverArtType::Front,
                source: ArtSource::Itunes,
                mime_type: "image/jpeg".to_string(),
                width: None,
                height: None,
                data_path: Some(label),
                url: Some(hi_res),
            })
        })
        .collect())
}

#[derive(Deserialize)]
struct DeezerResponse {
    data: Vec<DeezerAlbum>,
}

#[derive(Deserialize)]
struct DeezerAlbum {
    id: u64,
    title: String,
    cover_xl: Option<String>,
    cover_big: Option<String>,
    artist: DeezerArtist,
}

#[derive(Deserialize)]
struct DeezerArtist {
    name: String,
}

pub async fn search_deezer_cover_art(query: &str) -> Result<Vec<CoverArt>> {
    let url = format!(
        "https://api.deezer.com/search/album?q={}&limit=10",
        urlencoding::encode(query)
    );

    let client = reqwest::Client::builder()
        .user_agent("Notata/0.1.0")
        .build()?;

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(NotataError::Provider {
            provider: "deezer".to_string(),
            message: format!("HTTP {}", response.status()),
        });
    }

    let data: DeezerResponse = response.json().await?;

    Ok(data
        .data
        .into_iter()
        .filter_map(|album| {
            let url = album.cover_xl.or(album.cover_big)?;
            let label = format!("{} — {}", album.artist.name, album.title);
            Some(CoverArt {
                id: format!("deezer-{}", album.id),
                art_type: CoverArtType::Front,
                source: ArtSource::Deezer,
                mime_type: "image/jpeg".to_string(),
                width: None,
                height: None,
                data_path: Some(label),
                url: Some(url),
            })
        })
        .collect())
}
