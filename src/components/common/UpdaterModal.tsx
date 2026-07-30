import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import {
  AlertCircle,
  ArrowDownCircle,
  CheckCircle2,
  ExternalLink,
  Loader2,
  RefreshCw,
  X,
} from "lucide-react";

type Phase = "idle" | "downloading" | "installing" | "done" | "error" | "restart";

interface UpdateInfo {
  version: string;
  currentVersion: string;
  body: string | null;
  date: string | null;
}

interface ProgressPayload {
  stage: "downloading" | "installing" | "done";
  downloaded: number;
  total: number;
  percentage: number;
}

/** Custom event other components fire to open the update check. */
export const CHECK_FOR_UPDATES_EVENT = "notata:check-for-updates";

export function requestUpdateCheck() {
  window.dispatchEvent(new CustomEvent(CHECK_FOR_UPDATES_EVENT));
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function UpdaterModal() {
  const [open, setOpen] = useState(false);
  const [phase, setPhase] = useState<Phase>("idle");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState(0);
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState(0);
  const [errorMsg, setErrorMsg] = useState("");
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    // Registered on mount so no progress event is missed while the async
    // listener registration is still in flight.
    const unlistenProgress = listen<ProgressPayload>(
      "updater-progress",
      (event) => {
        const { stage, downloaded, total, percentage } = event.payload;
        if (stage === "downloading") {
          setPhase("downloading");
          setDownloaded(downloaded);
          setTotal(total);
          setProgress(percentage);
        } else if (stage === "installing") {
          setPhase("installing");
          setProgress(99);
        } else if (stage === "done") {
          setPhase("restart");
          setProgress(100);
        }
      },
    );

    return () => {
      unlistenProgress.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const handler = () => void handleCheckNow();
    window.addEventListener(CHECK_FOR_UPDATES_EVENT, handler);
    return () => window.removeEventListener(CHECK_FOR_UPDATES_EVENT, handler);
  });

  async function handleCheckNow() {
    setChecking(true);
    setErrorMsg("");
    setUpdateInfo(null);
    try {
      const info = await invoke<UpdateInfo | null>("check_for_update");
      if (info) {
        setUpdateInfo(info);
        setPhase("idle");
      } else {
        setPhase("done");
      }
      setOpen(true);
    } catch (err) {
      setErrorMsg(String(err));
      setPhase("error");
      setOpen(true);
    } finally {
      setChecking(false);
    }
  }

  async function handleInstall() {
    setPhase("downloading");
    setProgress(0);
    setErrorMsg("");
    try {
      await invoke("install_update");
    } catch (err) {
      setErrorMsg(String(err));
      setPhase("error");
    }
  }

  async function handleOpenReleases() {
    await invoke("open_releases_page");
  }

  function handleRestart() {
    invoke("restart_app");
  }

  // A download or install must not be interrupted by dismissing the dialog.
  const isBlocked = phase === "downloading" || phase === "installing";

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!isBlocked) setOpen(next);
      }}
    >
      <DialogContent
        showCloseButton={false}
        onEscapeKeyDown={(e) => {
          if (isBlocked) e.preventDefault();
        }}
        onPointerDownOutside={(e) => {
          if (isBlocked) e.preventDefault();
        }}
        className="sm:max-w-lg"
      >
        <DialogHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <ArrowDownCircle className="size-5 shrink-0 text-primary" />
              <DialogTitle>
                {phase === "done" && !updateInfo
                  ? "Up to date"
                  : "Update available"}
              </DialogTitle>
            </div>
            {!isBlocked && (
              <button
                onClick={() => setOpen(false)}
                className="rounded-sm text-muted-foreground transition-colors hover:text-foreground"
                aria-label="Dismiss"
              >
                <X className="size-4" />
              </button>
            )}
          </div>

          {updateInfo && (
            <DialogDescription asChild>
              <div className="flex items-center gap-1.5 text-sm">
                <span className="text-muted-foreground">
                  v{updateInfo.currentVersion}
                </span>
                <span className="text-muted-foreground">→</span>
                <span className="font-semibold text-foreground">
                  v{updateInfo.version}
                </span>
                {updateInfo.date && (
                  <span className="ml-1 text-muted-foreground">
                    ·{" "}
                    {new Date(updateInfo.date).toLocaleDateString(undefined, {
                      year: "numeric",
                      month: "short",
                      day: "numeric",
                    })}
                  </span>
                )}
              </div>
            </DialogDescription>
          )}
        </DialogHeader>

        <div className="flex flex-col gap-4">
          {updateInfo?.body && phase === "idle" && (
            <div className="max-h-48 overflow-y-auto rounded-md bg-muted px-3 py-2 text-sm">
              <pre className="font-sans whitespace-pre-wrap break-words">
                {updateInfo.body}
              </pre>
            </div>
          )}

          {phase === "done" && !updateInfo && (
            <div className="flex items-center gap-2 rounded-md border border-green-500/50 bg-green-500/10 p-3 text-sm text-green-600 dark:text-green-400">
              <CheckCircle2 className="size-4 shrink-0" />
              <span>You're running the latest version.</span>
            </div>
          )}

          {phase === "error" && (
            <div className="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
              <AlertCircle className="mt-0.5 size-4 shrink-0" />
              <span>{errorMsg}</span>
            </div>
          )}

          {phase === "downloading" && (
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between text-sm">
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <Loader2 className="size-3.5 animate-spin" />
                  Downloading…
                </span>
                <span className="font-medium tabular-nums">{progress}%</span>
              </div>
              <Progress value={progress} className="h-3" />
              {total > 0 && (
                <p className="text-right text-xs tabular-nums text-muted-foreground">
                  {formatBytes(downloaded)} / {formatBytes(total)}
                </p>
              )}
            </div>
          )}

          {phase === "installing" && (
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between text-sm">
                <span className="flex items-center gap-1.5 text-muted-foreground">
                  <Loader2 className="size-3.5 animate-spin" />
                  Installing…
                </span>
                <span className="font-medium tabular-nums">99%</span>
              </div>
              <Progress value={99} className="h-3" />
            </div>
          )}

          {phase === "restart" && (
            <div className="flex flex-col gap-3">
              <div className="flex items-center gap-2 rounded-md border border-green-500/50 bg-green-500/10 p-3 text-sm text-green-600 dark:text-green-400">
                <CheckCircle2 className="size-4 shrink-0" />
                <span>Update installed. Restart to apply.</span>
              </div>
              <Button onClick={handleRestart} className="w-full gap-2">
                <RefreshCw className="size-4" />
                Restart now
              </Button>
            </div>
          )}

          {(phase === "idle" || phase === "error") && (
            <div className="flex flex-col gap-2">
              {updateInfo && (
                <Button
                  onClick={handleInstall}
                  className="w-full gap-2"
                  disabled={checking}
                >
                  <ArrowDownCircle className="size-4" />
                  {phase === "error" ? "Retry update" : "Update now"}
                </Button>
              )}

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  className="flex-1 gap-1.5"
                  onClick={handleOpenReleases}
                >
                  <ExternalLink className="size-3.5" />
                  All releases
                </Button>
                <Button
                  variant="ghost"
                  className="flex-1"
                  onClick={() => setOpen(false)}
                  disabled={isBlocked}
                >
                  Later
                </Button>
              </div>
            </div>
          )}

          {phase === "done" && !updateInfo && (
            <Button variant="outline" onClick={() => setOpen(false)}>
              Close
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
