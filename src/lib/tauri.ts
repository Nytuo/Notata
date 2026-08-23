import { invoke } from "@tauri-apps/api/core";
import type {
  Allin1Status,
  ApiKeyStatus,
  BookCover,
  BookMetadata,
  BookProperties,
  AudioProperties,
  BatchEdit,
  BatchPreviewEntry,
  BatchResult,
  Chapter,
  CoverArt,
  CoverArtData,
  DirectoryNode,
  DuplicateMode,
  DuplicateReport,
  EpisodeMetadata,
  LibraryRoot,
  LibraryStats,
  MediaFile,
  MediaKind,
  MovieMetadata,
  ProviderInfo,
  ProviderRelease,
  ProviderSearchResult,
  RenameOutcome,
  RenamePlan,
  RenamePlanEntry,
  RemoteArtwork,
  RenamePreset,
  ResolveOutcome,
  ScanResult,
  SeriesMetadata,
  FfmpegStatus,
  TrackMetadata,
  TranscodeFormatInfo,
  TranscodeOptions,
  TranscodePreviewEntry,
  TranscodeResult,
  VideoArtwork,
  VideoMetadata,
  VideoProperties,
  VideoProviderInfo,
  VideoSearchResult,
} from "./types";

export const commands = {
  scanDirectory: (path: string, mediaKind?: MediaKind) =>
    invoke<ScanResult>("scan_directory", { path, mediaKind: mediaKind ?? null }),

  setRootMediaKind: (rootId: string, mediaKind: MediaKind) =>
    invoke<void>("set_root_media_kind", { rootId, mediaKind }),

  getLibraryRoots: () => invoke<LibraryRoot[]>("get_library_roots"),

  getFilesInDirectory: (path: string) =>
    invoke<MediaFile[]>("get_files_in_directory", { path }),

  getFilesByRoot: (rootId: string) =>
    invoke<MediaFile[]>("get_files_by_root", { rootId }),

  getDirectoryTree: (root: string) =>
    invoke<DirectoryNode>("get_directory_tree", { root }),

  getLibraryStats: () => invoke<LibraryStats>("get_library_stats"),

  removeLibraryRoot: (rootId: string) =>
    invoke<void>("remove_library_root", { rootId }),

  readMetadata: (path: string) =>
    invoke<TrackMetadata>("read_metadata", { path }),

  readMetadataBatch: (paths: string[]) =>
    invoke<[string, TrackMetadata][]>("read_metadata_batch", { paths }),

  writeMetadata: (path: string, metadata: TrackMetadata) =>
    invoke<void>("write_metadata", { path, metadata }),

  writeMetadataBatch: (entries: [string, TrackMetadata][]) =>
    invoke<[string, boolean, string][]>("write_metadata_batch", { entries }),

  getAudioProperties: (path: string) =>
    invoke<AudioProperties>("get_audio_properties", { path }),

  getEmbeddedCoverArt: (path: string) =>
    invoke<CoverArtData | null>("get_embedded_cover_art", { path }),

  searchReleases: (provider: string, query: string, artist?: string) =>
    invoke<ProviderSearchResult[]>("search_releases", {
      provider,
      query,
      artist: artist ?? null,
    }),

  searchRecordings: (provider: string, query: string, artist?: string) =>
    invoke<ProviderSearchResult[]>("search_recordings", {
      provider,
      query,
      artist: artist ?? null,
    }),

  searchArtists: (provider: string, query: string) =>
    invoke<ProviderSearchResult[]>("search_artists", { provider, query }),

  getReleaseDetails: (provider: string, releaseId: string) =>
    invoke<ProviderRelease>("get_release_details", { provider, releaseId }),

  listProviders: () => invoke<ProviderInfo[]>("list_providers"),

  fetchProviderCoverArt: (provider: string, releaseId: string) =>
    invoke<CoverArt[]>("fetch_provider_cover_art", { provider, releaseId }),

  downloadCoverArt: (url: string) =>
    invoke<CoverArtData>("download_cover_art", { url }),

  embedCoverArt: (path: string, imageData: number[], mimeType: string) =>
    invoke<void>("embed_cover_art", { path, imageData, mimeType }),

  removeCoverArt: (path: string) =>
    invoke<void>("remove_cover_art", { path }),

  searchCoverArt: (query: string) =>
    invoke<CoverArt[]>("search_cover_art", { query }),

  // -------------------------------------------------------------- video ----

  listVideoProviders: () => invoke<VideoProviderInfo[]>("list_video_providers"),

  searchMovies: (provider: string, query: string, year?: number) =>
    invoke<VideoSearchResult[]>("search_movies", {
      provider,
      query,
      year: year ?? null,
    }),

  searchSeries: (provider: string, query: string, year?: number) =>
    invoke<VideoSearchResult[]>("search_series", {
      provider,
      query,
      year: year ?? null,
    }),

  getMovieDetails: (provider: string, movieId: string) =>
    invoke<MovieMetadata>("get_movie_details", { provider, movieId }),

  getSeriesDetails: (provider: string, seriesId: string) =>
    invoke<SeriesMetadata>("get_series_details", { provider, seriesId }),

  getSeriesEpisodes: (provider: string, seriesId: string, season?: number) =>
    invoke<EpisodeMetadata[]>("get_series_episodes", {
      provider,
      seriesId,
      season: season ?? null,
    }),

  getProviderArtwork: (provider: string, id: string, isSeries: boolean) =>
    invoke<RemoteArtwork[]>("get_provider_artwork", { provider, id, isSeries }),

  readVideoMetadata: (path: string) =>
    invoke<VideoMetadata>("read_video_metadata", { path }),

  getVideoProperties: (path: string) =>
    invoke<VideoProperties>("get_video_properties", { path }),

  getVideoArtwork: (path: string) =>
    invoke<VideoArtwork[]>("get_video_artwork", { path }),

  writeVideoMetadata: (path: string, metadata: VideoMetadata) =>
    invoke<string>("write_video_metadata", { path, metadata }),

  writeVideoMetadataBatch: (entries: [string, VideoMetadata][]) =>
    invoke<[string, boolean, string][]>("write_video_metadata_batch", { entries }),

  saveVideoPoster: (path: string, imageData: number[], mimeType: string) =>
    invoke<string>("save_video_poster", { path, imageData, mimeType }),

  applyMovieToMetadata: (current: VideoMetadata, movie: MovieMetadata) =>
    invoke<VideoMetadata>("apply_movie_to_metadata", { current, movie }),

  applyEpisodeToMetadata: (
    current: VideoMetadata,
    series: SeriesMetadata,
    episode: EpisodeMetadata,
  ) =>
    invoke<VideoMetadata>("apply_episode_to_metadata", { current, series, episode }),

  // -------------------------------------------------------- comics/books ----

  readBookMetadata: (path: string) =>
    invoke<BookMetadata>("read_book_metadata", { path }),

  getBookProperties: (path: string) =>
    invoke<BookProperties>("get_book_properties", { path }),

  getBookCover: (path: string) =>
    invoke<BookCover | null>("get_book_cover", { path }),

  writeBookMetadata: (path: string, metadata: BookMetadata) =>
    invoke<string>("write_book_metadata", { path, metadata }),

  writeBookMetadataBatch: (entries: [string, BookMetadata][]) =>
    invoke<[string, boolean, string][]>("write_book_metadata_batch", { entries }),

  writeBookCover: (path: string, imageData: number[], mimeType: string) =>
    invoke<string>("write_book_cover", { path, imageData, mimeType }),

  // ------------------------------------------------------------------ fs ----

  readFileBytes: (path: string) => invoke<ArrayBuffer>("read_file_bytes", { path }),

  // ----------------------------------------------------------- settings ----

  setApiKey: (provider: string, apiKey: string) =>
    invoke<void>("set_api_key", { provider, apiKey }),

  getApiKeyStatus: () => invoke<ApiKeyStatus>("get_api_key_status"),

  setPreference: (key: string, value: string) =>
    invoke<void>("set_preference", { key, value }),

  getPreference: (key: string) =>
    invoke<string | null>("get_preference", { key }),

  // ------------------------------------------------------------ renamer ----

  listRenamePresets: () => invoke<RenamePreset[]>("list_rename_presets"),

  validateRenameTemplate: (template: string) =>
    invoke<void>("validate_rename_template", { template }),

  previewRename: (paths: string[], template: string, baseDir?: string) =>
    invoke<RenamePlan>("preview_rename", {
      paths,
      template,
      baseDir: baseDir ?? null,
    }),

  applyRename: (entries: RenamePlanEntry[]) =>
    invoke<RenameOutcome[]>("apply_rename", { entries }),

  // -------------------------------------------------------------- dedup ----

  findDuplicates: (mode: DuplicateMode, threshold?: number) =>
    invoke<DuplicateReport>("find_duplicates", {
      mode,
      threshold: threshold ?? null,
    }),

  resolveDuplicates: (paths: string[]) =>
    invoke<ResolveOutcome[]>("resolve_duplicates", { paths }),

  readAudioPreview: (path: string) =>
    invoke<ArrayBuffer>("read_audio_preview", { path }),

  // -------------------------------------------------------------- batch ----

  previewBatchEdit: (paths: string[], edits: BatchEdit[]) =>
    invoke<BatchPreviewEntry[]>("preview_batch_edit", { paths, edits }),

  applyBatchEdit: (paths: string[], edits: BatchEdit[]) =>
    invoke<BatchResult[]>("apply_batch_edit", { paths, edits }),

  // ----------------------------------------------------------- transcode ----

  listTranscodeFormats: () =>
    invoke<TranscodeFormatInfo[]>("list_transcode_formats"),

  checkFfmpegAvailable: () => invoke<FfmpegStatus>("check_ffmpeg_available"),

  previewTranscode: (paths: string[], options: TranscodeOptions) =>
    invoke<TranscodePreviewEntry[]>("preview_transcode", { paths, options }),

  transcodeFiles: (paths: string[], options: TranscodeOptions) =>
    invoke<TranscodeResult[]>("transcode_files", { paths, options }),

  // ------------------------------------------------------------ chapters ----

  readChapters: (path: string) => invoke<Chapter[]>("read_chapters", { path }),

  writeChapters: (path: string, chapters: Chapter[]) =>
    invoke<void>("write_chapters", { path, chapters }),

  detectChaptersDsp: (path: string) =>
    invoke<Chapter[]>("detect_chapters_dsp", { path }),

  checkAllin1Available: () => invoke<Allin1Status>("check_allin1_available"),

  detectChaptersAi: (path: string) =>
    invoke<Chapter[]>("detect_chapters_ai", { path }),
} as const;
