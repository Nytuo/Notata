import { useState, useEffect } from "react";
import {
  Search,
  Loader2,
  Film,
  Tv,
  Star,
  AlertCircle,
  ArrowRight,
} from "lucide-react";
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
  EpisodeMetadata,
  SeriesMetadata,
  VideoMetadata,
  VideoProviderInfo,
  VideoResultType,
  VideoSearchResult,
} from "@/lib/types";

interface VideoRematchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Fields shown in the before/after comparison. */
const COMPARE_FIELDS: { key: keyof VideoMetadata; label: string }[] = [
  { key: "title", label: "Title" },
  { key: "showTitle", label: "Show" },
  { key: "season", label: "Season" },
  { key: "episode", label: "Episode" },
  { key: "year", label: "Year" },
  { key: "releaseDate", label: "Released" },
  { key: "runtimeMinutes", label: "Runtime" },
  { key: "rating", label: "Rating" },
  { key: "tagline", label: "Tagline" },
  { key: "plot", label: "Plot" },
  { key: "genres", label: "Genres" },
  { key: "directors", label: "Directors" },
  { key: "writers", label: "Writers" },
  { key: "studios", label: "Studios" },
  { key: "imdbId", label: "IMDb ID" },
  { key: "tmdbId", label: "TMDB ID" },
];

function display(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (Array.isArray(value)) {
    return value
      .map((v) =>
        typeof v === "object" && v !== null && "name" in v
          ? String((v as { name: string }).name)
          : String(v),
      )
      .join(", ");
  }
  return String(value);
}

export function VideoRematchDialog({
  open,
  onOpenChange,
}: VideoRematchDialogProps) {
  const { currentMetadata, applyFromProvider } = useVideoMetadataStore();

  const [providers, setProviders] = useState<VideoProviderInfo[]>([]);
  const [provider, setProvider] = useState("tmdb");
  const [kind, setKind] = useState<VideoResultType>("movie");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<VideoSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selected, setSelected] = useState<VideoSearchResult | null>(null);
  const [episodes, setEpisodes] = useState<EpisodeMetadata[]>([]);
  const [series, setSeries] = useState<SeriesMetadata | null>(null);
  const [selectedEpisode, setSelectedEpisode] = useState<EpisodeMetadata | null>(
    null,
  );
  const [merged, setMerged] = useState<VideoMetadata | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  useEffect(() => {
    if (!open) {
      setResults([]);
      setSelected(null);
      setEpisodes([]);
      setSeries(null);
      setSelectedEpisode(null);
      setMerged(null);
      return;
    }

    commands.listVideoProviders().then(setProviders).catch(() => {});

    if (currentMetadata) {
      // Seed the search from what we already know about the file.
      const isEpisode = currentMetadata.kind === "episode";
      setKind(isEpisode ? "series" : "movie");
      setQuery(
        (isEpisode ? currentMetadata.showTitle : currentMetadata.title) ?? "",
      );
    }
  }, [open, currentMetadata]);

  const current = providers.find((p) => p.id === provider);
  const needsKey = current && !current.configured;

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    setIsSearching(true);
    setResults([]);
    setSelected(null);
    setEpisodes([]);
    setSeries(null);
    setSelectedEpisode(null);
    setMerged(null);
    try {
      const res =
        kind === "movie"
          ? await commands.searchMovies(provider, query)
          : await commands.searchSeries(provider, query);
      setResults(res);
      if (res.length === 0) toast.info("No results");
    } catch (err) {
      toast.error(String(err));
    } finally {
      setIsSearching(false);
    }
  };

  const handleSelect = async (result: VideoSearchResult) => {
    if (!currentMetadata) return;
    setSelected(result);
    setIsLoadingDetails(true);
    setMerged(null);
    setSelectedEpisode(null);
    try {
      if (result.resultType === "movie") {
        const movie = await commands.getMovieDetails(result.provider, result.id);
        setMerged(await commands.applyMovieToMetadata(currentMetadata, movie));
      } else {
        // For a series we still need the specific episode before merging.
        const [details, eps] = await Promise.all([
          commands.getSeriesDetails(result.provider, result.id),
          commands.getSeriesEpisodes(
            result.provider,
            result.id,
            currentMetadata.season ?? undefined,
          ),
        ]);
        setSeries(details);
        setEpisodes(eps);

        const match = eps.find(
          (ep) =>
            ep.season === currentMetadata.season &&
            ep.episode === currentMetadata.episode,
        );
        if (match) await pickEpisode(details, match);
      }
    } catch (err) {
      toast.error(String(err));
    } finally {
      setIsLoadingDetails(false);
    }
  };

  const pickEpisode = async (
    seriesData: SeriesMetadata,
    episode: EpisodeMetadata,
  ) => {
    if (!currentMetadata) return;
    setSelectedEpisode(episode);
    setMerged(
      await commands.applyEpisodeToMetadata(currentMetadata, seriesData, episode),
    );
  };

  const handleApply = () => {
    if (!merged) return;
    setIsApplying(true);
    try {
      // Text only — the poster has its own picker, so a rematch never
      // silently replaces artwork the user chose deliberately.
      applyFromProvider(merged);
      toast.success("Metadata applied — review, then Save NFO");
      onOpenChange(false);
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="shrink-0 border-b px-6 py-4">
          <DialogTitle className="flex items-center gap-2">
            {kind === "movie" ? (
              <Film className="h-5 w-5" />
            ) : (
              <Tv className="h-5 w-5" />
            )}
            Rematch Video Metadata
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSearch} className="flex shrink-0 gap-2 border-b p-4">
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
            placeholder="Title..."
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
          <div className="w-[32%] shrink-0 overflow-y-auto border-r p-2">
            {results.length === 0 ? (
              <p className="py-8 text-center text-xs text-muted-foreground">
                Search to find a match
              </p>
            ) : (
              <div className="space-y-1">
                {results.map((r) => (
                  <div
                    key={`${r.provider}-${r.id}`}
                    className={`cursor-pointer rounded-md border p-2 transition-colors hover:bg-accent ${
                      selected?.id === r.id ? "border-primary bg-accent" : ""
                    }`}
                    onClick={() => handleSelect(r)}
                  >
                    <div className="flex gap-2">
                      {r.posterUrl && (
                        <img
                          src={r.posterUrl}
                          alt=""
                          className="h-14 w-10 shrink-0 rounded object-cover"
                          loading="lazy"
                        />
                      )}
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-xs font-medium">{r.title}</p>
                        <div className="mt-0.5 flex gap-1">
                          {r.year && (
                            <Badge variant="outline" className="text-[10px]">
                              {r.year}
                            </Badge>
                          )}
                          {r.rating != null && r.rating > 0 && (
                            <Badge
                              variant="secondary"
                              className="gap-0.5 text-[10px]"
                            >
                              <Star className="h-2.5 w-2.5" />
                              {r.rating.toFixed(1)}
                            </Badge>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {episodes.length > 0 && (
            <div className="w-[24%] shrink-0 overflow-y-auto border-r">
              <div className="border-b p-2">
                <p className="text-xs font-medium">Episodes</p>
              </div>
              {episodes.map((ep, i) => (
                <div
                  key={i}
                  className={`flex cursor-pointer items-center gap-1 border-b px-2 py-1.5 text-xs hover:bg-accent ${
                    selectedEpisode === ep ? "bg-accent font-medium" : ""
                  }`}
                  onClick={() => series && pickEpisode(series, ep)}
                >
                  <span className="shrink-0 tabular-nums text-muted-foreground">
                    S{String(ep.season).padStart(2, "0")}E
                    {String(ep.episode).padStart(2, "0")}
                  </span>
                  <span className="flex-1 truncate">{ep.title}</span>
                </div>
              ))}
            </div>
          )}

          <div className="flex-1 overflow-y-auto p-4">
            {isLoadingDetails ? (
              <div className="flex h-full items-center justify-center">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : merged && currentMetadata ? (
              <div>
                <div className="grid grid-cols-[110px_1fr_1fr] gap-2 border-b pb-2 text-xs font-medium text-muted-foreground">
                  <div>Field</div>
                  <div>Current</div>
                  <div>New</div>
                </div>
                {COMPARE_FIELDS.map((field) => {
                  const before = display(currentMetadata[field.key]);
                  const after = display(merged[field.key]);
                  if (!before && !after) return null;
                  const changed = before !== after;
                  return (
                    <div
                      key={field.key}
                      className={`grid grid-cols-[110px_1fr_1fr] gap-2 border-b py-1.5 text-xs ${
                        changed ? "bg-green-500/5" : ""
                      }`}
                    >
                      <div className="text-muted-foreground">{field.label}</div>
                      <div
                        className={`line-clamp-3 ${changed ? "text-muted-foreground line-through" : ""}`}
                      >
                        {before || <span className="italic opacity-60">empty</span>}
                      </div>
                      <div
                        className={`line-clamp-3 ${changed ? "font-medium text-green-600 dark:text-green-400" : ""}`}
                      >
                        {after || <span className="italic opacity-60">empty</span>}
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : selected && episodes.length > 0 ? (
              <p className="py-8 text-center text-xs text-muted-foreground">
                Pick the matching episode
              </p>
            ) : (
              <p className="py-8 text-center text-xs text-muted-foreground">
                Select a result to compare against the current metadata
              </p>
            )}
          </div>
        </div>

        <Separator />
        <div className="flex shrink-0 items-center justify-between px-6 py-4">
          <span className="text-xs text-muted-foreground">
            {merged && "Applying updates the editor — save to write the NFO"}
          </span>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleApply} disabled={!merged || isApplying}>
              {isApplying ? (
                <Loader2 className="mr-1 h-4 w-4 animate-spin" />
              ) : (
                <ArrowRight className="mr-1 h-4 w-4" />
              )}
              Apply metadata
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
