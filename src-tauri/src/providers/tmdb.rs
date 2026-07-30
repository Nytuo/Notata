use std::sync::RwLock;

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::{NotataError, Result};
use crate::models::video::{
    EpisodeMetadata, MovieMetadata, RemoteArtwork, SeriesMetadata, VideoResultType,
    VideoSearchResult,
};
use crate::providers::video::VideoMetadataProvider;

const TMDB_BASE: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

pub struct TmdbProvider {
    api_key: RwLock<Option<String>>,
    client: reqwest::Client,
}

impl TmdbProvider {
    pub fn new() -> Self {
        Self {
            api_key: RwLock::new(None),
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
                provider: "tmdb".to_string(),
                message: "No TMDB API key configured. Add one in Settings.".to_string(),
            })
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str, extra: &[(&str, String)]) -> Result<T> {
        let key = self.key()?;
        let mut url = format!("{}{}?api_key={}", TMDB_BASE, path, key);
        for (k, v) in extra {
            url.push_str(&format!("&{}={}", k, urlencoding::encode(v)));
        }

        let response = self.client.get(&url).send().await?;
        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(NotataError::Provider {
                provider: "tmdb".to_string(),
                message: "TMDB rejected the API key.".to_string(),
            });
        }
        if !status.is_success() {
            return Err(NotataError::Provider {
                provider: "tmdb".to_string(),
                message: format!("HTTP {}", status),
            });
        }

        Ok(response.json::<T>().await?)
    }
}

impl Default for TmdbProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn image_url(path: &Option<String>, size: &str) -> Option<String> {
    path.as_ref()
        .map(|p| format!("{}/{}{}", TMDB_IMAGE_BASE, size, p))
}

/// TMDB dates are `YYYY-MM-DD`; take the leading year.
fn year_from_date(date: &Option<String>) -> Option<i32> {
    date.as_ref()
        .and_then(|d| d.get(0..4))
        .and_then(|y| y.parse().ok())
}

#[derive(Deserialize)]
struct TmdbSearchResponse<T> {
    results: Vec<T>,
}

#[derive(Deserialize)]
struct TmdbMovieResult {
    id: u64,
    title: String,
    release_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    vote_average: Option<f32>,
}

#[derive(Deserialize)]
struct TmdbSeriesResult {
    id: u64,
    name: String,
    first_air_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    vote_average: Option<f32>,
}

#[derive(Deserialize)]
struct TmdbGenre {
    name: String,
}

#[derive(Deserialize)]
struct TmdbCompany {
    name: String,
}

#[derive(Deserialize)]
struct TmdbCountry {
    iso_3166_1: String,
}

#[derive(Deserialize)]
struct TmdbCrewMember {
    name: String,
    job: String,
}

#[derive(Deserialize)]
struct TmdbCastMember {
    name: String,
}

#[derive(Deserialize)]
struct TmdbCredits {
    #[serde(default)]
    cast: Vec<TmdbCastMember>,
    #[serde(default)]
    crew: Vec<TmdbCrewMember>,
}

#[derive(Deserialize)]
struct TmdbMovieDetail {
    id: u64,
    title: String,
    original_title: Option<String>,
    release_date: Option<String>,
    tagline: Option<String>,
    overview: Option<String>,
    runtime: Option<u32>,
    #[serde(default)]
    genres: Vec<TmdbGenre>,
    vote_average: Option<f32>,
    #[serde(default)]
    production_companies: Vec<TmdbCompany>,
    #[serde(default)]
    production_countries: Vec<TmdbCountry>,
    imdb_id: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    credits: Option<TmdbCredits>,
}

#[derive(Deserialize)]
struct TmdbNetwork {
    name: String,
}

#[derive(Deserialize)]
struct TmdbExternalIds {
    imdb_id: Option<String>,
    tvdb_id: Option<u64>,
}

#[derive(Deserialize)]
struct TmdbSeriesDetail {
    id: u64,
    name: String,
    first_air_date: Option<String>,
    overview: Option<String>,
    #[serde(default)]
    genres: Vec<TmdbGenre>,
    vote_average: Option<f32>,
    #[serde(default)]
    networks: Vec<TmdbNetwork>,
    status: Option<String>,
    number_of_seasons: Option<u32>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    external_ids: Option<TmdbExternalIds>,
}

#[derive(Deserialize)]
struct TmdbEpisode {
    id: Option<u64>,
    name: Option<String>,
    season_number: Option<u32>,
    episode_number: Option<u32>,
    air_date: Option<String>,
    overview: Option<String>,
    vote_average: Option<f32>,
    runtime: Option<u32>,
    still_path: Option<String>,
}

#[derive(Deserialize)]
struct TmdbSeason {
    #[serde(default)]
    episodes: Vec<TmdbEpisode>,
}

#[derive(Deserialize)]
struct TmdbImage {
    file_path: String,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    iso_639_1: Option<String>,
    vote_average: Option<f32>,
}

#[derive(Deserialize)]
struct TmdbImages {
    #[serde(default)]
    posters: Vec<TmdbImage>,
    #[serde(default)]
    backdrops: Vec<TmdbImage>,
}

impl From<TmdbEpisode> for EpisodeMetadata {
    fn from(e: TmdbEpisode) -> Self {
        EpisodeMetadata {
            title: e.name.unwrap_or_default(),
            season: e.season_number.unwrap_or(0),
            episode: e.episode_number.unwrap_or(0),
            air_date: e.air_date,
            overview: e.overview,
            rating: e.vote_average,
            runtime_minutes: e.runtime,
            still_url: image_url(&e.still_path, "w300"),
            tmdb_id: e.id.map(|i| i.to_string()),
            tvdb_id: None,
        }
    }
}

#[async_trait]
impl VideoMetadataProvider for TmdbProvider {
    fn id(&self) -> &str {
        "tmdb"
    }

    fn display_name(&self) -> &str {
        "TMDB"
    }

    fn is_configured(&self) -> bool {
        self.api_key
            .read()
            .map(|k| k.is_some())
            .unwrap_or(false)
    }

    fn configure(&self, api_key: Option<String>) {
        if let Ok(mut k) = self.api_key.write() {
            *k = api_key.filter(|s| !s.trim().is_empty());
        }
    }

    async fn search_movie(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<VideoSearchResult>> {
        let mut extra = vec![("query", query.to_string())];
        if let Some(y) = year {
            extra.push(("year", y.to_string()));
        }

        let resp: TmdbSearchResponse<TmdbMovieResult> =
            self.get("/search/movie", &extra).await?;

        Ok(resp
            .results
            .into_iter()
            .map(|m| VideoSearchResult {
                provider: "tmdb".to_string(),
                result_type: VideoResultType::Movie,
                id: m.id.to_string(),
                title: m.title,
                year: year_from_date(&m.release_date),
                overview: m.overview,
                poster_url: image_url(&m.poster_path, "w342"),
                rating: m.vote_average,
            })
            .collect())
    }

    async fn search_series(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<VideoSearchResult>> {
        let mut extra = vec![("query", query.to_string())];
        if let Some(y) = year {
            extra.push(("first_air_date_year", y.to_string()));
        }

        let resp: TmdbSearchResponse<TmdbSeriesResult> = self.get("/search/tv", &extra).await?;

        Ok(resp
            .results
            .into_iter()
            .map(|s| VideoSearchResult {
                provider: "tmdb".to_string(),
                result_type: VideoResultType::Series,
                id: s.id.to_string(),
                title: s.name,
                year: year_from_date(&s.first_air_date),
                overview: s.overview,
                poster_url: image_url(&s.poster_path, "w342"),
                rating: s.vote_average,
            })
            .collect())
    }

    async fn get_movie(&self, id: &str) -> Result<MovieMetadata> {
        let detail: TmdbMovieDetail = self
            .get(
                &format!("/movie/{}", id),
                &[("append_to_response", "credits".to_string())],
            )
            .await?;

        let (directors, writers, cast) = match detail.credits {
            Some(c) => {
                let directors = c
                    .crew
                    .iter()
                    .filter(|m| m.job == "Director")
                    .map(|m| m.name.clone())
                    .collect();
                let writers = c
                    .crew
                    .iter()
                    .filter(|m| m.job == "Writer" || m.job == "Screenplay")
                    .map(|m| m.name.clone())
                    .collect();
                // Full billing is rarely useful in a tag; keep the top names.
                let cast = c.cast.iter().take(20).map(|m| m.name.clone()).collect();
                (directors, writers, cast)
            }
            None => (Vec::new(), Vec::new(), Vec::new()),
        };

        Ok(MovieMetadata {
            title: detail.title,
            original_title: detail.original_title,
            year: year_from_date(&detail.release_date),
            release_date: detail.release_date,
            tagline: detail.tagline.filter(|t| !t.is_empty()),
            overview: detail.overview,
            runtime_minutes: detail.runtime,
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.vote_average,
            country: detail
                .production_countries
                .first()
                .map(|c| c.iso_3166_1.clone()),
            studios: detail.production_companies.into_iter().map(|c| c.name).collect(),
            directors,
            writers,
            cast,
            tmdb_id: Some(detail.id.to_string()),
            imdb_id: detail.imdb_id,
            poster_url: image_url(&detail.poster_path, "original"),
            backdrop_url: image_url(&detail.backdrop_path, "original"),
        })
    }

    async fn get_series(&self, id: &str) -> Result<SeriesMetadata> {
        let detail: TmdbSeriesDetail = self
            .get(
                &format!("/tv/{}", id),
                &[("append_to_response", "external_ids".to_string())],
            )
            .await?;

        let external = detail.external_ids;

        Ok(SeriesMetadata {
            title: detail.name,
            year: year_from_date(&detail.first_air_date),
            first_aired: detail.first_air_date,
            overview: detail.overview,
            genres: detail.genres.into_iter().map(|g| g.name).collect(),
            rating: detail.vote_average,
            network: detail.networks.first().map(|n| n.name.clone()),
            status: detail.status,
            total_seasons: detail.number_of_seasons,
            tmdb_id: Some(detail.id.to_string()),
            tvdb_id: external.as_ref().and_then(|e| e.tvdb_id).map(|i| i.to_string()),
            imdb_id: external.and_then(|e| e.imdb_id),
            poster_url: image_url(&detail.poster_path, "original"),
            backdrop_url: image_url(&detail.backdrop_path, "original"),
            episodes: Vec::new(),
        })
    }

    async fn get_episodes(
        &self,
        series_id: &str,
        season: Option<u32>,
    ) -> Result<Vec<EpisodeMetadata>> {
        // TMDB only exposes episodes per season, so without an explicit season
        // we walk every season the series reports.
        let seasons: Vec<u32> = match season {
            Some(s) => vec![s],
            None => {
                let detail: TmdbSeriesDetail =
                    self.get(&format!("/tv/{}", series_id), &[]).await?;
                (1..=detail.number_of_seasons.unwrap_or(0)).collect()
            }
        };

        let mut episodes = Vec::new();
        for s in seasons {
            let season_detail: TmdbSeason = self
                .get(&format!("/tv/{}/season/{}", series_id, s), &[])
                .await?;
            episodes.extend(season_detail.episodes.into_iter().map(EpisodeMetadata::from));
        }

        Ok(episodes)
    }

    async fn get_artwork(&self, id: &str, is_series: bool) -> Result<Vec<RemoteArtwork>> {
        let path = if is_series {
            format!("/tv/{}/images", id)
        } else {
            format!("/movie/{}/images", id)
        };

        // `include_image_language` keeps language-neutral art in the results,
        // which is usually what a tagger wants.
        let images: TmdbImages = self
            .get(&path, &[("include_image_language", "en,null".to_string())])
            .await?;

        let convert = |img: TmdbImage, art_type: &str| RemoteArtwork {
            provider: "tmdb".to_string(),
            art_type: art_type.to_string(),
            url: format!("{}/original{}", TMDB_IMAGE_BASE, img.file_path),
            thumb_url: format!("{}/w342{}", TMDB_IMAGE_BASE, img.file_path),
            width: img.width,
            height: img.height,
            language: img.iso_639_1,
            rating: img.vote_average,
        };

        let mut out: Vec<RemoteArtwork> = images
            .posters
            .into_iter()
            .map(|i| convert(i, "poster"))
            .collect();
        out.extend(
            images
                .backdrops
                .into_iter()
                .map(|i| convert(i, "backdrop")),
        );

        Ok(out)
    }
}
