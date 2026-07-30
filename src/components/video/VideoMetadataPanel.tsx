import { useState } from "react";
import {
  Save,
  Undo2,
  Loader2,
  Search,
  Film,
  Tv,
  FileText,
  ImageIcon,
  Plus,
  X,
  Pencil,
  Replace,
} from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useVideoMetadataStore } from "@/stores/videoMetadataStore";
import { VideoRematchDialog } from "@/components/video/VideoRematchDialog";
import { VideoPosterPicker } from "@/components/video/VideoPosterPicker";
import { toast } from "sonner";
import type { VideoMetadata } from "@/lib/types";

const SOURCE_LABELS: Record<string, { label: string; hint: string }> = {
  nfo: { label: "NFO", hint: "Loaded from an NFO sidecar" },
  embedded: { label: "Embedded", hint: "Loaded from tags inside the container" },
  filename: {
    label: "Filename",
    hint: "No metadata found — values guessed from the filename",
  },
  none: { label: "None", hint: "No metadata found" },
};

function formatDuration(ms: number | null): string {
  if (!ms) return "—";
  const totalMin = Math.floor(ms / 60000);
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

/** Comma/semicolon separated list editor for the string[] fields. */
function ListField({
  label,
  values,
  onChange,
  placeholder,
}: {
  label: string;
  values: string[];
  onChange: (next: string[]) => void;
  placeholder?: string;
}) {
  return (
    <div>
      <label className="mb-1 block text-xs text-muted-foreground">{label}</label>
      <Input
        className="h-8 text-sm"
        value={values.join("; ")}
        placeholder={placeholder}
        onChange={(e) =>
          onChange(
            e.target.value
              .split(";")
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
      />
    </div>
  );
}

export function VideoMetadataPanel() {
  const [rematchOpen, setRematchOpen] = useState(false);
  const [posterPickerOpen, setPosterPickerOpen] = useState(false);
  const {
    currentPath,
    currentMetadata,
    properties,
    artwork,
    isDirty,
    isLoading,
    isSaving,
    updateField,
    saveMetadata,
    revertChanges,
  } = useVideoMetadataStore();

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!currentMetadata || !currentPath) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        Select a video file to view metadata
      </div>
    );
  }

  const handleSave = async () => {
    try {
      const nfoPath = await saveMetadata();
      toast.success(`Saved to ${nfoPath.split("/").pop()}`);
    } catch (e) {
      toast.error(`Could not save: ${e}`);
    }
  };

  const isEpisode = currentMetadata.kind === "episode";
  const poster = artwork.find((a) => a.artType === "poster");
  const source = SOURCE_LABELS[currentMetadata.source] ?? SOURCE_LABELS.none;

  const textField = (
    key: keyof VideoMetadata,
    label: string,
    opts: { type?: "number" | "text"; half?: boolean } = {},
  ) => {
    const raw = currentMetadata[key];
    const value = raw === null || raw === undefined ? "" : String(raw);
    return (
      <div key={key} className={opts.half ? "" : "@md:col-span-2"}>
        <label className="mb-1 block text-xs text-muted-foreground">{label}</label>
        <Input
          className="h-8 text-sm"
          type={opts.type ?? "text"}
          value={value}
          onChange={(e) => {
            const next =
              opts.type === "number"
                ? e.target.value
                  ? Number(e.target.value)
                  : null
                : e.target.value || null;
            updateField(key, next as never);
          }}
        />
      </div>
    );
  };

  return (
    <div className="@container flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between gap-2 border-b px-3">
        <div className="flex min-w-0 items-center gap-2">
          {isEpisode ? (
            <Tv className="h-4 w-4 text-muted-foreground" />
          ) : (
            <Film className="h-4 w-4 text-muted-foreground" />
          )}
          <span className="hidden text-xs font-medium uppercase tracking-wide text-muted-foreground @sm:inline">
            {isEpisode ? "Episode" : "Movie"}
          </span>
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="outline" className="gap-1 text-[10px]">
                <FileText className="h-3 w-3" />
                {source.label}
              </Badge>
            </TooltipTrigger>
            <TooltipContent>{source.hint}</TooltipContent>
          </Tooltip>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            className="h-7 gap-1 px-2 text-xs"
            onClick={() => setRematchOpen(true)}
          >
            <Search className="h-3.5 w-3.5" />
            <span className="hidden @sm:inline">Rematch</span>
          </Button>
          {isDirty && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1 px-2 text-xs"
              onClick={revertChanges}
              disabled={isSaving}
            >
              <Undo2 className="h-3.5 w-3.5" />
              <span className="hidden @md:inline">Revert</span>
            </Button>
          )}
          <Button
            size="sm"
            className="h-7 gap-1 px-2 text-xs"
            onClick={handleSave}
            disabled={!isDirty || isSaving}
          >
            {isSaving ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Save className="h-3.5 w-3.5" />
            )}
            <span className="hidden @sm:inline">Save NFO</span>
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Poster sits beside the fields only when the pane can afford it. */}
        <div className="flex flex-col gap-4 @lg:flex-row">
          <div className="flex shrink-0 flex-col items-center @lg:block">
            {/* Poster is its own rematch surface, like the music cover art:
                click to browse, right-click for the menu. */}
            <ContextMenu>
              <ContextMenuTrigger asChild>
                <button
                  className="group relative rounded-md"
                  onClick={() => setPosterPickerOpen(true)}
                  aria-label="Change poster"
                >
                  {poster ? (
                    <img
                      src={`data:${poster.mimeType};base64,${poster.data}`}
                      alt="Poster"
                      className="rounded-md border object-cover"
                      style={{ width: 150, height: 225 }}
                    />
                  ) : (
                    <div
                      className="flex flex-col items-center justify-center rounded-md border bg-muted"
                      style={{ width: 150, height: 225 }}
                    >
                      <ImageIcon className="h-8 w-8 text-muted-foreground" />
                      <span className="mt-1 px-2 text-center text-xs text-muted-foreground">
                        No poster
                      </span>
                    </div>
                  )}
                  <div className="absolute inset-0 flex items-center justify-center rounded-md bg-black/0 opacity-0 transition-all group-hover:bg-black/40 group-hover:opacity-100">
                    <Pencil className="h-6 w-6 text-white drop-shadow" />
                  </div>
                </button>
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuItem onClick={() => setPosterPickerOpen(true)}>
                  <Replace className="mr-2 h-4 w-4" />
                  Change poster
                </ContextMenuItem>
              </ContextMenuContent>
            </ContextMenu>
            <p className="mt-1 w-[150px] text-center text-[10px] text-muted-foreground">
              Click to change the poster
            </p>
          </div>

          <div className="min-w-0 flex-1 space-y-3">
            <div className="grid grid-cols-1 gap-x-3 gap-y-2 @md:grid-cols-2">
              {textField("title", "Title")}
              {isEpisode && textField("showTitle", "Show Title")}
              {isEpisode && textField("season", "Season", { type: "number", half: true })}
              {isEpisode && textField("episode", "Episode", { type: "number", half: true })}
              {isEpisode && textField("aired", "Aired", { half: true })}
              {!isEpisode && textField("originalTitle", "Original Title")}
              {textField("year", "Year", { type: "number", half: true })}
              {textField("releaseDate", "Release Date", { half: true })}
              {textField("runtimeMinutes", "Runtime (min)", {
                type: "number",
                half: true,
              })}
              {textField("certification", "Certification", { half: true })}
              {textField("rating", "Rating", { type: "number", half: true })}
              {textField("votes", "Votes", { type: "number", half: true })}
              {!isEpisode && textField("tagline", "Tagline")}
            </div>

            <div>
              <label className="mb-1 block text-xs text-muted-foreground">Plot</label>
              <textarea
                className="min-h-[80px] w-full rounded-md border bg-transparent px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={currentMetadata.plot ?? ""}
                onChange={(e) => updateField("plot", e.target.value || null)}
              />
            </div>

            <ListField
              label="Genres"
              values={currentMetadata.genres}
              onChange={(v) => updateField("genres", v)}
              placeholder="Drama; Thriller"
            />
            <ListField
              label="Directors"
              values={currentMetadata.directors}
              onChange={(v) => updateField("directors", v)}
            />
            <ListField
              label="Writers"
              values={currentMetadata.writers}
              onChange={(v) => updateField("writers", v)}
            />
            <ListField
              label="Studios"
              values={currentMetadata.studios}
              onChange={(v) => updateField("studios", v)}
            />
            <ListField
              label="Countries"
              values={currentMetadata.countries}
              onChange={(v) => updateField("countries", v)}
            />

            <div>
              <label className="mb-1 block text-xs text-muted-foreground">Cast</label>
              <div className="space-y-1">
                {currentMetadata.actors.map((actor, i) => (
                  <div key={i} className="flex gap-1">
                    <Input
                      className="h-7 flex-1 text-xs"
                      value={actor.name}
                      placeholder="Name"
                      onChange={(e) => {
                        const next = [...currentMetadata.actors];
                        next[i] = { ...next[i], name: e.target.value };
                        updateField("actors", next);
                      }}
                    />
                    <Input
                      className="h-7 flex-1 text-xs"
                      value={actor.role ?? ""}
                      placeholder="Role"
                      onChange={(e) => {
                        const next = [...currentMetadata.actors];
                        next[i] = { ...next[i], role: e.target.value || null };
                        updateField("actors", next);
                      }}
                    />
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-7 w-7 shrink-0"
                      onClick={() =>
                        updateField(
                          "actors",
                          currentMetadata.actors.filter((_, j) => j !== i),
                        )
                      }
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  </div>
                ))}
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 gap-1 text-xs"
                  onClick={() =>
                    updateField("actors", [
                      ...currentMetadata.actors,
                      { name: "", role: null, thumb: null },
                    ])
                  }
                >
                  <Plus className="h-3 w-3" />
                  Add actor
                </Button>
              </div>
            </div>

            <Separator />

            <div className="grid grid-cols-1 gap-x-3 gap-y-2 @md:grid-cols-3">
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  IMDb ID
                </label>
                <Input
                  className="h-8 font-mono text-xs"
                  value={currentMetadata.imdbId ?? ""}
                  onChange={(e) => updateField("imdbId", e.target.value || null)}
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  TMDB ID
                </label>
                <Input
                  className="h-8 font-mono text-xs"
                  value={currentMetadata.tmdbId ?? ""}
                  onChange={(e) => updateField("tmdbId", e.target.value || null)}
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-muted-foreground">
                  TheTVDB ID
                </label>
                <Input
                  className="h-8 font-mono text-xs"
                  value={currentMetadata.tvdbId ?? ""}
                  onChange={(e) => updateField("tvdbId", e.target.value || null)}
                />
              </div>
            </div>

            {properties && (
              <div>
                <h3 className="mb-2 text-xs font-medium text-muted-foreground">
                  File
                </h3>
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary">{properties.container}</Badge>
                  {properties.durationMs != null && (
                    <Badge variant="secondary">
                      {formatDuration(properties.durationMs)}
                    </Badge>
                  )}
                  <Badge variant="secondary">
                    {formatSize(properties.fileSize)}
                  </Badge>
                </div>
              </div>
            )}

            <div>
              <h3 className="mb-1 text-xs font-medium text-muted-foreground">
                Path
              </h3>
              <p className="break-all font-mono text-xs text-muted-foreground">
                {currentPath}
              </p>
              {currentMetadata.nfoPath && (
                <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                  NFO: {currentMetadata.nfoPath}
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      <VideoRematchDialog open={rematchOpen} onOpenChange={setRematchOpen} />
      <VideoPosterPicker
        open={posterPickerOpen}
        onOpenChange={setPosterPickerOpen}
      />
    </div>
  );
}
