import { useState, useCallback, useEffect, useRef } from "react";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { commands } from "@/lib/tauri";
import { useVideoMetadataStore } from "@/stores/videoMetadataStore";
import { toast } from "sonner";
import type {
  RemoteArtwork,
  VideoProviderInfo,
  VideoResultType,
  VideoSearchResult,
} from "@/lib/types";

interface VideoPosterPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

interface Thumb {
  data: string;
  mimeType: string;
}

export function VideoPosterPicker({ open, onOpenChange }: VideoPosterPickerProps) {
  const { currentPath, currentMetadata, artwork, setArtwork } =
    useVideoMetadataStore();

  const [providers, setProviders] = useState<VideoProviderInfo[]>([]);
  const [provider, setProvider] = useState("tmdb");
  const [kind, setKind] = useState<VideoResultType>("movie");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<VideoSearchResult[]>([]);
  const [candidates, setCandidates] = useState<RemoteArtwork[]>([]);
  const [thumbs, setThumbs] = useState<Record<string, Thumb>>({});
  const [selected, setSelected] = useState<RemoteArtwork | null>(null);
  const [preview, setPreview] = useState<Thumb | null>(null);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoadingArt, setIsLoadingArt] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const autoSearched = useRef(false);

  const loadArtwork = useCallback(
    async (providerId: string, id: string, isSeries: boolean) => {
      setIsLoadingArt(true);
      setCandidates([]);
      setThumbs({});
      setSelected(null);
      setPreview(null);
      try {
        const art = await commands.getProviderArtwork(providerId, id, isSeries);
        const posters = art.filter((a) => a.artType === "poster");
        setCandidates(posters.length > 0 ? posters : art);

        // Thumbnails come through the backend so the webview never has to
        // fetch a remote image directly.
        for (const item of (posters.length > 0 ? posters : art).slice(0, 24)) {
          try {
            const image = await commands.downloadCoverArt(item.thumbUrl);
            setThumbs((prev) => ({
              ...prev,
              [item.url]: { data: image.data, mimeType: image.mimeType },
            }));
          } catch {
            // A single failed thumbnail should not stop the rest.
          }
        }
      } catch (e) {
        toast.error(String(e));
      } finally {
        setIsLoadingArt(false);
      }
    },
    [],
  );

  const runSearch = useCallback(
    async (text: string, searchKind: VideoResultType, providerId: string) => {
      if (!text.trim()) return;
      setIsSearching(true);
      setHasSearched(true);
      setResults([]);
      try {
        const found =
          searchKind === "movie"
            ? await commands.searchMovies(providerId, text)
            : await commands.searchSeries(providerId, text);
        setResults(found);

        // Jump straight to the artwork when there is an obvious match.
        if (found.length > 0) {
          await loadArtwork(providerId, found[0].id, searchKind === "series");
        }
      } catch (e) {
        toast.error(String(e));
      } finally {
        setIsSearching(false);
      }
    },
    [loadArtwork],
  );

  useEffect(() => {
    if (!open) {
      autoSearched.current = false;
      setResults([]);
      setCandidates([]);
      setThumbs({});
      setSelected(null);
      setPreview(null);
      setHasSearched(false);
      return;
    }

    commands.listVideoProviders().then(setProviders).catch(() => {});

    if (currentMetadata && !autoSearched.current) {
      const isEpisode = currentMetadata.kind === "episode";
      const seed =
        (isEpisode ? currentMetadata.showTitle : currentMetadata.title) ?? "";
      const nextKind: VideoResultType = isEpisode ? "series" : "movie";
      setKind(nextKind);
      setQuery(seed);

      // An existing provider id skips the search entirely.
      const knownId =
        provider === "tvdb" ? currentMetadata.tvdbId : currentMetadata.tmdbId;

      autoSearched.current = true;
      if (knownId) {
        loadArtwork(provider, knownId, isEpisode);
        setHasSearched(true);
      } else if (seed) {
        runSearch(seed, nextKind, provider);
      }
    }
  }, [open, currentMetadata, provider, loadArtwork, runSearch]);

  const handleSelect = async (art: RemoteArtwork) => {
    setSelected(art);
    const cached = thumbs[art.url];
    if (cached) {
      setPreview(cached);
      return;
    }
    try {
      const image = await commands.downloadCoverArt(art.url);
      setPreview({ data: image.data, mimeType: image.mimeType });
    } catch {
      toast.error("Could not load that image");
    }
  };

  const handleApply = async () => {
    if (!selected || !currentPath) return;
    setIsApplying(true);
    try {
      // Always fetch the full-size version, not the grid thumbnail.
      const image = await commands.downloadCoverArt(selected.url);
      const raw = atob(image.data);
      const bytes = new Uint8Array(raw.length);
      for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);

      const savedTo = await commands.saveVideoPoster(
        currentPath,
        Array.from(bytes),
        image.mimeType,
      );

      setArtwork([
        ...artwork.filter((a) => a.artType !== "poster"),
        {
          artType: "poster",
          path: savedTo,
          data: image.data,
          mimeType: image.mimeType,
        },
      ]);

      toast.success(`Poster saved as ${savedTo.split("/").pop()}`);
      onOpenChange(false);
    } catch (e) {
      toast.error(`Could not save the poster: ${e}`);
    } finally {
      setIsApplying(false);
    }
  };

  const current = providers.find((p) => p.id === provider);
  const needsKey = current && !current.configured;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="shrink-0 border-b px-6 py-4">
          <DialogTitle className="flex items-center gap-2">
            <ImageIcon className="h-5 w-5" />
            Poster
          </DialogTitle>
        </DialogHeader>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            runSearch(query, kind, provider);
          }}
          className="flex shrink-0 gap-2 border-b p-4"
        >
          <Tabs value={kind} onValueChange={(v) => setKind(v as VideoResultType)}>
            <TabsList className="h-8">
              <TabsTrigger value="movie" className="text-xs">
                Movie
              </TabsTrigger>
              <TabsTrigger value="series" className="text-xs">
                Series
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <Select value={provider} onValueChange={setProvider}>
            <SelectTrigger className="h-8 w-[120px] text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {providers.map((p) => (
                <SelectItem key={p.id} value={p.id} className="text-xs">
                  {p.displayName}
                  {!p.configured && " (no key)"}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Input
            className="h-8 flex-1 text-xs"
            placeholder="Title…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Button
            type="submit"
            size="sm"
            className="h-8"
            disabled={isSearching || !query.trim()}
          >
            {isSearching ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <Search className="h-3 w-3" />
            )}
          </Button>
        </form>

        {needsKey && (
          <div className="flex shrink-0 items-center gap-2 border-b bg-amber-500/10 px-4 py-2 text-xs text-amber-700 dark:text-amber-400">
            <AlertCircle className="h-3.5 w-3.5" />
            {current?.displayName} needs an API key — add one in Settings.
          </div>
        )}

        <div className="flex flex-1 overflow-hidden">
          {results.length > 1 && (
            <div className="w-[26%] shrink-0 overflow-y-auto border-r p-2">
              <p className="px-1 pb-1 text-[11px] font-medium text-muted-foreground">
                Matches
              </p>
              {results.map((r) => (
                <button
                  key={`${r.provider}-${r.id}`}
                  className="w-full rounded-md p-2 text-left text-xs hover:bg-accent"
                  onClick={() =>
                    loadArtwork(r.provider, r.id, kind === "series")
                  }
                >
                  <span className="block truncate font-medium">{r.title}</span>
                  {r.year && (
                    <span className="text-muted-foreground">{r.year}</span>
                  )}
                </button>
              ))}
            </div>
          )}

          <div className="flex-1 overflow-y-auto p-3">
            {isLoadingArt ? (
              <div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground">
                <Loader2 className="h-6 w-6 animate-spin" />
                <p className="text-sm">Loading artwork…</p>
              </div>
            ) : candidates.length === 0 ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                {hasSearched
                  ? "No artwork found for this title"
                  : "Search for a title to browse its posters"}
              </p>
            ) : (
              <div className="grid grid-cols-3 gap-3 @lg:grid-cols-4">
                {candidates.map((art) => {
                  const thumb = thumbs[art.url];
                  const isSelected = selected?.url === art.url;
                  return (
                    <button
                      key={art.url}
                      className={`group relative overflow-hidden rounded-md border transition-all hover:ring-2 hover:ring-primary ${
                        isSelected ? "ring-2 ring-primary" : ""
                      }`}
                      onClick={() => handleSelect(art)}
                    >
                      <div className="flex aspect-[2/3] items-center justify-center bg-muted">
                        {thumb ? (
                          <img
                            src={`data:${thumb.mimeType};base64,${thumb.data}`}
                            alt=""
                            className="h-full w-full object-cover"
                          />
                        ) : (
                          <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                        )}
                      </div>
                      <div className="absolute bottom-0 left-0 right-0 flex gap-1 bg-gradient-to-t from-black/70 to-transparent p-1 pt-5">
                        {art.language && (
                          <Badge
                            variant="secondary"
                            className="border-0 bg-black/50 text-[9px] text-white"
                          >
                            {art.language}
                          </Badge>
                        )}
                        {art.width && (
                          <Badge
                            variant="secondary"
                            className="border-0 bg-black/50 text-[9px] text-white"
                          >
                            {art.width}×{art.height}
                          </Badge>
                        )}
                      </div>
                      {isSelected && (
                        <div className="absolute inset-0 flex items-center justify-center bg-primary/20">
                          <Check className="h-7 w-7 text-primary drop-shadow" />
                        </div>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          <div className="flex w-[240px] shrink-0 flex-col border-l">
            <div className="shrink-0 border-b p-3">
              <p className="text-xs font-medium text-muted-foreground">Preview</p>
            </div>
            <div className="flex flex-1 flex-col items-center justify-center gap-3 p-4">
              {preview ? (
                <img
                  src={`data:${preview.mimeType};base64,${preview.data}`}
                  alt="Poster preview"
                  className="max-w-full rounded-md border object-contain"
                  style={{ maxHeight: 300 }}
                />
              ) : (
                <div className="flex flex-col items-center gap-2 text-muted-foreground">
                  <ImageIcon className="h-9 w-9" />
                  <p className="text-xs">Select a poster</p>
                </div>
              )}
            </div>
            {preview && (
              <>
                <Separator />
                <div className="shrink-0 p-4">
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
                    Save poster
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
