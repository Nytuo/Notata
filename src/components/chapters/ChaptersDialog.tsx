import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  BookMarked,
  Check,
  Info,
  Loader2,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Save,
  Sparkles,
  Trash2,
  TriangleAlert,
  Waves,
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
import { Progress } from "@/components/ui/progress";
import { Separator } from "@/components/ui/separator";
import { commands } from "@/lib/tauri";
import { toast } from "sonner";
import type {
  Allin1Status,
  AudioProperties,
  Chapter,
  ChapterDetectProgress,
} from "@/lib/types";

interface ChaptersDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  path: string;
  audioProperties: AudioProperties | null;
  onSaved?: () => void;
}

function mimeTypeFor(format: string | null | undefined): string {
  switch (format?.toUpperCase()) {
    case "MP3":
      return "audio/mpeg";
    case "FLAC":
      return "audio/flac";
    case "WAV":
      return "audio/wav";
    case "OGG":
    case "VORBIS":
      return "audio/ogg";
    case "OPUS":
      return "audio/opus";
    case "M4A":
    case "M4B":
    case "AAC":
    case "MP4":
      return "audio/mp4";
    default:
      return "audio/*";
  }
}

function formatMs(ms: number): string {
  const clamped = Math.max(0, Math.floor(ms));
  const minutes = Math.floor(clamped / 60_000);
  const seconds = Math.floor((clamped % 60_000) / 1000);
  const millis = clamped % 1000;
  return `${minutes}:${seconds.toString().padStart(2, "0")}.${millis
    .toString()
    .padStart(3, "0")}`;
}

function parseTimeInput(value: string): number | null {
  const match = value.trim().match(/^(\d+):(\d{1,2})(?:\.(\d{1,3}))?$/);
  if (!match) return null;
  const minutes = parseInt(match[1], 10);
  const seconds = parseInt(match[2], 10);
  const millis = match[3] ? parseInt(match[3].padEnd(3, "0"), 10) : 0;
  if (seconds >= 60) return null;
  return minutes * 60_000 + seconds * 1000 + millis;
}

interface EditableChapter {
  key: string;
  title: string;
  startMs: number;
}

let nextKey = 0;

function toEditable(chapters: Chapter[]): EditableChapter[] {
  return chapters
    .map((c) => ({ key: `${nextKey++}`, title: c.title, startMs: c.startMs }))
    .sort((a, b) => a.startMs - b.startMs);
}

export function ChaptersDialog({
  open,
  onOpenChange,
  path,
  audioProperties,
  onSaved,
}: ChaptersDialogProps) {
  const [chapters, setChapters] = useState<EditableChapter[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isDetecting, setIsDetecting] = useState<"dsp" | "ai" | null>(null);
  const [detectProgress, setDetectProgress] = useState<ChapterDetectProgress | null>(null);
  const [allin1Status, setAllin1Status] = useState<Allin1Status | null>(null);
  const [checkingAllin1, setCheckingAllin1] = useState(false);

  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const objectUrlRef = useRef<string | null>(null);
  const timelineRef = useRef<HTMLDivElement | null>(null);

  const durationMs = audioProperties?.durationMs ?? 0;

  useEffect(() => {
    if (!open || !path) return;
    setIsLoading(true);
    commands
      .readChapters(path)
      .then((loaded) => setChapters(toEditable(loaded)))
      .catch((e) => toast.error(`Failed to load chapters: ${e}`))
      .finally(() => setIsLoading(false));
  }, [open, path]);

  const checkAllin1 = async () => {
    setCheckingAllin1(true);
    try {
      setAllin1Status(await commands.checkAllin1Available());
    } catch {
      setAllin1Status({ available: false, found: false, path: "allin1" });
    } finally {
      setCheckingAllin1(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    checkAllin1();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    commands
      .readAudioPreview(path)
      .then((bytes) => {
        if (cancelled) return;
        const blob = new Blob([bytes], { type: mimeTypeFor(audioProperties?.format) });
        const url = URL.createObjectURL(blob);
        objectUrlRef.current = url;
        if (audioRef.current) audioRef.current.src = url;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      audioRef.current?.pause();
      setIsPlaying(false);
      setCurrentTime(0);
      if (objectUrlRef.current) {
        URL.revokeObjectURL(objectUrlRef.current);
        objectUrlRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, path]);

  const togglePlay = () => {
    const audio = audioRef.current;
    if (!audio) return;
    if (isPlaying) {
      audio.pause();
    } else {
      audio.play();
    }
  };

  const seekTo = (ms: number) => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.currentTime = ms / 1000;
    setCurrentTime(ms);
  };

  const handleTimelineClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!timelineRef.current || durationMs === 0) return;
    const rect = timelineRef.current.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    seekTo(ratio * durationMs);
  };

  const sortedChapters = useMemo(
    () => [...chapters].sort((a, b) => a.startMs - b.startMs),
    [chapters],
  );

  const addChapter = () => {
    setChapters((prev) => [
      ...prev,
      { key: `${nextKey++}`, title: `Chapter ${prev.length + 1}`, startMs: currentTime },
    ]);
  };

  const removeChapter = (key: string) => {
    setChapters((prev) => prev.filter((c) => c.key !== key));
  };

  const updateChapter = (key: string, patch: Partial<EditableChapter>) => {
    setChapters((prev) => prev.map((c) => (c.key === key ? { ...c, ...patch } : c)));
  };

  const buildChaptersForSave = (): Chapter[] => {
    const sorted = [...chapters].sort((a, b) => a.startMs - b.startMs);
    return sorted.map((c, i) => ({
      id: `chp${i + 1}`,
      title: c.title || `Chapter ${i + 1}`,
      startMs: c.startMs,
      endMs: i + 1 < sorted.length ? sorted[i + 1].startMs : durationMs || c.startMs,
    }));
  };

  const runDetection = async (method: "dsp" | "ai") => {
    setIsDetecting(method);
    setDetectProgress(null);
    const unlisten = await listen<ChapterDetectProgress>("chapters:detect-progress", (e) => {
      setDetectProgress(e.payload);
    });

    try {
      const detected =
        method === "dsp"
          ? await commands.detectChaptersDsp(path)
          : await commands.detectChaptersAi(path);
      setChapters(toEditable(detected));
      toast.success(`Detected ${detected.length} chapters`);
    } catch (e) {
      toast.error(`Detection failed: ${e}`);
    } finally {
      unlisten();
      setIsDetecting(null);
      setDetectProgress(null);
    }
  };

  const handleDetectAi = async () => {
    if (!allin1Status?.available) {
      if (allin1Status?.found) {
        toast.error(
          `allin1 was found but isn't working${allin1Status.error ? `: ${allin1Status.error}` : ""}. See the note below.`,
        );
      } else {
        toast.error(
          "allin1 wasn't found. Install it with \"pip install allin1\" (see the note below), then click Check again.",
        );
      }
      return;
    }
    await runDetection("ai");
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await commands.writeChapters(path, buildChaptersForSave());
      toast.success("Chapters saved");
      onSaved?.();
      onOpenChange(false);
    } catch (e) {
      toast.error(`Failed to save chapters: ${e}`);
    } finally {
      setIsSaving(false);
    }
  };

  const isBusy = isLoading || isSaving || isDetecting !== null;

  return (
    <Dialog open={open} onOpenChange={(next) => !isBusy && onOpenChange(next)}>
        <DialogContent className="flex h-[80vh] w-[90vw] max-w-3xl flex-col gap-0 p-0">
          <DialogHeader className="shrink-0 border-b px-6 py-4">
            <DialogTitle className="flex items-center gap-2">
              <BookMarked className="h-5 w-5" />
              Chapters
              <Badge variant="secondary">{chapters.length}</Badge>
            </DialogTitle>
          </DialogHeader>

          <div className="min-h-0 flex-1 overflow-y-auto p-6 space-y-4">
            <audio
              ref={audioRef}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              onTimeUpdate={(e) => setCurrentTime(e.currentTarget.currentTime * 1000)}
              className="hidden"
            />

            <div className="flex items-center gap-3">
              <Button size="icon" variant="outline" className="h-8 w-8 shrink-0" onClick={togglePlay}>
                {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
              </Button>
              <span className="w-14 shrink-0 text-xs tabular-nums text-muted-foreground">
                {formatMs(currentTime)}
              </span>
              <div
                ref={timelineRef}
                onClick={handleTimelineClick}
                className="relative h-6 flex-1 cursor-pointer rounded bg-muted"
              >
                {durationMs > 0 &&
                  sortedChapters.map((c, i) => {
                    const nextStart =
                      i + 1 < sortedChapters.length ? sortedChapters[i + 1].startMs : durationMs;
                    const left = (c.startMs / durationMs) * 100;
                    const width = ((nextStart - c.startMs) / durationMs) * 100;
                    return (
                      <div
                        key={c.key}
                        title={c.title}
                        className="absolute top-0 h-full border-r border-background/60 bg-primary/30"
                        style={{ left: `${left}%`, width: `${width}%` }}
                      />
                    );
                  })}
                {durationMs > 0 && (
                  <div
                    className="absolute top-0 h-full w-0.5 bg-primary"
                    style={{ left: `${(currentTime / durationMs) * 100}%` }}
                  />
                )}
              </div>
              <span className="w-14 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                {formatMs(durationMs)}
              </span>
            </div>

            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                className="h-7 gap-1 text-xs"
                onClick={() => runDetection("dsp")}
                disabled={isBusy}
              >
                {isDetecting === "dsp" ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Waves className="h-3.5 w-3.5" />
                )}
                Detect (DSP)
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="h-7 gap-1 text-xs"
                onClick={handleDetectAi}
                disabled={isBusy || checkingAllin1}
              >
                {isDetecting === "ai" ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Sparkles className="h-3.5 w-3.5" />
                )}
                Detect (AI)
              </Button>
              {checkingAllin1 ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
              ) : allin1Status?.available ? (
                <Badge
                  variant="outline"
                  className="gap-1 border-emerald-500/40 bg-emerald-500/10 text-[10px] text-emerald-700 dark:text-emerald-400"
                >
                  <Check className="h-3 w-3" />
                  allin1 found
                </Badge>
              ) : allin1Status?.found ? (
                <Badge
                  variant="outline"
                  className="gap-1 border-amber-500/40 bg-amber-500/10 text-[10px] text-amber-700 dark:text-amber-400"
                  title={allin1Status.error ?? undefined}
                >
                  <TriangleAlert className="h-3 w-3" />
                  Found, but not working
                </Badge>
              ) : (
                <Badge
                  variant="outline"
                  className="gap-1 border-destructive/40 bg-destructive/10 text-[10px] text-destructive"
                >
                  <TriangleAlert className="h-3 w-3" />
                  Not found
                </Badge>
              )}
              <Button
                size="sm"
                variant="ghost"
                className="ml-auto h-7 gap-1 text-xs"
                onClick={addChapter}
                disabled={isBusy}
              >
                <Plus className="h-3.5 w-3.5" />
                Add chapter
              </Button>
            </div>

            <div className="flex items-start gap-2 rounded-md border bg-muted/40 p-2.5 text-xs text-muted-foreground">
              <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <div className="space-y-1">
                <p>
                  AI detection runs{" "}
                  <a
                    href="https://github.com/mir-aidj/all-in-one"
                    target="_blank"
                    rel="noreferrer"
                    className="underline underline-offset-2"
                  >
                    mir-aidj/all-in-one
                  </a>{" "}
                  locally and needs a one-time setup — it isn't bundled with Notata:
                </p>
                <ol className="list-decimal space-y-0.5 pl-4">
                  <li>Install Python 3.9–3.11</li>
                  <li>
                    Run <code className="rounded bg-background px-1 py-0.5">pip install allin1</code> in a terminal
                  </li>
                  <li>The first detection downloads ~2GB of model weights automatically and can take a while</li>
                </ol>
                <p>
                  If it's installed somewhere not on your PATH, point Notata at it from Settings →
                  Transcoding.
                </p>
                {!allin1Status?.available && !checkingAllin1 && allin1Status?.found && allin1Status.error && (
                  <pre className="max-h-24 overflow-y-auto whitespace-pre-wrap rounded-md border border-amber-500/30 bg-amber-500/5 p-2 text-[11px] text-amber-800 dark:text-amber-300">
                    {allin1Status.error}
                  </pre>
                )}
                {!allin1Status?.available && !checkingAllin1 && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="mt-1 h-6 gap-1 text-[11px]"
                    onClick={checkAllin1}
                  >
                    <RefreshCw className="h-3 w-3" />
                    Check again
                  </Button>
                )}
              </div>
            </div>

            {isDetecting && detectProgress && (
              <div className="space-y-1.5">
                <p className="truncate text-xs text-muted-foreground">{detectProgress.stage}</p>
                <Progress value={detectProgress.percent * 100} />
              </div>
            )}

            <Separator />

            <div className="space-y-2">
              {isLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                </div>
              ) : sortedChapters.length === 0 ? (
                <p className="py-8 text-center text-xs text-muted-foreground">
                  No chapters yet. Add one manually or run detection above.
                </p>
              ) : (
                sortedChapters.map((c) => (
                  <div key={c.key} className="flex items-center gap-2">
                    <Input
                      className="h-8 flex-1 text-xs"
                      value={c.title}
                      onChange={(e) => updateChapter(c.key, { title: e.target.value })}
                      placeholder="Chapter title"
                      disabled={isBusy}
                    />
                    <Input
                      className="h-8 w-24 text-xs tabular-nums"
                      defaultValue={formatMs(c.startMs)}
                      onBlur={(e) => {
                        const parsed = parseTimeInput(e.target.value);
                        if (parsed !== null) {
                          updateChapter(c.key, { startMs: parsed });
                        } else {
                          e.target.value = formatMs(c.startMs);
                        }
                      }}
                      disabled={isBusy}
                    />
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-8 shrink-0 text-xs"
                      onClick={() => updateChapter(c.key, { startMs: currentTime })}
                      disabled={isBusy}
                    >
                      Set to current
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-8 w-8 shrink-0 text-destructive"
                      onClick={() => removeChapter(c.key)}
                      disabled={isBusy}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ))
              )}
            </div>
          </div>

          <Separator />
          <div className="flex shrink-0 items-center justify-end gap-2 px-6 py-4">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={isBusy}>
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={isBusy}>
              {isSaving ? (
                <Loader2 className="mr-1 h-4 w-4 animate-spin" />
              ) : (
                <Save className="mr-1 h-4 w-4" />
              )}
              Save chapters
            </Button>
          </div>
      </DialogContent>
    </Dialog>
  );
}
