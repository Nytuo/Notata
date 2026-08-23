import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Save, Undo2, Loader2, Search, Music, BookMarked } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { useMetadataStore } from "@/stores/metadataStore";
import { CoverArtDisplay } from "@/components/coverart/CoverArtDisplay";
import { CoverArtPicker } from "@/components/coverart/CoverArtPicker";
import { RematchDialog } from "@/components/metadata/RematchDialog";
import { ChaptersDialog } from "@/components/chapters/ChaptersDialog";
import { toast } from "sonner";
import type { TrackMetadata } from "@/lib/types";

function formatDuration(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function formatBitrate(kbps: number): string {
  return `${kbps} kbps`;
}

function formatSampleRate(hz: number): string {
  return `${(hz / 1000).toFixed(1)} kHz`;
}

export function MetadataPanel() {
  const { t } = useTranslation("metadata");
  const [rematchOpen, setRematchOpen] = useState(false);
  const [chaptersOpen, setChaptersOpen] = useState(false);
  const [coverArtPickerOpen, setCoverArtPickerOpen] = useState(false);
  const {
    currentPath,
    currentMetadata,
    audioProperties,
    coverArt,
    isDirty,
    isLoading,
    isSaving,
    updateField,
    saveMetadata,
    revertChanges,
  } = useMetadataStore();

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
        Select a file to view metadata
      </div>
    );
  }

  const handleSave = async () => {
    try {
      await saveMetadata();
      toast.success(t("save_success"));
    } catch {
      toast.error(t("save_error"));
    }
  };

  const fields: {
    key: keyof TrackMetadata;
    label: string;
    type?: "number" | "text";
    half?: boolean;
  }[] = [
    { key: "title", label: t("fields.title") },
    { key: "artist", label: t("fields.artist") },
    { key: "albumArtist", label: t("fields.album_artist") },
    { key: "album", label: t("fields.album") },
    { key: "trackNumber", label: t("fields.track_number"), type: "number", half: true },
    { key: "totalTracks", label: t("fields.total_tracks"), type: "number", half: true },
    { key: "discNumber", label: t("fields.disc_number"), type: "number", half: true },
    { key: "totalDiscs", label: t("fields.total_discs"), type: "number", half: true },
    { key: "year", label: t("fields.year"), type: "number", half: true },
    { key: "date", label: t("fields.date"), half: true },
    { key: "comment", label: t("fields.comment") },
    { key: "isrc", label: t("fields.isrc") },
  ];

  return (
    <div className="@container flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center gap-2 border-b px-3">
        <Music className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {t("title")}
        </span>

        <div className="ml-auto flex items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            className="h-7 gap-1 px-2 text-xs"
            onClick={() => setChaptersOpen(true)}
          >
            <BookMarked className="h-3.5 w-3.5" />
            <span className="hidden @sm:inline">Chapters</span>
          </Button>
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
            <span className="hidden @sm:inline">Save</span>
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Cover sits beside the fields only when the pane is wide enough;
            below that it stacks so the inputs keep a usable width. */}
        <div className="flex flex-col gap-4 @lg:flex-row">
          <div className="flex justify-center @lg:block">
            <CoverArtDisplay
              coverArt={coverArt}
              onClick={() => setCoverArtPickerOpen(true)}
              onRemove={() => {
                // TODO: remove embedded cover art
              }}
            />
          </div>

          <div className="flex-1 space-y-3 min-w-0">
            <div className="grid grid-cols-1 gap-x-3 gap-y-2 @md:grid-cols-2">
              {fields.map((field) => (
                <div
                  key={field.key}
                  className={field.half ? "" : "@md:col-span-2"}
                >
                  <label className="mb-1 block text-xs text-muted-foreground">
                    {field.label}
                  </label>
                  <Input
                    className="h-8 text-sm"
                    type={field.type || "text"}
                    value={getValue(currentMetadata, field.key)}
                    onChange={(e) => {
                      const val =
                        field.type === "number"
                          ? e.target.value
                            ? parseInt(e.target.value, 10)
                            : null
                          : e.target.value || null;
                      updateField(field.key, val as never);
                    }}
                  />
                </div>
              ))}
            </div>

            <div>
              <label className="mb-1 block text-xs text-muted-foreground">
                {t("fields.genre")}
              </label>
              <Input
                className="h-8 text-sm"
                value={currentMetadata.genre?.join("; ") ?? ""}
                onChange={(e) => {
                  const genres = e.target.value
                    ? e.target.value.split(";").map((s) => s.trim()).filter(Boolean)
                    : null;
                  updateField("genre", genres);
                }}
                placeholder="Rock; Alternative"
              />
            </div>

            <div>
              <label className="mb-1 block text-xs text-muted-foreground">
                {t("fields.composer")}
              </label>
              <Input
                className="h-8 text-sm"
                value={currentMetadata.composer?.join("; ") ?? ""}
                onChange={(e) => {
                  const composers = e.target.value
                    ? e.target.value.split(";").map((s) => s.trim()).filter(Boolean)
                    : null;
                  updateField("composer", composers);
                }}
              />
            </div>

            <Separator />

            {audioProperties && (
              <div>
                <h3 className="mb-2 text-xs font-medium text-muted-foreground">
                  {t("properties.title")}
                </h3>
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary">{audioProperties.format}</Badge>
                  <Badge variant="secondary">
                    {formatDuration(audioProperties.durationMs)}
                  </Badge>
                  <Badge variant="secondary">
                    {formatBitrate(audioProperties.bitrateKbps)}
                  </Badge>
                  <Badge variant="secondary">
                    {formatSampleRate(audioProperties.sampleRateHz)}
                  </Badge>
                  <Badge variant="secondary">
                    {audioProperties.channels}ch
                  </Badge>
                  {audioProperties.bitsPerSample && (
                    <Badge variant="secondary">
                      {audioProperties.bitsPerSample}bit
                    </Badge>
                  )}
                </div>
              </div>
            )}

            <div>
              <h3 className="mb-1 text-xs font-medium text-muted-foreground">
                {t("properties.path")}
              </h3>
              <p className="break-all font-mono text-xs text-muted-foreground">
                {currentPath}
              </p>
            </div>
          </div>
        </div>
      </div>

      <RematchDialog open={rematchOpen} onOpenChange={setRematchOpen} />
      <CoverArtPicker open={coverArtPickerOpen} onOpenChange={setCoverArtPickerOpen} />
      <ChaptersDialog
        open={chaptersOpen}
        onOpenChange={setChaptersOpen}
        path={currentPath}
        audioProperties={audioProperties}
      />
    </div>
  );
}

function getValue(
  metadata: TrackMetadata,
  key: keyof TrackMetadata,
): string {
  const val = metadata[key];
  if (val === null || val === undefined) return "";
  if (typeof val === "number") return val.toString();
  return String(val);
}
