<div align="center">
<img src="src-tauri/icons/icon.png" alt="Notata Logo" width="200"/>
<h1>Notata</h1>
Tag your media once, and let every server read it

  <br />
  <br />
  <a href="https://github.com/Nytuo/Notata/issues/new?labels=bug&title=bug%3A+">Report a Bug</a>
  ·
  <a href="https://github.com/Nytuo/Notata/issues/new?labels=enhancement&title=feat%3A+">Request a Feature</a>
  ·
  <a href="https://github.com/Nytuo/Notata/discussions">Ask a Question</a>

</div>

<div align="center">
<br />

[![Project license](https://img.shields.io/github/license/Nytuo/Notata.svg?style=flat-square)](LICENSE)

[![code with love by Nytuo](https://img.shields.io/badge/%3C%2F%3E%20with%20%E2%99%A5%20by-Nytuo-ff1414.svg?style=flat-square)](https://github.com/Nytuo)

</div>

<details open="open">
<summary>Table of Contents</summary>

- [About](#about)
- [What Notata Can Do](#what-notata-can-do)
  - [Music](#music)
  - [Movies \& Series](#movies--series)
  - [Comics \& Books](#comics--books)
  - [Across every library](#across-every-library)
- [Supported Formats](#supported-formats)
- [Metadata Sources](#metadata-sources)
- [Renaming](#renaming)
  - [Template syntax](#template-syntax)
  - [Presets](#presets)
- [Getting Started](#getting-started)
  - [API keys](#api-keys)
  - [Building from source](#building-from-source)
- [Project Status](#project-status)
- [Technologies](#technologies)
- [Authors \& contributors](#authors--contributors)
- [License](#license)

</details>

---

## About

Notata is a desktop metadata manager for media libraries. It sits somewhere
between MusicBrainz Picard, mp3tag, and TinyMediaManager — one application that
reads and writes tags for **music, movies, series, comics, and books**, using
the formats your media server already understands.

Nothing is locked in a proprietary database. Music edits go into the audio
file's own tags, movies and series get Kodi-style NFO sidecars and poster
files, and comics and ebooks are written straight into the XML inside the
archive. Point Plex, Jellyfin, Navidrome, Komga, or Calibre at the same folder
afterwards and everything is simply there.

## What Notata Can Do

Every library folder you add is tagged as **Music**, **Movies**, **TV Series**,
or **Comics & Books**, which decides the editor and the write format for the
files inside it.

### Music

- Read and write embedded tags — ID3v2, Vorbis comments, MP4 atoms, APE, and more
- **Rematch** against MusicBrainz with a side-by-side comparison before anything
  is applied
- **Cover art picker** searching CoverArtArchive, Apple Music, and Deezer, with
  a preview before the image is embedded

### Movies & Series

- Metadata written as **Kodi-style NFO sidecars** (`<movie>`, `<episodedetails>`,
  `<tvshow>`), the format Plex, Jellyfin, and Kodi all read
- Reading falls back gracefully: NFO → embedded container tags → a filename
  parse, so the editor is never blank and a badge tells you which source it used
- **Rematch** against TMDB or TheTVDB, with per-episode matching for series
- **Poster picker** — a separate flow from the metadata rematch, so refreshing
  the text never silently replaces artwork you chose yourself
- Full cast list with roles, crew, studios, certification, and provider ids

### Comics & Books

- **CBZ** — reads and writes `ComicInfo.xml` (the ComicRack schema): series,
  issue, volume, writer/penciller/inker/colorist/letterer/cover artist,
  publisher, imprint, characters, story arc, age rating
- **EPUB** — reads and writes the Dublin Core block of the OPF package document,
  including author vs. translator vs. editor roles and both EPUB 3 and Calibre
  series conventions
- Covers are extracted from inside the archive; page counts come from the
  archive contents
- Archive writes are staged to a temporary file and swapped in only once
  complete, so an interrupted save cannot corrupt the book

### Across every library

- **Batch editing** — set, clear, find-and-replace, or number sequentially
  across any selection, with a before/after preview
- **Renaming** from templates, with a dry-run plan that flags collisions and
  missing metadata before a single file moves
- **Duplicate detection** — byte-identical matching, or fuzzy matching on tags
  that finds the same recording across different encodings. Resolving moves
  files to a dated quarantine folder rather than deleting them
- **Session tracking** — files edited during this session and files new since
  the last scan are badged and filterable
- **Themes** — light, dark, or system, with six colour palettes
- **Languages** — English and French, with more easy to add
- **Automatic updates** with release notes and progress

## Supported Formats

| Kind | Formats |
| --- | --- |
| Audio | `MP3` `FLAC` `OGG` `OPUS` `AAC` `M4A` `M4B` `WMA` `APE` `WAV` `AIFF` `DSF` `WV` |
| Video | `MP4` `MKV` `AVI` `MOV` `WMV` `FLV` `WEBM` `M4V` `TS` `M2TS` |
| Comics | `CBZ` `CBT` · `CBR` and `CB7` are read-only |
| Books | `EPUB` |

Images, `.nfo`, and `.cue` files are treated as sidecars — read where relevant,
but never listed or counted as library entries.

> `CBR` is RAR-based and needs a decoder Notata does not ship. Those files fall
> back to a filename parse, and the editor tells you why saving is unavailable.

## Metadata Sources

| Provider | Used for | Key required |
| --- | --- | --- |
| [MusicBrainz](https://musicbrainz.org/) | Music releases, recordings, artists | No |
| [Cover Art Archive](https://coverartarchive.org/) | Album art | No |
| [Apple Music](https://itunes.apple.com/) | Album art | No |
| [Deezer](https://developers.deezer.com/) | Album art | No |
| [TMDB](https://www.themoviedb.org/) | Movies, series, posters, cast | Yes |
| [TheTVDB](https://thetvdb.com/) | Series and episode data, artwork | Yes |

Keys are stored locally in Notata's own database and are only ever sent to the
provider they belong to.

## Renaming

Renaming always runs as a plan first. Nothing moves until you have seen the
resulting paths, and entries that would collide with each other — or that have
no metadata to rename from — are flagged and skipped rather than producing
files called `() ().mkv`.

### Template syntax

| Syntax | Meaning |
| --- | --- |
| `{field}` | Substitute a value |
| `{track:02}` | Zero-pad a number to a given width |
| `[ ... ]` | Optional group — dropped entirely if a field inside is empty |
| `/` | Path separator, creating folders |
| `%{` | Escape, for a literal brace |

Values are sanitised as they are substituted, so a track by `AC/DC` cannot
invent a directory level the template never asked for.

Available fields depend on the media type — music exposes `albumartist`,
`album`, `track`, `disc`, `isrc` and friends; video exposes `seriestitle`,
`season`, `episode`, `imdbid`, `certification`; comics and books expose
`series`, `number`, `author`, `publisher`, `isbn`.

### Presets

| Preset | Result |
| --- | --- |
| Plex — Music | `Artist/Album (Year)/01 - Title` |
| Jellyfin — Music | `Artist/Album (Year)/01 - Title` |
| Navidrome — Music | `Artist/Album/01 - Title` |
| Plex — Movies | `Movie (Year)/Movie (Year).ext` |
| Jellyfin — Movies | `Movie (Year)/Movie (Year) [imdbid-tt123].ext` |
| Plex — TV Shows | `Show (Year)/Season 01/Show - s01e01 - Title.ext` |
| Jellyfin — TV Shows | `Show (Year)/Season 01/Show S01E01 Title.ext` |
| Comics — Series/Issue | `Series/Series #012 (2011).cbz` |
| Books — Author/Title | `Author/Series/Title (Year).epub` |

## Getting Started

Add a folder with **Add library**, choose what kind of media it holds, and
Notata indexes it. Select a file to edit its metadata; select several to batch
edit or rename them.

### API keys

Music works out of the box. Movies and series need your own free API key from
[TMDB](https://www.themoviedb.org/settings/api) or
[TheTVDB](https://thetvdb.com/api-information) — add it under
**Settings → Providers**.

### Building from source

Requires [Rust](https://rustup.rs/) and [bun](https://bun.sh/).

```bash
bun install
bun run tauri dev      # run in development
bun run tauri build    # produce a release bundle
```

Running the test suite:

```bash
cd src-tauri && cargo test
```

## Project Status

Notata is under active development. Back up anything irreplaceable before running
batch operations against it, and use the rename preview — it exists for a
reason.

Bug reports that include the file which caused them are especially welcome.

## Technologies

<div style="display: flex; align-items: center; gap: 10px;">
  <img src="https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust"/>
  <img src="https://img.shields.io/badge/TAURI-black?style=for-the-badge&logo=tauri"/>
  <img src="https://img.shields.io/badge/React-black?style=for-the-badge&logo=React"/>
  <img src="https://img.shields.io/badge/typeScript-black?style=for-the-badge&logo=typescript"/>
  <img src="https://img.shields.io/badge/vite-black?style=for-the-badge&logo=vite"/>
  <img src="https://img.shields.io/badge/tailwindcss-black?style=for-the-badge&logo=tailwindcss"/>
  <img src="https://img.shields.io/badge/bun-black?style=for-the-badge&logo=bun"/>
</div>

Built on [lofty](https://github.com/Serial-ATA/lofty-rs) for audio tags,
[quick-xml](https://github.com/tafia/quick-xml) for NFO and OPF documents,
[zip](https://github.com/zip-rs/zip2) for comic and ebook archives,
[rusqlite](https://github.com/rusqlite/rusqlite) for the local index, and
[shadcn/ui](https://ui.shadcn.com/) for the interface.

## Authors & contributors

The original setup of this repository is by
[Arnaud BEUX](https://github.com/Nytuo).

For a full list of all authors and contributors, see
[the contributors page](https://github.com/Nytuo/Notata/contributors).

## License

Notata is licensed under the **GNU General Public License v3**.
Notata is provided **"as is"** without any **warranty**. Use at your own risk.
See [LICENSE](LICENSE) for more information.
