import { useCallback, useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  AudioWaveform,
  Check,
  FolderOpen,
  Loader2,
  TriangleAlert,
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
import { Checkbox } from "@/components/ui/checkbox";
import { Progress } from "@/components/ui/progress";
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
import type {
  FfmpegStatus,
  TranscodeFormatInfo,
  TranscodeOptions,
  TranscodePreviewEntry,
  TranscodeProgress,
  TranscodeResult,
} from "@/lib/types";

interface TranscodeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  paths: string[];
}

const BITRATE_PRESETS = [128, 192, 256, 320];

export function TranscodeDialog({ open, onOpenChange, paths }: TranscodeDialogProps) {
  const refreshFiles = useLibraryStore((s) => s.refreshFiles);
  const markManyModified = useSessionStore((s) => s.markManyModified);

  const [formats, setFormats] = useState<TranscodeFormatInfo[]>([]);
  const [ffmpeg, setFfmpeg] = useState<FfmpegStatus | null>(null);
  const [checkingFfmpeg, setCheckingFfmpeg] = useState(false);

  const [targetFormat, setTargetFormat] = useState("mp3");
  const [bitrate, setBitrate] = useState(256);
  const [flacCompression, setFlacCompression] = useState(5);
  const [preferStreamCopy, setPreferStreamCopy] = useState(true);
  const [replaceInPlace, setReplaceInPlace] = useState(true);
  const [destFolder, setDestFolder] = useState("");

  const [preview, setPreview] = useState<TranscodePreviewEntry[]>([]);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [progress, setProgress] = useState<TranscodeProgress | null>(null);

  useEffect(() => {
    if (!open) return;
    commands.listTranscodeFormats().then(setFormats).catch(() => {});
    setCheckingFfmpeg(true);
    commands
      .checkFfmpegAvailable()
      .then(setFfmpeg)
      .catch(() => setFfmpeg({ available: false, version: null, path: "ffmpeg" }))
      .finally(() => setCheckingFfmpeg(false));
  }, [open]);

  useEffect(() => {
    if (!open) {
      setPreview([]);
      setProgress(null);
    }
  }, [open]);

  const selectedFormat = formats.find((f) => f.id === targetFormat);

  useEffect(() => {
    if (selectedFormat?.defaultBitrateKbps) {
      setBitrate(selectedFormat.defaultBitrateKbps);
    }
  }, [selectedFormat]);

  const buildOptions = useCallback((): TranscodeOptions => ({
    targetFormat,
    bitrateKbps: selectedFormat?.lossy ? bitrate : null,
    flacCompression: targetFormat === "flac" ? flacCompression : null,
    preferStreamCopy,
    destination: replaceInPlace
      ? { kind: "replace_in_place" }
      : { kind: "folder", path: destFolder },
  }), [targetFormat, selectedFormat, bitrate, flacCompression, preferStreamCopy, replaceInPlace, destFolder]);

  const loadPreview = useCallback(async () => {
    if (paths.length === 0) return;
    setIsLoadingPreview(true);
    try {
      setPreview(await commands.previewTranscode(paths, buildOptions()));
    } catch (e) {
      toast.error(`Preview failed: ${e}`);
    } finally {
      setIsLoadingPreview(false);
    }
  }, [paths, buildOptions]);

  const handlePickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string") setDestFolder(selected);
  };

  const handleApply = async () => {
    if (!replaceInPlace && !destFolder.trim()) {
      toast.error("Choose a destination folder first");
      return;
    }

    setIsApplying(true);
    setProgress(null);
    const unlisten = await listen<TranscodeProgress>("transcode:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      const results: TranscodeResult[] = await commands.transcodeFiles(paths, buildOptions());
      const succeeded = results.filter((r) => r.success && !r.skipped);
      const skipped = results.filter((r) => r.skipped);
      const failed = results.filter((r) => !r.success);

      if (replaceInPlace) {
        markManyModified(
          succeeded
            .map((r) => r.outputPath)
            .filter((p): p is string => !!p),
        );
        await refreshFiles();
      }

      if (failed.length === 0) {
        toast.success(
          `Transcoded ${succeeded.length} files` +
            (skipped.length ? `, ${skipped.length} already ${targetFormat}` : ""),
        );
        onOpenChange(false);
      } else {
        toast.warning(
          `Transcoded ${succeeded.length}, failed ${failed.length}: ${failed[0].error}`,
        );
      }
    } catch (e) {
      toast.error(`Transcoding failed: ${e}`);
    } finally {
      unlisten();
      setIsApplying(false);
      setProgress(null);
    }
  };

  const changedCount = preview.filter((p) => !p.alreadyTargetExtension || !preferStreamCopy).length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[95vw] sm:max-w-[95vw] h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="shrink-0 border-b px-6 py-4">
          <DialogTitle className="flex items-center gap-2">
            <AudioWaveform className="h-5 w-5" />
            Convert audio
            <Badge variant="secondary">{paths.length} files</Badge>
          </DialogTitle>
        </DialogHeader>

        <div className="min-h-0 flex-1 overflow-y-auto p-6 space-y-5">
          {checkingFfmpeg ? (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              Checking for ffmpeg…
            </div>
          ) : ffmpeg && !ffmpeg.available ? (
            <div className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs">
              <TriangleAlert className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
              <div>
                <p className="font-medium text-destructive">
                  ffmpeg was not found at "{ffmpeg.path}"
                </p>
                <p className="mt-1 text-muted-foreground">
                  Transcoding needs ffmpeg. Install it and make sure it's on
                  your PATH, or point Notata at it from Settings → Transcoding.
                </p>
              </div>
            </div>
          ) : null}

          <div className="grid max-w-2xl grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                Target format
              </label>
              <Select value={targetFormat} onValueChange={setTargetFormat}>
                <SelectTrigger className="h-8 w-full text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {formats.map((f) => (
                    <SelectItem key={f.id} value={f.id} className="text-xs">
                      {f.label}
                      <span className="ml-2 text-muted-foreground">
                        {f.lossy ? "lossy" : "lossless"}
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {selectedFormat?.lossy && (
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  Bitrate
                </label>
                <Select
                  value={String(bitrate)}
                  onValueChange={(v) => setBitrate(parseInt(v, 10))}
                >
                  <SelectTrigger className="h-8 w-full text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {BITRATE_PRESETS.map((b) => (
                      <SelectItem key={b} value={String(b)} className="text-xs">
                        {b} kbps
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {targetFormat === "flac" && (
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  Compression (0 fastest – 8 smallest)
                </label>
                <Input
                  type="number"
                  min={0}
                  max={8}
                  className="h-8 text-xs"
                  value={flacCompression}
                  onChange={(e) =>
                    setFlacCompression(
                      Math.min(8, Math.max(0, parseInt(e.target.value, 10) || 0)),
                    )
                  }
                />
              </div>
            )}
          </div>

          <label className="flex items-center gap-2 text-xs">
            <Checkbox
              checked={preferStreamCopy}
              onCheckedChange={(v) => setPreferStreamCopy(v === true)}
            />
            Skip re-encoding when the file is already in the target codec
            (lossless container change only)
          </label>

          <Separator />

          <div className="space-y-2">
            <label className="text-xs font-medium text-muted-foreground">
              Destination
            </label>
            <div className="flex gap-3 text-xs">
              <label className="flex items-center gap-1.5">
                <input
                  type="radio"
                  checked={replaceInPlace}
                  onChange={() => setReplaceInPlace(true)}
                />
                Replace original files
              </label>
              <label className="flex items-center gap-1.5">
                <input
                  type="radio"
                  checked={!replaceInPlace}
                  onChange={() => setReplaceInPlace(false)}
                />
                Save to a folder
              </label>
            </div>
            {!replaceInPlace && (
              <div className="flex gap-2">
                <Input
                  className="h-8 flex-1 text-xs"
                  placeholder="Destination folder"
                  value={destFolder}
                  onChange={(e) => setDestFolder(e.target.value)}
                />
                <Button size="sm" variant="outline" className="h-8" onClick={handlePickFolder}>
                  <FolderOpen className="h-3.5 w-3.5" />
                </Button>
              </div>
            )}
          </div>

          <div className="flex justify-end">
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs"
              onClick={loadPreview}
              disabled={isLoadingPreview || paths.length === 0}
            >
              {isLoadingPreview ? (
                <Loader2 className="mr-1 h-3 w-3 animate-spin" />
              ) : null}
              Preview
            </Button>
          </div>

          {preview.length > 0 && (
            <div className="space-y-0">
              <div className="grid grid-cols-[1fr_1fr] gap-2 border-b pb-2 text-xs font-medium text-muted-foreground">
                <div>Source</div>
                <div>Output</div>
              </div>
              {preview.map((p, i) => (
                <div
                  key={i}
                  className="grid grid-cols-[1fr_1fr] gap-2 border-b py-1.5 text-xs"
                >
                  <div className="truncate font-mono">{p.path.split("/").pop()}</div>
                  <div className="truncate font-mono text-muted-foreground">
                    {p.outputPath.split("/").pop()}
                  </div>
                </div>
              ))}
            </div>
          )}

          {isApplying && progress && (
            <div className="space-y-1.5">
              <p className="truncate text-xs text-muted-foreground">
                {progress.index + 1} of {progress.total} — {progress.path.split("/").pop()}
              </p>
              <Progress value={progress.percent * 100} />
            </div>
          )}
        </div>

        <Separator />
        <div className="flex shrink-0 items-center justify-between px-6 py-4">
          <span className="text-xs text-muted-foreground">
            {preview.length > 0 && `${changedCount} of ${preview.length} files will be encoded`}
          </span>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={isApplying}>
              Cancel
            </Button>
            <Button
              onClick={handleApply}
              disabled={isApplying || paths.length === 0 || !ffmpeg?.available}
            >
              {isApplying ? (
                <Loader2 className="mr-1 h-4 w-4 animate-spin" />
              ) : (
                <Check className="mr-1 h-4 w-4" />
              )}
              Convert {paths.length} files
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
