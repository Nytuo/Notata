import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Search, Loader2, Check, ImageIcon, AlertCircle } from "lucide-react";
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
import { toast } from "sonner";
import type { CoverArt, CoverArtData } from "@/lib/types";

interface CoverArtPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const SOURCE_LABELS: Record<string, string> = {
  cover_art_archive: "CoverArtArchive",
  itunes: "Apple Music",
  deezer: "Deezer",
  fanart_tv: "Fanart.tv",
};

function sourceLabel(source: string): string {
  return SOURCE_LABELS[source] ?? source;
}

interface ThumbnailState {
  data: string;
  mimeType: string;
}

export function CoverArtPicker({ open, onOpenChange }: CoverArtPickerProps) {
  const { t } = useTranslation("metadata");
  const { currentPath, currentMetadata, coverArt } = useMetadataStore();

  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CoverArt[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [selected, setSelected] = useState<CoverArt | null>(null);
  const [preview, setPreview] = useState<CoverArtData | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [thumbnails, setThumbnails] = useState<Record<string, ThumbnailState>>({});
  const autoSearched = useRef(false);

  const doSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    setSelected(null);
    setPreview(null);
    setHasSearched(true);
    try {
      const mbReleaseId = currentMetadata?.musicbrainzReleaseId;
      const [external, caa] = await Promise.all([
        commands.searchCoverArt(searchQuery),
        mbReleaseId
          ? commands.fetchProviderCoverArt("musicbrainz", mbReleaseId)
          : Promise.resolve([]),
      ]);
      const allResults = [...caa, ...external];
      setResults(allResults);
      loadThumbnails(allResults);
    } catch {
      toast.error("Failed to search cover art");
    } finally {
      setIsSearching(false);
    }
  }, [currentMetadata?.musicbrainzReleaseId]);

  const loadThumbnails = async (arts: CoverArt[]) => {
    for (const art of arts) {
      if (!art.url) continue;
      try {
        const data = await commands.downloadCoverArt(art.url);
        setThumbnails((prev) => ({
          ...prev,
          [art.id]: { data: data.data, mimeType: data.mimeType },
        }));
      } catch {
        // thumbnail failed to load, skip silently
      }
    }
  };

  useEffect(() => {
    if (open && currentMetadata && !autoSearched.current) {
      const prefill = [currentMetadata.artist, currentMetadata.album]
        .filter(Boolean)
        .join(" ");
      if (prefill) {
        setQuery(prefill);
        autoSearched.current = true;
        doSearch(prefill);
      }
    }
    if (!open) {
      autoSearched.current = false;
      setResults([]);
      setSelected(null);
      setPreview(null);
      setThumbnails({});
      setHasSearched(false);
    }
  }, [open, currentMetadata, doSearch]);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    doSearch(query);
  };

  const handleSelect = useCallback(async (art: CoverArt) => {
    if (!art.url) return;
    setSelected(art);
    const cached = thumbnails[art.id];
    if (cached) {
      setPreview({
        data: cached.data,
        mimeType: cached.mimeType,
        artType: art.artType,
        width: null,
        height: null,
      });
      return;
    }
    setIsLoadingPreview(true);
    try {
      const data = await commands.downloadCoverArt(art.url);
      setPreview(data);
    } catch {
      toast.error("Failed to load preview");
      setPreview(null);
    } finally {
      setIsLoadingPreview(false);
    }
  }, [thumbnails]);

  const handleApply = async () => {
    if (!preview || !currentPath) return;
    setIsApplying(true);
    try {
      const raw = atob(preview.data);
      const bytes = new Uint8Array(raw.length);
      for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
      await commands.embedCoverArt(currentPath, Array.from(bytes), preview.mimeType);
      useMetadataStore.setState({ coverArt: preview });
      toast.success(t("cover_art.embed_success"));
      onOpenChange(false);
    } catch {
      toast.error(t("cover_art.embed_error"));
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="px-6 py-4 border-b shrink-0">
          <DialogTitle className="flex items-center gap-2">
            <ImageIcon className="h-5 w-5" />
            Cover Art
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-1 overflow-hidden">
          {/* Left: search + grid */}
          <div className="flex flex-1 flex-col">
            <form onSubmit={handleSearch} className="flex gap-2 p-4 border-b shrink-0">
              <Input
                placeholder="Search artist, album..."
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <Button type="submit" disabled={isSearching || !query.trim()}>
                {isSearching ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Search className="h-4 w-4" />
                )}
              </Button>
            </form>

            <div className="flex-1 overflow-y-auto p-3">
              {isSearching ? (
                <div className="flex flex-col items-center justify-center py-12 gap-2 text-muted-foreground">
                  <Loader2 className="h-6 w-6 animate-spin" />
                  <p className="text-sm">Searching cover art sources...</p>
                </div>
              ) : results.length === 0 && hasSearched ? (
                <div className="flex flex-col items-center justify-center py-12 gap-2 text-muted-foreground">
                  <AlertCircle className="h-6 w-6" />
                  <p className="text-sm">No cover art found</p>
                  <p className="text-xs">Try a different search query</p>
                </div>
              ) : results.length === 0 ? (
                <p className="text-center text-sm text-muted-foreground py-8">
                  Search for cover art from Apple Music, Deezer, and CoverArtArchive
                </p>
              ) : (
                <div className="grid grid-cols-4 gap-3">
                  {results.map((art) => (
                    <CoverArtCard
                      key={art.id}
                      art={art}
                      thumbnail={thumbnails[art.id]}
                      isSelected={selected?.id === art.id}
                      onClick={() => handleSelect(art)}
                    />
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Right: preview */}
          <div className="w-[300px] shrink-0 border-l flex flex-col">
            <div className="p-3 border-b shrink-0">
              <p className="text-xs font-medium text-muted-foreground">Preview</p>
            </div>

            <div className="flex-1 flex flex-col items-center justify-center p-4 gap-4">
              {isLoadingPreview ? (
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              ) : preview ? (
                <>
                  <img
                    src={`data:${preview.mimeType};base64,${preview.data}`}
                    alt="Cover art preview"
                    className="max-w-full rounded-md border object-contain"
                    style={{ maxHeight: 260 }}
                  />
                  {selected && (
                    <div className="text-center space-y-1">
                      <Badge variant="outline">
                        {sourceLabel(typeof selected.source === "string" ? selected.source : "unknown")}
                      </Badge>
                      {selected.dataPath && (
                        <p className="text-xs text-muted-foreground">{selected.dataPath}</p>
                      )}
                    </div>
                  )}
                </>
              ) : coverArt ? (
                <>
                  <p className="text-xs text-muted-foreground mb-2">Current</p>
                  <img
                    src={`data:${coverArt.mimeType};base64,${coverArt.data}`}
                    alt="Current cover art"
                    className="max-w-full rounded-md border object-contain opacity-60"
                    style={{ maxHeight: 220 }}
                  />
                </>
              ) : (
                <div className="flex flex-col items-center gap-2 text-muted-foreground">
                  <ImageIcon className="h-10 w-10" />
                  <p className="text-xs">Select art to preview</p>
                </div>
              )}
            </div>

            {preview && (
              <>
                <Separator />
                <div className="p-4 shrink-0">
                  <Button
                    className="w-full"
                    onClick={handleApply}
                    disabled={isApplying}
                  >
                    {isApplying ? (
                      <Loader2 className="mr-1 h-4 w-4 animate-spin" />
                    ) : (
                      <Check className="mr-1 h-4 w-4" />
                    )}
                    Apply cover art
                  </Button>
                </div>
              </>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function CoverArtCard({
  art,
  thumbnail,
  isSelected,
  onClick,
}: {
  art: CoverArt;
  thumbnail?: ThumbnailState;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <div
      className={`group relative cursor-pointer overflow-hidden rounded-md border transition-all hover:ring-2 hover:ring-primary ${
        isSelected ? "ring-2 ring-primary" : ""
      }`}
      onClick={onClick}
    >
      <div className="aspect-square bg-muted flex items-center justify-center">
        {thumbnail ? (
          <img
            src={`data:${thumbnail.mimeType};base64,${thumbnail.data}`}
            alt={art.dataPath || "Cover art"}
            className="h-full w-full object-cover"
          />
        ) : (
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
        )}
      </div>
      <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/70 to-transparent p-2 pt-6">
        <Badge
          variant="secondary"
          className="text-[10px] bg-black/50 text-white border-0"
        >
          {sourceLabel(typeof art.source === "string" ? art.source : "unknown")}
        </Badge>
      </div>
      {isSelected && (
        <div className="absolute inset-0 flex items-center justify-center bg-primary/20">
          <Check className="h-8 w-8 text-primary drop-shadow" />
        </div>
      )}
    </div>
  );
}
