import { useState, useEffect, useCallback } from "react";
import { Loader2, Check, Plus, Trash2, Pencil } from "lucide-react";
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
import { useSessionStore } from "@/stores/sessionStore";
import { toast } from "sonner";
import type { BatchEdit, BatchPreviewEntry, FieldOp } from "@/lib/types";

interface BatchEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  paths: string[];
}

const FIELDS = [
  { value: "title", label: "Title" },
  { value: "artist", label: "Artist" },
  { value: "albumartist", label: "Album Artist" },
  { value: "album", label: "Album" },
  { value: "tracknumber", label: "Track Number" },
  { value: "totaltracks", label: "Total Tracks" },
  { value: "discnumber", label: "Disc Number" },
  { value: "totaldiscs", label: "Total Discs" },
  { value: "year", label: "Year" },
  { value: "date", label: "Date" },
  { value: "genre", label: "Genre" },
  { value: "composer", label: "Composer" },
  { value: "comment", label: "Comment" },
  { value: "isrc", label: "ISRC" },
];

const OPS = [
  { value: "set", label: "Set to" },
  { value: "clear", label: "Clear" },
  { value: "replace", label: "Find & replace" },
  { value: "enumerate", label: "Number sequentially" },
];

type OpKind = FieldOp["kind"];

interface EditRow {
  id: number;
  field: string;
  kind: OpKind;
  value: string;
  find: string;
  replace: string;
  start: string;
}

function newRow(id: number): EditRow {
  return {
    id,
    field: "title",
    kind: "set",
    value: "",
    find: "",
    replace: "",
    start: "1",
  };
}

function toBatchEdit(row: EditRow): BatchEdit {
  let op: FieldOp;
  switch (row.kind) {
    case "set":
      op = { kind: "set", value: row.value };
      break;
    case "clear":
      op = { kind: "clear" };
      break;
    case "replace":
      op = { kind: "replace", find: row.find, replace: row.replace };
      break;
    case "enumerate":
      op = { kind: "enumerate", start: parseInt(row.start, 10) || 1 };
      break;
  }
  return { field: row.field, op };
}

export function BatchEditDialog({
  open,
  onOpenChange,
  paths,
}: BatchEditDialogProps) {
  const [rows, setRows] = useState<EditRow[]>([newRow(0)]);
  const [preview, setPreview] = useState<BatchPreviewEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const refreshFiles = useLibraryStore((s) => s.refreshFiles);
  const markManyModified = useSessionStore((s) => s.markManyModified);

  const loadPreview = useCallback(async () => {
    if (paths.length === 0) return;
    setIsLoading(true);
    try {
      setPreview(await commands.previewBatchEdit(paths, rows.map(toBatchEdit)));
    } catch (e) {
      toast.error(`Preview failed: ${e}`);
    } finally {
      setIsLoading(false);
    }
  }, [paths, rows]);

  useEffect(() => {
    if (!open) {
      setRows([newRow(0)]);
      setPreview([]);
    }
  }, [open]);

  const handleApply = async () => {
    setIsApplying(true);
    try {
      const results = await commands.applyBatchEdit(paths, rows.map(toBatchEdit));
      const succeeded = results.filter((r) => r.success);
      const failed = results.filter((r) => !r.success);

      markManyModified(succeeded.map((r) => r.path));
      await refreshFiles();

      if (failed.length === 0) {
        toast.success(`Updated ${succeeded.length} files`);
      } else {
        toast.warning(
          `Updated ${succeeded.length}, failed ${failed.length}: ${failed[0].error}`,
        );
      }
      onOpenChange(false);
    } catch (e) {
      toast.error(`Batch edit failed: ${e}`);
    } finally {
      setIsApplying(false);
    }
  };

  const updateRow = (id: number, patch: Partial<EditRow>) =>
    setRows((rs) => rs.map((r) => (r.id === id ? { ...r, ...patch } : r)));

  const changedCount = preview.filter((p) => p.changed).length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[92vw] sm:max-w-5xl h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="shrink-0 border-b px-6 py-4">
          <DialogTitle className="flex items-center gap-2">
            <Pencil className="h-5 w-5" />
            Batch Edit
            <Badge variant="secondary">{paths.length} files</Badge>
          </DialogTitle>
        </DialogHeader>

        <div className="shrink-0 space-y-2 border-b p-4">
          {rows.map((row) => (
            <div key={row.id} className="flex items-center gap-2">
              <Select
                value={row.field}
                onValueChange={(v) => updateRow(row.id, { field: v })}
              >
                <SelectTrigger className="h-8 w-[160px] text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {FIELDS.map((f) => (
                    <SelectItem key={f.value} value={f.value} className="text-xs">
                      {f.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Select
                value={row.kind}
                onValueChange={(v) => updateRow(row.id, { kind: v as OpKind })}
              >
                <SelectTrigger className="h-8 w-[170px] text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {OPS.map((o) => (
                    <SelectItem key={o.value} value={o.value} className="text-xs">
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {row.kind === "set" && (
                <Input
                  className="h-8 flex-1 text-xs"
                  placeholder="New value"
                  value={row.value}
                  onChange={(e) => updateRow(row.id, { value: e.target.value })}
                />
              )}
              {row.kind === "replace" && (
                <>
                  <Input
                    className="h-8 flex-1 text-xs"
                    placeholder="Find"
                    value={row.find}
                    onChange={(e) => updateRow(row.id, { find: e.target.value })}
                  />
                  <Input
                    className="h-8 flex-1 text-xs"
                    placeholder="Replace with"
                    value={row.replace}
                    onChange={(e) => updateRow(row.id, { replace: e.target.value })}
                  />
                </>
              )}
              {row.kind === "enumerate" && (
                <Input
                  className="h-8 w-[120px] text-xs"
                  type="number"
                  placeholder="Start at"
                  value={row.start}
                  onChange={(e) => updateRow(row.id, { start: e.target.value })}
                />
              )}
              {row.kind === "clear" && <div className="flex-1" />}

              <Button
                size="icon"
                variant="ghost"
                className="h-8 w-8 shrink-0"
                onClick={() => setRows((rs) => rs.filter((r) => r.id !== row.id))}
                disabled={rows.length === 1}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))}

          <div className="flex gap-2">
            <Button
              size="sm"
              variant="ghost"
              className="h-7 gap-1 text-xs"
              onClick={() =>
                setRows((rs) => [...rs, newRow(Math.max(...rs.map((r) => r.id)) + 1)])
              }
            >
              <Plus className="h-3 w-3" />
              Add rule
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs"
              onClick={loadPreview}
              disabled={isLoading || paths.length === 0}
            >
              {isLoading ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : null}
              Preview changes
            </Button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          {preview.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              Configure rules, then preview the result before applying
            </p>
          ) : (
            <div className="space-y-0">
              <div className="grid grid-cols-[1fr_100px_1fr_1fr] gap-2 border-b pb-2 text-xs font-medium text-muted-foreground">
                <div>File</div>
                <div>Field</div>
                <div>Before</div>
                <div>After</div>
              </div>
              {preview.map((p, i) => (
                <div
                  key={i}
                  className={`grid grid-cols-[1fr_100px_1fr_1fr] gap-2 border-b py-1.5 text-xs ${
                    p.changed ? "" : "opacity-40"
                  }`}
                >
                  <div className="truncate font-mono">
                    {p.path.split("/").pop()}
                  </div>
                  <div className="text-muted-foreground">{p.field}</div>
                  <div
                    className={`truncate ${p.changed ? "text-muted-foreground line-through" : ""}`}
                  >
                    {p.before || <span className="italic">empty</span>}
                  </div>
                  <div
                    className={`truncate ${p.changed ? "font-medium text-green-600 dark:text-green-400" : ""}`}
                  >
                    {p.after || <span className="italic">empty</span>}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <Separator />
        <div className="flex shrink-0 items-center justify-between px-6 py-4">
          <span className="text-xs text-muted-foreground">
            {preview.length > 0 && `${changedCount} of ${preview.length} values change`}
          </span>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleApply} disabled={isApplying || paths.length === 0}>
              {isApplying ? (
                <Loader2 className="mr-1 h-4 w-4 animate-spin" />
              ) : (
                <Check className="mr-1 h-4 w-4" />
              )}
              Apply to {paths.length} files
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
