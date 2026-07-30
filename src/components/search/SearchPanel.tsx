import { useTranslation } from "react-i18next";
import { Search, Loader2, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { useSearchStore } from "@/stores/searchStore";
import { useMetadataStore } from "@/stores/metadataStore";
import type { TrackMetadata } from "@/lib/types";

export function SearchPanel() {
  const { t } = useTranslation("search");
  const {
    query,
    artistFilter,
    results,
    selectedResult,
    releaseDetails,
    isSearching,
    isLoadingDetails,
    setQuery,
    setArtistFilter,
    searchReleases,
    selectResult,
    loadReleaseDetails,
  } = useSearchStore();

  const { currentMetadata, applyFromProvider } = useMetadataStore();

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    searchReleases();
  };

  const handleSelectResult = async (result: typeof results[0]) => {
    selectResult(result);
    await loadReleaseDetails(result.provider, result.id);
  };

  const handleApplyTrack = (track: TrackMetadata) => {
    if (!currentMetadata) return;
    const merged: TrackMetadata = {
      ...currentMetadata,
      title: track.title ?? currentMetadata.title,
      artist: track.artist ?? currentMetadata.artist,
      albumArtist: track.albumArtist ?? currentMetadata.albumArtist,
      album: track.album ?? currentMetadata.album,
      trackNumber: track.trackNumber ?? currentMetadata.trackNumber,
      totalTracks: track.totalTracks ?? currentMetadata.totalTracks,
      discNumber: track.discNumber ?? currentMetadata.discNumber,
      totalDiscs: track.totalDiscs ?? currentMetadata.totalDiscs,
      year: track.year ?? currentMetadata.year,
      date: track.date ?? currentMetadata.date,
      isrc: track.isrc ?? currentMetadata.isrc,
      musicbrainzTrackId: track.musicbrainzTrackId ?? currentMetadata.musicbrainzTrackId,
      musicbrainzReleaseId: track.musicbrainzReleaseId ?? currentMetadata.musicbrainzReleaseId,
      musicbrainzArtistId: track.musicbrainzArtistId ?? currentMetadata.musicbrainzArtistId,
      musicbrainzReleaseGroupId:
        track.musicbrainzReleaseGroupId ?? currentMetadata.musicbrainzReleaseGroupId,
      genre: currentMetadata.genre,
      composer: track.composer ?? currentMetadata.composer,
      comment: currentMetadata.comment,
      lyrics: currentMetadata.lyrics,
      customTags: currentMetadata.customTags,
    };
    applyFromProvider(merged);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b px-4 py-3">
        <form onSubmit={handleSearch} className="space-y-2">
          <div className="flex gap-2">
            <Input
              className="h-8 text-sm"
              placeholder={t("placeholder")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <Button type="submit" size="sm" disabled={isSearching || !query.trim()}>
              {isSearching ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Search className="h-4 w-4" />
              )}
            </Button>
          </div>
          <Input
            className="h-8 text-sm"
            placeholder={t("artist_filter")}
            value={artistFilter}
            onChange={(e) => setArtistFilter(e.target.value)}
          />
        </form>
      </div>

      <ScrollArea className="flex-1">
        {results.length === 0 && !isSearching ? (
          <div className="p-4 text-center text-sm text-muted-foreground">
            {query ? t("results.no_results") : t("placeholder")}
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {results.map((result) => (
              <div
                key={`${result.provider}-${result.id}`}
                className={`cursor-pointer rounded-md border p-2 text-sm hover:bg-accent ${
                  selectedResult?.id === result.id ? "border-primary bg-accent" : ""
                }`}
                onClick={() => handleSelectResult(result)}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium">{result.title}</p>
                    {result.artist && (
                      <p className="truncate text-xs text-muted-foreground">
                        {result.artist}
                      </p>
                    )}
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {result.year && (
                      <Badge variant="outline" className="text-xs">
                        {result.year}
                      </Badge>
                    )}
                    {result.score != null && (
                      <Badge variant="secondary" className="text-xs">
                        {Math.round(result.score)}%
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {releaseDetails && selectedResult && (
          <>
            <Separator />
            <div className="p-3">
              <h3 className="mb-2 text-sm font-medium">
                {releaseDetails.album.title} — {releaseDetails.album.artist}
              </h3>
              <div className="space-y-1">
                {releaseDetails.tracks.map((track, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 rounded px-2 py-1 text-xs hover:bg-accent"
                  >
                    <span className="w-6 text-right tabular-nums text-muted-foreground">
                      {track.trackNumber}
                    </span>
                    <span className="flex-1 truncate">{track.title}</span>
                    <span className="truncate text-muted-foreground">
                      {track.artist}
                    </span>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 shrink-0"
                      onClick={() => handleApplyTrack(track)}
                      disabled={!currentMetadata}
                    >
                      <ArrowRight className="h-3 w-3" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        {isLoadingDetails && (
          <div className="flex items-center justify-center p-4">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        )}
      </ScrollArea>
    </div>
  );
}
