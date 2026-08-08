import {
  Save,
  Undo2,
  Loader2,
  BookOpen,
  Library,
  FileText,
  ImageIcon,
  AlertTriangle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useBookMetadataStore } from "@/stores/bookMetadataStore";
import { toast } from "sonner";
import type { BookMetadata } from "@/lib/types";

const SOURCE_LABELS: Record<string, { label: string; hint: string }> = {
  comic_info: {
    label: "ComicInfo",
    hint: "Loaded from ComicInfo.xml inside the archive",
  },
  opf: { label: "OPF", hint: "Loaded from the EPUB package document" },
  filename: {
    label: "Filename",
    hint: "No embedded metadata — values guessed from the filename",
  },
  none: { label: "None", hint: "No metadata found" },
};

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

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

export function BookMetadataPanel() {
  const {
    currentPath,
    currentMetadata,
    properties,
    cover,
    isDirty,
    isLoading,
    isSaving,
    isSavingCover,
    updateField,
    saveMetadata,
    pickCover,
    revertChanges,
  } = useBookMetadataStore();

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
        Select a comic or ebook to view metadata
      </div>
    );
  }

  const handleSave = async () => {
    try {
      const entry = await saveMetadata();
      toast.success(`Saved ${entry} inside the archive`);
    } catch (e) {
      toast.error(`Could not save: ${e}`);
    }
  };

  const isComic = currentMetadata.kind === "comic";
  const source = SOURCE_LABELS[currentMetadata.source] ?? SOURCE_LABELS.none;
  const readOnly = properties ? !properties.readable : false;

  const textField = (
    key: keyof BookMetadata,
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
          {isComic ? (
            <Library className="h-4 w-4 shrink-0 text-muted-foreground" />
          ) : (
            <BookOpen className="h-4 w-4 shrink-0 text-muted-foreground" />
          )}
          <span className="hidden text-xs font-medium uppercase tracking-wide text-muted-foreground @sm:inline">
            {isComic ? "Comic" : "Ebook"}
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
            disabled={!isDirty || isSaving || readOnly}
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

      {readOnly && (
        <div className="flex shrink-0 items-center gap-2 border-b bg-amber-500/10 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
          <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
          {properties?.container} archives are read-only here — convert to CBZ to
          save changes.
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-4">
        <div className="flex flex-col gap-4 @lg:flex-row">
          <div className="flex shrink-0 flex-col items-center @lg:block">
            <button
              type="button"
              className="group relative overflow-hidden rounded-md border disabled:cursor-not-allowed disabled:opacity-70"
              style={{ width: 150, height: 225 }}
              onClick={() => pickCover()}
              disabled={readOnly || isSavingCover}
              title={readOnly ? "Read-only archive" : "Choose a cover from disk"}
            >
              {cover ? (
                <img
                  src={`data:${cover.mimeType};base64,${cover.data}`}
                  alt="Cover"
                  className="h-full w-full object-cover"
                />
              ) : (
                <div className="flex h-full w-full flex-col items-center justify-center bg-muted">
                  <ImageIcon className="h-8 w-8 text-muted-foreground" />
                  <span className="mt-1 px-2 text-center text-xs text-muted-foreground">
                    No cover
                  </span>
                </div>
              )}
              {!readOnly && (
                <div className="absolute inset-0 flex items-center justify-center bg-black/60 opacity-0 transition-opacity group-hover:opacity-100">
                  {isSavingCover ? (
                    <Loader2 className="h-6 w-6 animate-spin text-white" />
                  ) : (
                    <span className="px-2 text-center text-xs font-medium text-white">
                      Choose from disk
                    </span>
                  )}
                </div>
              )}
            </button>
          </div>

          <div className="min-w-0 flex-1 space-y-3">
            <div className="grid grid-cols-1 gap-x-3 gap-y-2 @md:grid-cols-2">
              {textField("title", "Title")}
              {textField("series", "Series")}
              {textField("number", isComic ? "Issue" : "Series index", {
                half: true,
              })}
              {isComic && textField("count", "Issue count", {
                type: "number",
                half: true,
              })}
              {isComic &&
                textField("volume", "Volume", { type: "number", half: true })}
              {textField("year", "Year", { type: "number", half: true })}
              {isComic &&
                textField("month", "Month", { type: "number", half: true })}
              {textField("publisher", "Publisher", { half: true })}
              {isComic && textField("imprint", "Imprint", { half: true })}
              {textField("language", "Language", { half: true })}
              {textField("isbn", isComic ? "GTIN / ISBN" : "ISBN", {
                half: true,
              })}
              {textField("pageCount", "Pages", { type: "number", half: true })}
              {isComic && textField("ageRating", "Age rating", { half: true })}
            </div>

            <div>
              <label className="mb-1 block text-xs text-muted-foreground">
                {isComic ? "Summary" : "Description"}
              </label>
              <textarea
                className="min-h-[80px] w-full rounded-md border bg-transparent px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                value={currentMetadata.summary ?? ""}
                onChange={(e) => updateField("summary", e.target.value || null)}
              />
            </div>

            <ListField
              label={isComic ? "Writers" : "Authors"}
              values={currentMetadata.authors}
              onChange={(v) => updateField("authors", v)}
              placeholder="Neil Gaiman; Sam Kieth"
            />

            {isComic && (
              <>
                <ListField
                  label="Pencillers"
                  values={currentMetadata.pencillers}
                  onChange={(v) => updateField("pencillers", v)}
                />
                <ListField
                  label="Inkers"
                  values={currentMetadata.inkers}
                  onChange={(v) => updateField("inkers", v)}
                />
                <ListField
                  label="Colorists"
                  values={currentMetadata.colorists}
                  onChange={(v) => updateField("colorists", v)}
                />
                <ListField
                  label="Letterers"
                  values={currentMetadata.letterers}
                  onChange={(v) => updateField("letterers", v)}
                />
                <ListField
                  label="Cover artists"
                  values={currentMetadata.coverArtists}
                  onChange={(v) => updateField("coverArtists", v)}
                />
              </>
            )}

            <ListField
              label="Editors"
              values={currentMetadata.editors}
              onChange={(v) => updateField("editors", v)}
            />
            <ListField
              label="Translators"
              values={currentMetadata.translators}
              onChange={(v) => updateField("translators", v)}
            />
            <ListField
              label={isComic ? "Genres" : "Subjects"}
              values={currentMetadata.genres}
              onChange={(v) => updateField("genres", v)}
            />

            {isComic && (
              <>
                <ListField
                  label="Characters"
                  values={currentMetadata.characters}
                  onChange={(v) => updateField("characters", v)}
                />
                <div className="grid grid-cols-1 gap-x-3 gap-y-2 @md:grid-cols-2">
                  {textField("storyArc", "Story arc", { half: true })}
                  {textField("web", "Web", { half: true })}
                </div>
              </>
            )}

            <Separator />

            {properties && (
              <div>
                <h3 className="mb-2 text-xs font-medium text-muted-foreground">
                  File
                </h3>
                <div className="flex flex-wrap gap-2">
                  <Badge variant="secondary">{properties.container}</Badge>
                  <Badge variant="secondary">
                    {formatSize(properties.fileSize)}
                  </Badge>
                  {properties.pageCount != null && (
                    <Badge variant="secondary">
                      {properties.pageCount} pages
                    </Badge>
                  )}
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
              {currentMetadata.entryPath && (
                <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                  Entry: {currentMetadata.entryPath}
                </p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
