import { useState, useEffect, useCallback } from "react";
import {
  Loader2,
  Check,
  FileText,
  AlertTriangle,
  ArrowRight,
  FolderOpen,
} from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { commands } from "@/lib/tauri";
import { useLibraryStore } from "@/stores/libraryStore";
import { toast } from "sonner";
import type { RenamePlan, RenamePreset } from "@/lib/types";

interface RenameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  paths: string[];
}

/** Tokens the music template engine understands, shown as a cheat sheet. */
const TOKENS = [
  "albumartist",
  "artist",
  "album",
  "title",
  "track",
  "totaltracks",
  "disc",
  "year",
  "date",
  "genre",
  "composer",
  "ext",
];

export function RenameDialog({ open, onOpenChange, paths }: RenameDialogProps) {
  const [presets, setPresets] = useState<RenamePreset[]>([]);
  const [template, setTemplate] = useState(
    "{albumartist}/{album}[ ({year})]/{track:02} - {title}",
  );
  const [baseDir, setBaseDir] = useState<string>("");
  const [plan, setPlan] = useState<RenamePlan | null>(null);
  const [templateError, setTemplateError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const refreshFiles = useLibraryStore((s) => s.refreshFiles);

  useEffect(() => {
    commands.listRenamePresets().then(setPresets).catch(() => {});
  }, []);

  useEffect(() => {
    if (!open) {
      setPlan(null);
      setTemplateError(null);
    }
  }, [open]);

  const loadPreview = useCallback(async () => {
    if (paths.length === 0) return;
    setIsLoading(true);
    setTemplateError(null);
    try {
      await commands.validateRenameTemplate(template);
      setPlan(
        await commands.previewRename(paths, template, baseDir || undefined),
      );
    } catch (e) {
      setTemplateError(String(e));
      setPlan(null);
    } finally {
      setIsLoading(false);
    }
  }, [paths, template, baseDir]);

  const handlePickBaseDir = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string") setBaseDir(selected);
  };

  const handleApply = async () => {
    if (!plan) return;
    setIsApplying(true);
    try {
      const outcomes = await commands.applyRename(plan.entries);
      const succeeded = outcomes.filter((o) => o.success);
      const failed = outcomes.filter((o) => !o.success);

      await refreshFiles();

      if (failed.length === 0) {
        toast.success(`Renamed ${succeeded.length} files`);
      } else {
        toast.warning(
          `Renamed ${succeeded.length}, failed ${failed.length}: ${failed[0].error}`,
        );
      }
      onOpenChange(false);
    } catch (e) {
      toast.error(`Rename failed: ${e}`);
    } finally {
      setIsApplying(false);
    }
  };

  const applicable = plan
    ? plan.entries.filter((e) => e.changed && !e.conflict).length
    : 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="shrink-0 border-b px-6 py-4">
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            Rename Files
            <Badge variant="secondary">{paths.length} files</Badge>
          </DialogTitle>
        </DialogHeader>

        <div className="shrink-0 space-y-3 border-b p-4">
          <div className="flex gap-2">
            <Select
              onValueChange={(id) => {
                const preset = presets.find((p) => p.id === id);
                if (preset) setTemplate(preset.template);
              }}
            >
              <SelectTrigger className="h-8 w-[280px] text-xs">
                <SelectValue placeholder="Load a media server preset..." />
              </SelectTrigger>
              <SelectContent>
                {presets.map((p) => (
                  <SelectItem key={p.id} value={p.id} className="text-xs">
                    <span className="font-medium">{p.label}</span>
                    <span className="ml-2 text-muted-foreground">
                      {p.description}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>

            <Input
              className="h-8 flex-1 text-xs"
              placeholder="Destination folder (defaults to each file's own folder)"
              value={baseDir}
              onChange={(e) => setBaseDir(e.target.value)}
            />
            <Button
              size="sm"
              variant="outline"
              className="h-8"
              onClick={handlePickBaseDir}
            >
              <FolderOpen className="h-3.5 w-3.5" />
            </Button>
          </div>

          <div className="flex gap-2">
            <Input
              className="h-8 flex-1 font-mono text-xs"
              value={template}
              onChange={(e) => setTemplate(e.target.value)}
              placeholder="{albumartist}/{album}/{track:02} - {title}"
            />
            <Button
              size="sm"
              className="h-8"
              onClick={loadPreview}
              disabled={isLoading || paths.length === 0}
            >
              {isLoading ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : null}
              Preview
            </Button>
          </div>

          {templateError && (
            <p className="flex items-center gap-1 text-xs text-destructive">
              <AlertTriangle className="h-3 w-3" />
              {templateError}
            </p>
          )}

          <div className="flex flex-wrap gap-1">
            {TOKENS.map((token) => (
              <Badge
                key={token}
                variant="outline"
                className="cursor-pointer font-mono text-[10px] hover:bg-accent"
                onClick={() => setTemplate((t) => `${t}{${token}}`)}
              >
                {`{${token}}`}
              </Badge>
            ))}
            <span className="ml-2 text-[10px] text-muted-foreground">
              {"{track:02} pads numbers · [ ... ] is dropped when empty · / makes folders"}
            </span>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {!plan ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              Preview the rename to see the resulting paths before applying
            </p>
          ) : (
            <div className="space-y-1">
              {plan.entries.map((entry, i) => (
                <div
                  key={i}
                  className={`rounded-md border p-2 text-xs ${
                    entry.conflict
                      ? "border-destructive/40 bg-destructive/5"
                      : entry.changed
                        ? ""
                        : "opacity-40"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className="flex-1 truncate font-mono text-muted-foreground">
                      {entry.sourcePath.split("/").pop()}
                    </span>
                    <ArrowRight className="h-3 w-3 shrink-0 text-muted-foreground" />
                    <span
                      className={`flex-1 truncate font-mono ${
                        entry.changed && !entry.conflict
                          ? "font-medium text-green-600 dark:text-green-400"
                          : ""
                      }`}
                    >
                      {entry.relativeTarget}
                    </span>
                  </div>
                  {entry.conflict && (
                    <p className="mt-1 flex items-center gap-1 text-[11px] text-destructive">
                      <AlertTriangle className="h-3 w-3 shrink-0" />
                      {entry.conflict}
                    </p>
                  )}
                  {!entry.changed && !entry.conflict && (
                    <p className="mt-1 text-[11px] text-muted-foreground">
                      Already named correctly
                    </p>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        <Separator />
        <div className="flex shrink-0 items-center justify-between px-6 py-4">
          <div className="flex gap-3 text-xs text-muted-foreground">
            {plan && (
              <>
                <span>{plan.changed} to rename</span>
                {plan.conflicts > 0 && (
                  <span className="text-destructive">
                    {plan.conflicts} conflicts (skipped)
                  </span>
                )}
              </>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleApply} disabled={isApplying || applicable === 0}>
              {isApplying ? (
                <Loader2 className="mr-1 h-4 w-4 animate-spin" />
              ) : (
                <Check className="mr-1 h-4 w-4" />
              )}
              Rename {applicable} files
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
