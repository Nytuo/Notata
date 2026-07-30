use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::{NotataError, Result};
use crate::models::album::{AlbumMetadata, CoverArt};
use crate::models::artist::ArtistMetadata;
use crate::models::provider_result::{
    ProviderRelease, ProviderSearchResult, SearchResultType,
};
use crate::models::track::TrackMetadata;
use crate::providers::MetadataProvider;

const BASE_URL: &str = "https://musicbrainz.org/ws/2";
const USER_AGENT: &str = "Notata/0.1.0 (https://github.com/notata)";
const RATE_LIMIT: Duration = Duration::from_millis(1100);

pub struct MusicBrainzProvider {
    client: reqwest::Client,
    last_request: Mutex<Instant>,
}

impl MusicBrainzProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            last_request: Mutex::new(Instant::now() - RATE_LIMIT),
        }
    }

    fn rate_limit(&self) {
        let mut last = self.last_request.lock().unwrap();
        let elapsed = last.elapsed();
        if elapsed < RATE_LIMIT {
            std::thread::sleep(RATE_LIMIT - elapsed);
        }
        *last = Instant::now();
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.rate_limit();
        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NotataError::Provider {
                provider: "musicbrainz".to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        let body = response.json::<T>().await?;
        Ok(body)
    }
}

#[derive(Deserialize)]
struct MbReleaseSearchResponse {
    releases: Vec<MbReleaseResult>,
}

#[derive(Deserialize)]
struct MbReleaseResult {
    id: String,
    score: Option<u32>,
    title: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<MbArtistCredit>,
    #[serde(rename = "release-group", default)]
    release_group: Option<MbReleaseGroup>,
}

#[derive(Deserialize)]
struct MbArtistCredit {
    artist: MbArtist,
    #[serde(default)]
    joinphrase: Option<String>,
}

#[derive(Deserialize)]
struct MbArtist {
    id: String,
    name: String,
    #[serde(rename = "sort-name", default)]
    sort_name: Option<String>,
    #[serde(default)]
    disambiguation: Option<String>,
    #[serde(rename = "type", default)]
    artist_type: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

#[derive(Deserialize)]
struct MbReleaseGroup {
    id: String,
    #[serde(rename = "primary-type", default)]
    primary_type: Option<String>,
}

#[derive(Deserialize)]
struct MbRecordingSearchResponse {
    recordings: Vec<MbRecordingResult>,
}

#[derive(Deserialize)]
struct MbRecordingResult {
    id: String,
    score: Option<u32>,
    title: String,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<MbArtistCredit>,
    #[serde(default)]
    releases: Vec<MbRecordingRelease>,
    #[serde(default)]
    length: Option<u64>,
    #[serde(default)]
    isrcs: Vec<String>,
}

#[derive(Deserialize)]
struct MbRecordingRelease {
    id: String,
    title: String,
    #[serde(default)]
    date: Option<String>,
}

#[derive(Deserialize)]
struct MbArtistSearchResponse {
    artists: Vec<MbArtist>,
}

#[derive(Deserialize)]
struct MbReleaseDetail {
    id: String,
    title: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    barcode: Option<String>,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<MbArtistCredit>,
    #[serde(rename = "release-group", default)]
    release_group: Option<MbReleaseGroup>,
    #[serde(rename = "label-info", default)]
    label_info: Vec<MbLabelInfo>,
    #[serde(default)]
    media: Vec<MbMedia>,
}

#[derive(Deserialize)]
struct MbLabelInfo {
    #[serde(rename = "catalog-number", default)]
    catalog_number: Option<String>,
    #[serde(default)]
    label: Option<MbLabel>,
}

#[derive(Deserialize)]
struct MbLabel {
    name: String,
}

#[derive(Deserialize)]
struct MbMedia {
    position: u32,
    #[serde(rename = "track-count")]
    track_count: u32,
    #[serde(default)]
    tracks: Vec<MbTrack>,
}

#[derive(Deserialize)]
struct MbTrack {
    position: u32,
    title: String,
    #[serde(default)]
    length: Option<u64>,
    recording: MbTrackRecording,
}

#[derive(Deserialize)]
struct MbTrackRecording {
    id: String,
    #[serde(rename = "artist-credit", default)]
    artist_credit: Vec<MbArtistCredit>,
    #[serde(default)]
    isrcs: Vec<String>,
}

fn format_artist_credit(credits: &[MbArtistCredit]) -> String {
    credits
        .iter()
        .map(|c| {
            let name = c.artist.name.clone();
            match &c.joinphrase {
                Some(jp) => format!("{}{}", name, jp),
                None => name,
            }
        })
        .collect::<String>()
}

fn extract_year(date: &Option<String>) -> Option<i32> {
    date.as_ref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse().ok())
}

#[async_trait]
impl MetadataProvider for MusicBrainzProvider {
    fn id(&self) -> &str {
        "musicbrainz"
    }

    fn display_name(&self) -> &str {
        "MusicBrainz"
    }

    fn supported_media_types(&self) -> Vec<String> {
        vec!["audio".to_string()]
    }

    async fn search_release(
        &self,
        query: &str,
        artist: Option<&str>,
    ) -> Result<Vec<ProviderSearchResult>> {
        let search_query = match artist {
            Some(a) => format!("release:\"{}\" AND artist:\"{}\"", query, a),
            None => format!("release:\"{}\"", query),
        };

        let url = format!(
            "{}/release/?query={}&limit=25&fmt=json",
            BASE_URL,
            urlencoding::encode(&search_query)
        );

        let response: MbReleaseSearchResponse = self.get_json(&url).await?;

        Ok(response
            .releases
            .into_iter()
            .map(|r| {
                let artist_name = format_artist_credit(&r.artist_credit);
                let year = extract_year(&r.date);
                let mut extra = HashMap::new();
                if let Some(rg) = &r.release_group {
                    if let Some(pt) = &rg.primary_type {
                        extra.insert(
                            "releaseType".to_string(),
                            serde_json::Value::String(pt.clone()),
                        );
                    }
                }

                ProviderSearchResult {
                    provider: "musicbrainz".to_string(),
                    result_type: SearchResultType::Release,
                    id: r.id,
                    score: r.score.map(|s| s as f64),
                    title: r.title,
                    artist: Some(artist_name),
                    year,
                    extra,
                }
            })
            .collect())
    }

    async fn search_recording(
        &self,
        query: &str,
        artist: Option<&str>,
    ) -> Result<Vec<ProviderSearchResult>> {
        let search_query = match artist {
            Some(a) => format!("recording:\"{}\" AND artist:\"{}\"", query, a),
            None => format!("recording:\"{}\"", query),
        };

        let url = format!(
            "{}/recording/?query={}&limit=25&fmt=json",
            BASE_URL,
            urlencoding::encode(&search_query)
        );

        let response: MbRecordingSearchResponse = self.get_json(&url).await?;

        Ok(response
            .recordings
            .into_iter()
            .map(|r| {
                let artist_name = format_artist_credit(&r.artist_credit);
                let year = r
                    .releases
                    .first()
                    .and_then(|rel| extract_year(&rel.date));

                ProviderSearchResult {
                    provider: "musicbrainz".to_string(),
                    result_type: SearchResultType::Recording,
                    id: r.id,
                    score: r.score.map(|s| s as f64),
                    title: r.title,
                    artist: Some(artist_name),
                    year,
                    extra: HashMap::new(),
                }
            })
            .collect())
    }

    async fn search_artist(&self, query: &str) -> Result<Vec<ProviderSearchResult>> {
        let url = format!(
            "{}/artist/?query=artist:\"{}\"&limit=25&fmt=json",
            BASE_URL,
            urlencoding::encode(query)
        );

        let response: MbArtistSearchResponse = self.get_json(&url).await?;

        Ok(response
            .artists
            .into_iter()
            .map(|a| ProviderSearchResult {
                provider: "musicbrainz".to_string(),
                result_type: SearchResultType::Artist,
                id: a.id,
                score: None,
                title: a.name,
                artist: None,
                year: None,
                extra: HashMap::new(),
            })
            .collect())
    }

    async fn get_release(&self, id: &str) -> Result<ProviderRelease> {
        let url = format!(
            "{}/release/{}?inc=recordings+artists+labels+artist-credits+isrcs&fmt=json",
            BASE_URL, id
        );

        let release: MbReleaseDetail = self.get_json(&url).await?;

        let artist_name = format_artist_credit(&release.artist_credit);
        let year = extract_year(&release.date);
        let total_discs = release.media.len() as u32;

        let label = release
            .label_info
            .first()
            .and_then(|li| li.label.as_ref())
            .map(|l| l.name.clone());
        let catalog_number = release
            .label_info
            .first()
            .and_then(|li| li.catalog_number.clone());

        let release_type = release
            .release_group
            .as_ref()
            .and_then(|rg| rg.primary_type.clone());
        let release_group_id = release.release_group.as_ref().map(|rg| rg.id.clone());

        let mut tracks = Vec::new();
        for medium in &release.media {
            for track in &medium.tracks {
                let track_artist = format_artist_credit(&track.recording.artist_credit);
                tracks.push(TrackMetadata {
                    title: Some(track.title.clone()),
                    artist: Some(track_artist),
                    album_artist: Some(artist_name.clone()),
                    album: Some(release.title.clone()),
                    track_number: Some(track.position),
                    total_tracks: Some(medium.track_count),
                    disc_number: Some(medium.position),
                    total_discs: Some(total_discs),
                    year,
                    date: release.date.clone(),
                    genre: None,
                    composer: None,
                    comment: None,
                    lyrics: None,
                    isrc: track.recording.isrcs.first().cloned(),
                    musicbrainz_track_id: Some(track.recording.id.clone()),
                    musicbrainz_release_id: Some(release.id.clone()),
                    musicbrainz_artist_id: release
                        .artist_credit
                        .first()
                        .map(|c| c.artist.id.clone()),
                    musicbrainz_release_group_id: release_group_id.clone(),
                    custom_tags: Default::default(),
                });
            }
        }

        let artists: Vec<ArtistMetadata> = release
            .artist_credit
            .iter()
            .map(|c| ArtistMetadata {
                name: c.artist.name.clone(),
                sort_name: c.artist.sort_name.clone(),
                musicbrainz_artist_id: Some(c.artist.id.clone()),
                disambiguation: c.artist.disambiguation.clone(),
                artist_type: c.artist.artist_type.clone(),
                country: c.artist.country.clone(),
            })
            .collect();

        let total_tracks = release.media.iter().map(|m| m.track_count).sum();

        Ok(ProviderRelease {
            provider: "musicbrainz".to_string(),
            id: release.id.clone(),
            album: AlbumMetadata {
                title: release.title,
                artist: artist_name,
                year,
                release_date: release.date,
                genre: None,
                label,
                catalog_number,
                barcode: release.barcode,
                release_type,
                release_country: release.country,
                musicbrainz_release_id: Some(release.id),
                musicbrainz_release_group_id: release_group_id,
                total_tracks: Some(total_tracks),
                total_discs: Some(total_discs),
                cover_art: Vec::new(),
            },
            tracks,
            artists,
        })
    }

    async fn get_cover_art(&self, release_id: &str) -> Result<Vec<CoverArt>> {
        crate::providers::coverart::fetch_cover_art_archive(release_id).await
    }
}
