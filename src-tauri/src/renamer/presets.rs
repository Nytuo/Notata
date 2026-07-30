use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresetKind {
    Music,
    Movie,
    Series,
    Book,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePreset {
    pub id: String,
    pub label: String,
    /// Which media server convention this follows.
    pub server: String,
    pub kind: PresetKind,
    pub template: String,
    pub description: String,
}

/// Built-in templates matching the layouts Plex, Jellyfin, and Navidrome
/// document for their scanners.
pub fn builtin_presets() -> Vec<RenamePreset> {
    vec![
        // ---- Music -------------------------------------------------------
        RenamePreset {
            id: "plex-music".into(),
            label: "Plex — Music".into(),
            server: "Plex".into(),
            kind: PresetKind::Music,
            template: "{albumartist}/{album}[ ({year})]/[{disc}-]{track:02} - {title}".into(),
            description: "Artist/Album (Year)/01 - Title".into(),
        },
        RenamePreset {
            id: "jellyfin-music".into(),
            label: "Jellyfin — Music".into(),
            server: "Jellyfin".into(),
            kind: PresetKind::Music,
            template: "{albumartist}/{album}[ ({year})]/{track:02} - {title}".into(),
            description: "Artist/Album (Year)/01 - Title".into(),
        },
        RenamePreset {
            id: "navidrome-music".into(),
            label: "Navidrome — Music".into(),
            server: "Navidrome".into(),
            kind: PresetKind::Music,
            template: "{albumartist}/{album}/{track:02} - {title}".into(),
            description: "Artist/Album/01 - Title".into(),
        },
        RenamePreset {
            id: "flat-music".into(),
            label: "Flat — Artist - Title".into(),
            server: "Generic".into(),
            kind: PresetKind::Music,
            template: "{artist} - {title}".into(),
            description: "Artist - Title (no folders)".into(),
        },
        // ---- Movies ------------------------------------------------------
        RenamePreset {
            id: "plex-movie".into(),
            label: "Plex — Movies".into(),
            server: "Plex".into(),
            kind: PresetKind::Movie,
            template: "{title}[ ({year})]/{title}[ ({year})][ - {edition}]".into(),
            description: "Movie (Year)/Movie (Year).ext".into(),
        },
        RenamePreset {
            id: "jellyfin-movie".into(),
            label: "Jellyfin — Movies".into(),
            server: "Jellyfin".into(),
            kind: PresetKind::Movie,
            template: "{title}[ ({year})]/{title}[ ({year})][ [imdbid-{imdbid}]]".into(),
            description: "Movie (Year)/Movie (Year) [imdbid-tt123].ext".into(),
        },
        // ---- Series ------------------------------------------------------
        RenamePreset {
            id: "plex-series".into(),
            label: "Plex — TV Shows".into(),
            server: "Plex".into(),
            kind: PresetKind::Series,
            template:
                "{seriestitle}[ ({year})]/Season {season:02}/{seriestitle} - s{season:02}e{episode:02}[ - {episodetitle}]"
                    .into(),
            description: "Show (Year)/Season 01/Show - s01e01 - Title.ext".into(),
        },
        RenamePreset {
            id: "jellyfin-series".into(),
            label: "Jellyfin — TV Shows".into(),
            server: "Jellyfin".into(),
            kind: PresetKind::Series,
            template:
                "{seriestitle}[ ({year})]/Season {season:02}/{seriestitle} S{season:02}E{episode:02}[ {episodetitle}]"
                    .into(),
            description: "Show (Year)/Season 01/Show S01E01 Title.ext".into(),
        },
        // ---- Comics & books ----------------------------------------------
        RenamePreset {
            id: "comic-series".into(),
            label: "Comics — Series/Issue".into(),
            server: "Komga / Kavita".into(),
            kind: PresetKind::Book,
            template: "{series}/{series} #{number}[ ({year})]".into(),
            description: "Series/Series #012 (2011).cbz".into(),
        },
        RenamePreset {
            id: "ebook-author".into(),
            label: "Books — Author/Title".into(),
            server: "Calibre".into(),
            kind: PresetKind::Book,
            template: "{author}/[{series}/]{title}[ ({year})]".into(),
            description: "Author/Series/Title (Year).epub".into(),
        },
    ]
}
