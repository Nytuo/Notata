import { useState } from "react";
import { Loader2, Copy, Search, Trash2, Star, HardDrive } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
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
import type { DuplicateMode, DuplicateReport } from "@/lib/types";

interface DuplicatesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function DuplicatesDialog({ open, onOpenChange }: DuplicatesDialogProps) {
  const [mode, setMode] = useState<DuplicateMode>("fuzzy");
  const [report, setReport] = useState<DuplicateReport | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isResolving, setIsResolving] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const refreshFiles = useLibraryStore((s) => s.refreshFiles);

  const runScan = async () => {
    setIsScanning(true);
    setReport(null);
    setSelected(new Set());
    try {
      const result = await commands.findDuplicates(mode);
      setReport(result);
      // Pre-select everything except the copy worth keeping.
      const preselect = new Set<string>();
      result.groups.forEach((g) =>
        g.files.filter((f) => !f.recommendedKeep).forEach((f) => preselect.add(f.path)),
      );
      setSelected(preselect);
      if (result.totalGroups === 0) {
        toast.success("No duplicates found");
      }
    } catch (e) {
      toast.error(`Duplicate scan failed: ${e}`);
    } finally {
      setIsScanning(false);
    }
  };

  const handleResolve = async () => {
    setIsResolving(true);
    setConfirmOpen(false);
    try {
      const outcomes = await commands.resolveDuplicates([...selected]);
      const ok = outcomes.filter((o) => o.success).length;
      const failed = outcomes.length - ok;

      await refreshFiles();

      if (failed === 0) {
        toast.success(`Moved ${ok} files to the quarantine folder`);
      } else {
        toast.warning(`Moved ${ok}, failed ${failed}`);
      }
      await runScan();
    } catch (e) {
      toast.error(`Could not resolve duplicates: ${e}`);
    } finally {
      setIsResolving(false);
    }
  };

  const toggle = (path: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  const reclaimable = report
    ? report.groups
        .flatMap((g) => g.files)
        .filter((f) => selected.has(f.path))
        .reduce((sum, f) => sum + f.fileSize, 0)
    : 0;

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="w-[92vw] sm:max-w-6xl h-[85vh] flex flex-col p-0 gap-0">
          <DialogHeader className="shrink-0 border-b px-6 py-4">
            <DialogTitle className="flex items-center gap-2">
              <Copy className="h-5 w-5" />
              Duplicate Files
            </DialogTitle>
          </DialogHeader>

          <div className="flex shrink-0 items-center gap-2 border-b p-4">
            <Select
              value={mode}
              onValueChange={(v) => setMode(v as DuplicateMode)}
            >
              <SelectTrigger className="h-8 w-[260px] text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="fuzzy" className="text-xs">
                  Same recording (matched on tags)
                </SelectItem>
                <SelectItem value="exact" className="text-xs">
                  Byte-identical files (slower)
                </SelectItem>
              </SelectContent>
            </Select>

            <Button size="sm" className="h-8" onClick={runScan} disabled={isScanning}>
              {isScanning ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : (
                <Search className="mr-1 h-3 w-3" />
              )}
              Scan library
            </Button>

            {report && (
              <div className="ml-auto flex gap-2 text-xs text-muted-foreground">
                <Badge variant="outline">{report.totalGroups} groups</Badge>
                <Badge variant="outline">{report.totalFiles} files</Badge>
                <Badge variant="outline" className="gap-1">
                  <HardDrive className="h-3 w-3" />
                  {formatSize(report.reclaimableBytes)} reclaimable
                </Badge>
              </div>
            )}
          </div>

          <div className="flex-1 overflow-y-auto p-4">
            {isScanning ? (
              <div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground">
                <Loader2 className="h-6 w-6 animate-spin" />
                <p className="text-sm">
                  {mode === "exact"
                    ? "Hashing file contents..."
                    : "Comparing tags across the library..."}
                </p>
              </div>
            ) : !report ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                Scan the library to find duplicate tracks
              </p>
            ) : report.groups.length === 0 ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                No duplicates found
              </p>
            ) : (
              <div className="space-y-3">
                {report.groups.map((group) => (
                  <div key={group.id} className="rounded-md border">
                    <div className="flex items-center justify-between border-b bg-muted/40 px-3 py-2">
                      <span className="text-xs font-medium">{group.reason}</span>
                      <Badge variant="secondary" className="text-[10px]">
                        {formatSize(group.wastedBytes)} wasted
                      </Badge>
                    </div>
                    {group.files.map((file) => (
                      <div
                        key={file.path}
                        className="flex items-center gap-2 border-b px-3 py-2 text-xs last:border-b-0"
                      >
                        <Checkbox
                          checked={selected.has(file.path)}
                          onCheckedChange={() => toggle(file.path)}
                        />
                        {file.recommendedKeep && (
                          <Star className="h-3 w-3 shrink-0 fill-amber-400 text-amber-500" />
                        )}
                        <div className="min-w-0 flex-1">
                          <p className="truncate font-mono">{file.path}</p>
                          {(file.artist || file.title) && (
                            <p className="truncate text-[11px] text-muted-foreground">
                              {file.artist} — {file.title}
                            </p>
                          )}
                        </div>
                        <div className="flex shrink-0 gap-1">
                          {file.format && (
                            <Badge variant="outline" className="text-[10px]">
                              {file.format.toUpperCase()}
                            </Badge>
                          )}
                          {file.bitrateKbps != null && (
                            <Badge variant="outline" className="text-[10px]">
                              {file.bitrateKbps} kbps
                            </Badge>
                          )}
                          <Badge variant="outline" className="text-[10px]">
                            {formatSize(file.fileSize)}
                          </Badge>
                          {group.mode === "fuzzy" && (
                            <Badge variant="secondary" className="text-[10px]">
                              {Math.round(file.score * 100)}%
                            </Badge>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            )}
          </div>

          <Separator />
          <div className="flex shrink-0 items-center justify-between px-6 py-4">
            <span className="text-xs text-muted-foreground">
              {selected.size > 0 &&
                `${selected.size} selected · ${formatSize(reclaimable)} will be moved to quarantine`}
            </span>
            <div className="flex gap-2">
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                Close
              </Button>
              <Button
                variant="destructive"
                onClick={() => setConfirmOpen(true)}
                disabled={isResolving || selected.size === 0}
              >
                {isResolving ? (
                  <Loader2 className="mr-1 h-4 w-4 animate-spin" />
                ) : (
                  <Trash2 className="mr-1 h-4 w-4" />
                )}
                Resolve {selected.size} duplicates
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Move {selected.size} files to quarantine?</AlertDialogTitle>
            <AlertDialogDescription>
              These files will be moved into a dated quarantine folder in Notata's
              data directory, not deleted. You can restore them from there, or
              delete the folder yourself once you are happy with the result.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleResolve}>
              Move to quarantine
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
