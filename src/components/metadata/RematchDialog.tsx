import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search, Loader2, Check, ArrowRight, ChevronRight } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { useMetadataStore } from "@/stores/metadataStore";
import { commands } from "@/lib/tauri";
import type {
  ProviderSearchResult,
  ProviderRelease,
  TrackMetadata,
} from "@/lib/types";

interface RematchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type Step = "search" | "compare";

export function RematchDialog({ open, onOpenChange }: RematchDialogProps) {
  const { t } = useTranslation("search");
  const { currentMetadata, applyFromProvider } = useMetadataStore();

  const [step, setStep] = useState<Step>("search");
  const [query, setQuery] = useState("");
  const [artistFilter, setArtistFilter] = useState("");
  const [results, setResults] = useState<ProviderSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedResult, setSelectedResult] = useState<ProviderSearchResult | null>(null);
  const [releaseDetails, setReleaseDetails] = useState<ProviderRelease | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);
  const [selectedTrack, setSelectedTrack] = useState<TrackMetadata | null>(null);

  const handleOpenChange = (open: boolean) => {
    if (!open) {
      setStep("search");
      setResults([]);
      setSelectedResult(null);
      setReleaseDetails(null);
      setSelectedTrack(null);
    }
    onOpenChange(open);
  };

  const prefillFromMetadata = () => {
    if (!currentMetadata) return;
    setQuery(currentMetadata.album || currentMetadata.title || "");
    setArtistFilter(currentMetadata.artist || currentMetadata.albumArtist || "");
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    setIsSearching(true);
    setResults([]);
    setSelectedResult(null);
    setReleaseDetails(null);
    setSelectedTrack(null);
    try {
      const res = await commands.searchReleases(
        "musicbrainz",
        query,
        artistFilter || undefined,
      );
      setResults(res);
    } finally {
      setIsSearching(false);
    }
  };

  const handleSelectResult = async (result: ProviderSearchResult) => {
    setSelectedResult(result);
    setIsLoadingDetails(true);
    setSelectedTrack(null);
    try {
      const details = await commands.getReleaseDetails(result.provider, result.id);
      setReleaseDetails(details);
      const matched = details.tracks.find(
        (tr) =>
          tr.trackNumber === currentMetadata?.trackNumber ||
          tr.title?.toLowerCase() === currentMetadata?.title?.toLowerCase(),
      );
      if (matched) {
        setSelectedTrack(matched);
      }
    } finally {
      setIsLoadingDetails(false);
    }
  };

  const handleApply = () => {
    if (!selectedTrack || !currentMetadata) return;
    const merged: TrackMetadata = {
      ...currentMetadata,
      title: selectedTrack.title ?? currentMetadata.title,
      artist: selectedTrack.artist ?? currentMetadata.artist,
      albumArtist: selectedTrack.albumArtist ?? currentMetadata.albumArtist,
      album: selectedTrack.album ?? currentMetadata.album,
      trackNumber: selectedTrack.trackNumber ?? currentMetadata.trackNumber,
      totalTracks: selectedTrack.totalTracks ?? currentMetadata.totalTracks,
      discNumber: selectedTrack.discNumber ?? currentMetadata.discNumber,
      totalDiscs: selectedTrack.totalDiscs ?? currentMetadata.totalDiscs,
      year: selectedTrack.year ?? currentMetadata.year,
      date: selectedTrack.date ?? currentMetadata.date,
      isrc: selectedTrack.isrc ?? currentMetadata.isrc,
      musicbrainzTrackId:
        selectedTrack.musicbrainzTrackId ?? currentMetadata.musicbrainzTrackId,
      musicbrainzReleaseId:
        selectedTrack.musicbrainzReleaseId ?? currentMetadata.musicbrainzReleaseId,
      musicbrainzArtistId:
        selectedTrack.musicbrainzArtistId ?? currentMetadata.musicbrainzArtistId,
      musicbrainzReleaseGroupId:
        selectedTrack.musicbrainzReleaseGroupId ?? currentMetadata.musicbrainzReleaseGroupId,
      genre: currentMetadata.genre,
      composer: selectedTrack.composer ?? currentMetadata.composer,
      comment: currentMetadata.comment,
      lyrics: currentMetadata.lyrics,
      customTags: currentMetadata.customTags,
    };
    applyFromProvider(merged);
    handleOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="px-6 py-4 border-b shrink-0">
          <DialogTitle className="flex items-center gap-2">
            <Search className="h-5 w-5" />
            Rematch Metadata
            {step === "compare" && (
              <>
                <ChevronRight className="h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground font-normal">Compare</span>
              </>
            )}
          </DialogTitle>
        </DialogHeader>

        {step === "search" && (
          <div className="flex flex-1 overflow-hidden">
            {/* Left: search form + results */}
            <div className="flex flex-1 flex-col border-r">
              <form onSubmit={handleSearch} className="space-y-2 p-4 border-b shrink-0">
                <div className="flex gap-2">
                  <Input
                    placeholder="Album or track name..."
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    autoFocus
                  />
                  <Button type="submit" disabled={isSearching || !query.trim()}>
                    {isSearching ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Search className="h-4 w-4" />
                    )}
                  </Button>
                </div>
                <div className="flex gap-2">
                  <Input
                    placeholder="Artist filter..."
                    value={artistFilter}
                    onChange={(e) => setArtistFilter(e.target.value)}
                    className="flex-1"
                  />
                  {currentMetadata && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={prefillFromMetadata}
                    >
                      Auto-fill
                    </Button>
                  )}
                </div>
              </form>

              <div className="flex-1 overflow-y-auto p-2">
                {results.length === 0 && !isSearching ? (
                  <p className="p-4 text-center text-sm text-muted-foreground">
                    {query ? t("results.no_results") : "Search MusicBrainz for a release"}
                  </p>
                ) : (
                  <div className="space-y-1">
                    {results.map((r) => (
                      <div
                        key={`${r.provider}-${r.id}`}
                        className={`cursor-pointer rounded-md border p-3 hover:bg-accent transition-colors ${
                          selectedResult?.id === r.id ? "border-primary bg-accent" : ""
                        }`}
                        onClick={() => handleSelectResult(r)}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="min-w-0 flex-1">
                            <p className="font-medium truncate">{r.title}</p>
                            {r.artist && (
                              <p className="text-sm text-muted-foreground truncate">
                                {r.artist}
                              </p>
                            )}
                          </div>
                          <div className="flex shrink-0 gap-1">
                            {r.year && (
                              <Badge variant="outline">{r.year}</Badge>
                            )}
                            {r.score != null && (
                              <Badge variant="secondary">{Math.round(r.score)}%</Badge>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Right: track listing from selected release */}
            <div className="w-[300px] shrink-0 flex flex-col">
              {isLoadingDetails ? (
                <div className="flex flex-1 items-center justify-center">
                  <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                </div>
              ) : releaseDetails ? (
                <>
                  <div className="border-b p-3">
                    <p className="font-medium text-sm truncate">
                      {releaseDetails.album.title}
                    </p>
                    <p className="text-xs text-muted-foreground truncate">
                      {releaseDetails.album.artist}
                      {releaseDetails.album.year && ` (${releaseDetails.album.year})`}
                    </p>
                  </div>
                  <div className="flex-1 overflow-y-auto">
                    {releaseDetails.tracks.map((track, i) => (
                      <div
                        key={i}
                        className={`flex items-center gap-2 px-3 py-2 text-sm cursor-pointer hover:bg-accent border-b ${
                          selectedTrack === track ? "bg-accent font-medium" : ""
                        }`}
                        onClick={() => setSelectedTrack(track)}
                      >
                        <span className="w-5 text-right text-xs tabular-nums text-muted-foreground">
                          {track.trackNumber}
                        </span>
                        <span className="flex-1 truncate">{track.title}</span>
                        {selectedTrack === track && (
                          <Check className="h-3 w-3 text-primary shrink-0" />
                        )}
                      </div>
                    ))}
                  </div>
                  {selectedTrack && (
                    <div className="border-t p-3 shrink-0">
                      <Button
                        className="w-full"
                        size="sm"
                        onClick={() => setStep("compare")}
                      >
                        <ArrowRight className="mr-1 h-4 w-4" />
                        Compare with current
                      </Button>
                    </div>
                  )}
                </>
              ) : (
                <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground p-4 text-center">
                  Select a release to see its tracks
                </div>
              )}
            </div>
          </div>
        )}

        {step === "compare" && selectedTrack && currentMetadata && (
          <div className="flex flex-1 flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto px-6 py-4">
              <ComparisonTable current={currentMetadata} matched={selectedTrack} />
            </div>
            <Separator />
            <div className="flex items-center justify-between px-6 py-4 shrink-0">
              <Button variant="outline" onClick={() => setStep("search")}>
                Back to search
              </Button>
              <div className="flex gap-2">
                <Button variant="ghost" onClick={() => handleOpenChange(false)}>
                  Cancel
                </Button>
                <Button onClick={handleApply}>
                  <Check className="mr-1 h-4 w-4" />
                  Apply metadata
                </Button>
              </div>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

const COMPARE_FIELDS: { key: keyof TrackMetadata; label: string }[] = [
  { key: "title", label: "Title" },
  { key: "artist", label: "Artist" },
  { key: "albumArtist", label: "Album Artist" },
  { key: "album", label: "Album" },
  { key: "trackNumber", label: "Track" },
  { key: "totalTracks", label: "Total Tracks" },
  { key: "discNumber", label: "Disc" },
  { key: "totalDiscs", label: "Total Discs" },
  { key: "year", label: "Year" },
  { key: "date", label: "Date" },
  { key: "isrc", label: "ISRC" },
  { key: "musicbrainzTrackId", label: "MB Track ID" },
  { key: "musicbrainzReleaseId", label: "MB Release ID" },
  { key: "musicbrainzArtistId", label: "MB Artist ID" },
];

function ComparisonTable({
  current,
  matched,
}: {
  current: TrackMetadata;
  matched: TrackMetadata;
}) {
  return (
    <div className="space-y-0">
      <div className="grid grid-cols-[140px_1fr_1fr] gap-2 pb-2 border-b text-xs font-medium text-muted-foreground">
        <div>Field</div>
        <div>Current</div>
        <div>New (MusicBrainz)</div>
      </div>
      {COMPARE_FIELDS.map((field) => {
        const currentVal = formatField(current[field.key]);
        const matchedVal = formatField(matched[field.key]);
        const changed = currentVal !== matchedVal && matchedVal !== "";
        return (
          <div
            key={field.key}
            className={`grid grid-cols-[140px_1fr_1fr] gap-2 py-2 border-b text-sm ${
              changed ? "bg-green-500/5" : ""
            }`}
          >
            <div className="text-xs text-muted-foreground font-medium self-center">
              {field.label}
            </div>
            <div className={`truncate ${changed ? "text-muted-foreground line-through" : ""}`}>
              {currentVal || <span className="text-muted-foreground italic">empty</span>}
            </div>
            <div className={`truncate ${changed ? "font-medium text-green-600 dark:text-green-400" : ""}`}>
              {matchedVal || <span className="text-muted-foreground italic">empty</span>}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function formatField(val: unknown): string {
  if (val === null || val === undefined) return "";
  if (Array.isArray(val)) return val.join("; ");
  return String(val);
}
