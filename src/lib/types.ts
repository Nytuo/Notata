export type MediaType =
  | "audio"
  | "video"
  | "comic"
  | "book"
  | "image"
  | "cue"
  | "nfo"
  | "unknown";

export type AudioFormat =
  | "mp3"
  | "flac"
  | "ogg"
  | "opus"
  | "aac"
  | "mp4a"
  | "wma"
  | "ape"
  | "wav"
  | "aiff"
  | "dsf"
  | "wavpack"
  | { unknown: string };

export interface MediaFile {
  id: string;
  path: string;
  fileName: string;
  parentDir: string;
  mediaType: MediaType;
  audioFormat: AudioFormat | null;
  fileSize: number;
  modifiedAt: number;
  scannedAt: number;
  hasCoverArt: boolean;
  durationMs: number | null;
  bitrateKbps: number | null;
  sampleRateHz: number | null;
  channels: number | null;
  /** When this path was first indexed; survives re-scans. */
  firstSeenAt: number;
  /** Last time Notata wrote tags to this file, if ever. */
  lastModifiedByApp: number | null;
  /** First seen during the most recent scan. */
  isNew: boolean;
}

export interface DirectoryNode {
  path: string;
  name: string;
  children: DirectoryNode[];
  fileCount: number;
}

export interface ScanResult {
  totalFiles: number;
  audioFiles: number;
  skipped: number;
  durationMs: number;
}

export interface ScanProgress {
  scanned: number;
  currentFile: string;
}

/** What kind of media a library root holds. */
export type MediaKind = "music" | "movies" | "series" | "books";

export interface LibraryRoot {
  id: string;
  path: string;
  label: string | null;
  addedAt: number;
  lastScan: number | null;
  previousScan: number | null;
  mediaKind: MediaKind;
}

export interface LibraryStats {
  totalFiles: number;
  totalSize: number;
  roots: LibraryRoot[];
}

export interface TrackMetadata {
  title: string | null;
  artist: string | null;
  albumArtist: string | null;
  album: string | null;
  trackNumber: number | null;
  totalTracks: number | null;
  discNumber: number | null;
  totalDiscs: number | null;
  year: number | null;
  date: string | null;
  genre: string[] | null;
  composer: string[] | null;
  comment: string | null;
  lyrics: string | null;
  isrc: string | null;
  musicbrainzTrackId: string | null;
  musicbrainzReleaseId: string | null;
  musicbrainzArtistId: string | null;
  musicbrainzReleaseGroupId: string | null;
  customTags: Record<string, string[]>;
}

export interface AudioProperties {
  durationMs: number;
  bitrateKbps: number;
  sampleRateHz: number;
  channels: number;
  bitsPerSample: number | null;
  format: string;
}

export type CoverArtType =
  | "front"
  | "back"
  | "disc"
  | "booklet"
  | "artist"
  | { other: string };

export type ArtSource =
  | "embedded"
  | "local_file"
  | "cover_art_archive"
  | "fanart_tv"
  | "manual";

export interface CoverArt {
  id: string;
  artType: CoverArtType;
  source: ArtSource;
  mimeType: string;
  width: number | null;
  height: number | null;
  dataPath: string | null;
  url: string | null;
}

export interface CoverArtData {
  data: string;
  mimeType: string;
  artType: CoverArtType;
  width: number | null;
  height: number | null;
}

export type SearchResultType = "release" | "recording" | "artist" | "release_group";

export interface ProviderSearchResult {
  provider: string;
  resultType: SearchResultType;
  id: string;
  score: number | null;
  title: string;
  artist: string | null;
  year: number | null;
  extra: Record<string, unknown>;
}

export interface ProviderRelease {
  provider: string;
  id: string;
  album: AlbumMetadata;
  tracks: TrackMetadata[];
  artists: ArtistMetadata[];
}

export interface AlbumMetadata {
  title: string;
  artist: string;
  year: number | null;
  releaseDate: string | null;
  genre: string[] | null;
  label: string | null;
  catalogNumber: string | null;
  barcode: string | null;
  releaseType: string | null;
  releaseCountry: string | null;
  musicbrainzReleaseId: string | null;
  musicbrainzReleaseGroupId: string | null;
  totalTracks: number | null;
  totalDiscs: number | null;
  coverArt: CoverArt[];
}

export interface ArtistMetadata {
  name: string;
  sortName: string | null;
  musicbrainzArtistId: string | null;
  disambiguation: string | null;
  artistType: string | null;
  country: string | null;
}

export interface ProviderInfo {
  id: string;
  displayName: string;
  supportedTypes: string[];
}

// ---------------------------------------------------------------- video ----

export type VideoResultType = "movie" | "series";

export interface VideoSearchResult {
  provider: string;
  resultType: VideoResultType;
  id: string;
  title: string;
  year: number | null;
  overview: string | null;
  posterUrl: string | null;
  rating: number | null;
}

export interface MovieMetadata {
  title: string;
  originalTitle: string | null;
  year: number | null;
  releaseDate: string | null;
  tagline: string | null;
  overview: string | null;
  runtimeMinutes: number | null;
  genres: string[];
  rating: number | null;
  country: string | null;
  studios: string[];
  directors: string[];
  writers: string[];
  cast: string[];
  tmdbId: string | null;
  imdbId: string | null;
  posterUrl: string | null;
  backdropUrl: string | null;
}

export interface EpisodeMetadata {
  title: string;
  season: number;
  episode: number;
  airDate: string | null;
  overview: string | null;
  rating: number | null;
  runtimeMinutes: number | null;
  stillUrl: string | null;
  tmdbId: string | null;
  tvdbId: string | null;
}

export interface SeriesMetadata {
  title: string;
  year: number | null;
  firstAired: string | null;
  overview: string | null;
  genres: string[];
  rating: number | null;
  network: string | null;
  status: string | null;
  totalSeasons: number | null;
  tmdbId: string | null;
  tvdbId: string | null;
  imdbId: string | null;
  posterUrl: string | null;
  backdropUrl: string | null;
  episodes: EpisodeMetadata[];
}

export interface VideoProviderInfo {
  id: string;
  displayName: string;
  requiresApiKey: boolean;
  configured: boolean;
}

/** A candidate image offered by a provider during poster rematch. */
export interface RemoteArtwork {
  provider: string;
  artType: string;
  url: string;
  thumbUrl: string;
  width: number | null;
  height: number | null;
  language: string | null;
  rating: number | null;
}

export type VideoKind = "movie" | "episode";

export type VideoMetadataSource = "nfo" | "embedded" | "filename" | "none";

export interface ActorCredit {
  name: string;
  role: string | null;
  thumb: string | null;
}

/** The editable metadata for one video file, mirroring NFO fields. */
export interface VideoMetadata {
  kind: VideoKind;
  title: string | null;
  originalTitle: string | null;
  sortTitle: string | null;
  year: number | null;
  releaseDate: string | null;
  tagline: string | null;
  plot: string | null;
  outline: string | null;
  runtimeMinutes: number | null;
  rating: number | null;
  votes: number | null;
  certification: string | null;
  genres: string[];
  studios: string[];
  countries: string[];
  directors: string[];
  writers: string[];
  actors: ActorCredit[];
  tags: string[];
  showTitle: string | null;
  season: number | null;
  episode: number | null;
  aired: string | null;
  imdbId: string | null;
  tmdbId: string | null;
  tvdbId: string | null;
  trailer: string | null;
  source: VideoMetadataSource;
  nfoPath: string | null;
}

export interface VideoProperties {
  durationMs: number | null;
  container: string;
  fileSize: number;
  overallBitrateKbps: number | null;
}

export interface VideoArtwork {
  artType: string;
  path: string;
  data: string;
  mimeType: string;
}

export interface ApiKeyStatus {
  tmdbConfigured: boolean;
  tvdbConfigured: boolean;
}

// -------------------------------------------------------------- renamer ----

export type PresetKind = "music" | "movie" | "series";

export interface RenamePreset {
  id: string;
  label: string;
  server: string;
  kind: PresetKind;
  template: string;
  description: string;
}

export interface RenamePlanEntry {
  sourcePath: string;
  targetPath: string;
  relativeTarget: string;
  changed: boolean;
  conflict: string | null;
}

export interface RenamePlan {
  entries: RenamePlanEntry[];
  total: number;
  changed: number;
  conflicts: number;
}

export interface RenameOutcome {
  sourcePath: string;
  targetPath: string;
  success: boolean;
  error: string | null;
}

// ---------------------------------------------------------------- dedup ----

export type DuplicateMode = "exact" | "fuzzy";

export interface DuplicateFile {
  path: string;
  fileName: string;
  fileSize: number;
  durationMs: number | null;
  bitrateKbps: number | null;
  format: string | null;
  title: string | null;
  artist: string | null;
  album: string | null;
  score: number;
  recommendedKeep: boolean;
}

export interface DuplicateGroup {
  id: string;
  mode: DuplicateMode;
  reason: string;
  files: DuplicateFile[];
  wastedBytes: number;
}

export interface DuplicateReport {
  groups: DuplicateGroup[];
  totalGroups: number;
  totalFiles: number;
  reclaimableBytes: number;
}

export interface ResolveOutcome {
  path: string;
  success: boolean;
  movedTo: string | null;
  error: string | null;
}

// ---------------------------------------------------------------- batch ----

export type FieldOp =
  | { kind: "set"; value: string }
  | { kind: "clear" }
  | { kind: "replace"; find: string; replace: string }
  | { kind: "enumerate"; start: number };

export interface BatchEdit {
  field: string;
  op: FieldOp;
}

export interface BatchPreviewEntry {
  path: string;
  field: string;
  before: string;
  after: string;
  changed: boolean;
}

export interface BatchResult {
  path: string;
  success: boolean;
  error: string | null;
}

// --------------------------------------------------------- comics/books ----

export type BookKind = "comic" | "ebook";

export type BookMetadataSource = "comic_info" | "opf" | "filename" | "none";

/** Editable metadata for a comic issue or an ebook. */
export interface BookMetadata {
  kind: BookKind;
  title: string | null;
  series: string | null;
  number: string | null;
  count: number | null;
  volume: number | null;
  summary: string | null;
  year: number | null;
  month: number | null;
  day: number | null;
  authors: string[];
  pencillers: string[];
  inkers: string[];
  colorists: string[];
  letterers: string[];
  coverArtists: string[];
  editors: string[];
  translators: string[];
  publisher: string | null;
  imprint: string | null;
  genres: string[];
  characters: string[];
  storyArc: string | null;
  language: string | null;
  isbn: string | null;
  pageCount: number | null;
  ageRating: string | null;
  web: string | null;
  rights: string | null;
  source: BookMetadataSource;
  entryPath: string | null;
}

export interface BookProperties {
  container: string;
  fileSize: number;
  pageCount: number | null;
  readable: boolean;
}

export interface BookCover {
  data: string;
  mimeType: string;
  entryPath: string;
}
