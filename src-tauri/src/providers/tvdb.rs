use std::sync::RwLock;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::{NotataError, Result};
use crate::models::video::{
    EpisodeMetadata, MovieMetadata, RemoteArtwork, SeriesMetadata, VideoResultType,
    VideoSearchResult,
};
use crate::providers::video::VideoMetadataProvider;

const TVDB_BASE: &str = "https://api4.thetvdb.com/v4";

pub struct TvdbProvider {
    api_key: RwLock<Option<String>>,
    /// Bearer token from `/login`. Async mutex because it is held across the
    /// login request itself.
    token: AsyncMutex<Option<String>>,
    client: reqwest::Client,
}

impl TvdbProvider {
    pub fn new() -> Self {
        Self {
            api_key: RwLock::new(None),
            token: AsyncMutex::new(None),
            client: reqwest::Client::builder()
                .user_agent("Notata/0.1.0")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    fn key(&self) -> Result<String> {
        self.api_key
            .read()
            .ok()
            .and_then(|k| k.clone())
            .ok_or_else(|| NotataError::Provider {
                provider: "tvdb".to_string(),
                message: "No TheTVDB API key configured. Add one in Settings.".to_string(),
            })
    }

    fn err(message: impl Into<String>) -> NotataError {
        NotataError::Provider {
            provider: "tvdb".to_string(),
            message: message.into(),
        }
    }

    /// Return a cached bearer token, logging in on first use.
    async fn token(&self) -> Result<String> {
        let mut guard = self.token.lock().await;
        if let Some(t) = guard.as_ref() {
            return Ok(t.clone());
        }

        let key = self.key()?;
        let response = self
            .client
            .post(format!("{}/login", TVDB_BASE))
            .json(&serde_json::json!({ "apikey": key }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Self::err(format!(
                "login failed with HTTP {}",
                response.status()
            )));
        }

        let body: TvdbResponse<TvdbLoginData> = response.json().await?;
        let token = body.data.token;
        *guard = Some(token.clone());
        Ok(token)
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let token = self.token().await?;
        let mut url = format!("{}{}", TVDB_BASE, path);
        if !query.is_empty() {
            let qs: Vec<String> = query
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&qs.join("&"));
        }

        let response = self.client.get(&url).bearer_auth(&token).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            // Tokens expire; drop the cached one so the next call logs in again.
            *self.token.lock().await = None;
            return Err(Self::err("TheTVDB token expired or was rejected."));
        }
        if !status.is_success() {
            return Err(Self::err(format!("HTTP {}", status)));
        }

        let body: TvdbResponse<T> = response.json().await?;
        Ok(body.data)
    }
}

impl Default for TvdbProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TvdbResponse<T> {
    data: T,
}

#[derive(Deserialize)]
struct TvdbLoginData {
    token: String,
}

#[derive(Deserialize)]
struct TvdbSearchItem {
    #[serde(default)]
    tvdb_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    year: Option<String>,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    image_url: Option<String>,
}

#[derive(Deserialize)]
struct TvdbNameHolder {
    name: String,
}

#[derive(Deserialize)]
struct TvdbSeriesExtended {
    id: u64,
    name: Option<String>,
    overview: Option<String>,
    #[serde(default)]
    first_aired: Option<String>,
    #[serde(default)]
    status: Option<TvdbNameHolder>,
    #[serde(default)]
    genres: Vec<TvdbNameHolder>,
    #[serde(default)]
    score: Option<f32>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Deserialize)]
struct TvdbEpisode {
    id: Option<u64>,
    name: Option<String>,
    #[serde(default, rename = "seasonNumber")]
    season_number: Option<u32>,
    #[serde(default)]
    number: Option<u32>,
    #[serde(default)]
    aired: Option<String>,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    runtime: Option<u32>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Deserialize)]
struct TvdbEpisodesData {
    #[serde(default)]
    episodes: Vec<TvdbEpisode>,
}

#[derive(Deserialize)]
struct TvdbArtwork {
    image: Option<String>,
    thumbnail: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    score: Option<f32>,
    /// TheTVDB artwork type ids: 2/7 are posters, 3/15 are backgrounds.
    #[serde(default)]
    #[serde(rename = "type")]
    art_type: Option<u32>,
}

#[derive(Deserialize)]
struct TvdbArtworkData {
    #[serde(default)]
    artworks: Vec<TvdbArtwork>,
}

fn parse_year(year: &Option<String>) -> Option<i32> {
    year.as_ref().and_then(|y| y.get(0..4)?.parse().ok())
}

#[async_trait]
impl VideoMetadataProvider for TvdbProvider {
    fn id(&self) -> &str {
        "tvdb"
    }

    fn display_name(&self) -> &str {
        "TheTVDB"
    }

    fn is_configured(&self) -> bool {
        self.api_key.read().map(|k| k.is_some()).unwrap_or(false)
    }

    fn configure(&self, api_key: Option<String>) {
        if let Ok(mut k) = self.api_key.write() {
            *k = api_key.filter(|s| !s.trim().is_empty());
        }
        // Force a fresh login against the new key.
        if let Ok(mut t) = self.token.try_lock() {
            *t = None;
        }
    }

    async fn search_movie(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<VideoSearchResult>> {
        let mut params = vec![("query", query.to_string()), ("type", "movie".to_string())];
        if let Some(y) = year {
            params.push(("year", y.to_string()));
        }

        let items: Vec<TvdbSearchItem> = self.get("/search", &params).await?;

        Ok(items
            .into_iter()
            .filter_map(|i| {
                Some(VideoSearchResult {
                    provider: "tvdb".to_string(),
                    result_type: VideoResultType::Movie,
                    id: i.tvdb_id?,
                    title: i.name.unwrap_or_default(),
                    year: parse_year(&i.year),
                    overview: i.overview,
                    poster_url: i.image_url,
                    rating: None,
                })
            })
            .collect())
    }

    async fn search_series(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<VideoSearchResult>> {
        let mut params = vec![("query", query.to_string()), ("type", "series".to_string())];
        if let Some(y) = year {
            params.push(("year", y.to_string()));
        }

        let items: Vec<TvdbSearchItem> = self.get("/search", &params).await?;

        Ok(items
            .into_iter()
            .filter_map(|i| {
                Some(VideoSearchResult {
                    provider: "tvdb".to_string(),
                    result_type: VideoResultType::Series,
                    id: i.tvdb_id?,
                    title: i.name.unwrap_or_default(),
                    year: parse_year(&i.year),
                    overview: i.overview,
                    poster_url: i.image_url,
                    rating: None,
                })
            })
            .collect())
    }

    async fn get_movie(&self, id: &str) -> Result<MovieMetadata> {
        let detail: TvdbSeriesExtended = self.get(&format!("/movies/{}/extended", id), &[]).await?;

        Ok(MovieMetadata {
            title: detail.name.unwrap_or_default(),
            year: parse_year(&detail.first_aired),
            release_date: detail.first_aired,
            overview: detail.overview,
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.score,
            poster_url: detail.image,
            ..Default::default()
        })
    }

    async fn get_series(&self, id: &str) -> Result<SeriesMetadata> {
        let detail: TvdbSeriesExtended = self.get(&format!("/series/{}/extended", id), &[]).await?;

        Ok(SeriesMetadata {
            title: detail.name.unwrap_or_default(),
            year: parse_year(&detail.first_aired),
            first_aired: detail.first_aired,
            overview: detail.overview,
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.score,
            network: None,
            status: detail.status.map(|s| s.name),
            total_seasons: None,
            tmdb_id: None,
            tvdb_id: Some(detail.id.to_string()),
            imdb_id: None,
            poster_url: detail.image,
            backdrop_url: None,
            episodes: Vec::new(),
        })
    }

    async fn get_episodes(
        &self,
        series_id: &str,
        season: Option<u32>,
    ) -> Result<Vec<EpisodeMetadata>> {
        let data: TvdbEpisodesData = self
            .get(&format!("/series/{}/episodes/default", series_id), &[])
            .await?;

        Ok(data
            .episodes
            .into_iter()
            .filter(|e| season.is_none() || e.season_number == season)
            .map(|e| EpisodeMetadata {
                title: e.name.unwrap_or_default(),
                season: e.season_number.unwrap_or(0),
                episode: e.number.unwrap_or(0),
                air_date: e.aired,
                overview: e.overview,
                rating: None,
                runtime_minutes: e.runtime,
                still_url: e.image,
                tmdb_id: None,
                tvdb_id: e.id.map(|i| i.to_string()),
            })
            .collect())
    }

    async fn get_artwork(&self, id: &str, is_series: bool) -> Result<Vec<RemoteArtwork>> {
        let path = if is_series {
            format!("/series/{}/extended", id)
        } else {
            format!("/movies/{}/extended", id)
        };

        let data: TvdbArtworkData = self.get(&path, &[]).await?;

        Ok(data
            .artworks
            .into_iter()
            .filter_map(|a| {
                let url = a.image?;
                // Only posters and backgrounds are useful here; TheTVDB also
                // returns icons, banners, and clear art.
                let art_type = match a.art_type {
                    Some(2) | Some(7) | Some(14) => "poster",
                    Some(3) | Some(15) => "backdrop",
                    _ => return None,
                };
                Some(RemoteArtwork {
                    provider: "tvdb".to_string(),
                    art_type: art_type.to_string(),
                    thumb_url: a.thumbnail.clone().unwrap_or_else(|| url.clone()),
                    url,
                    width: a.width,
                    height: a.height,
                    language: a.language,
                    rating: a.score,
                })
            })
            .collect())
    }
}
