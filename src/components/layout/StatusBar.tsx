import { useTranslation } from "react-i18next";
import { Loader2, CircleDot, Check } from "lucide-react";
import { useLibraryStore } from "@/stores/libraryStore";
import { useMetadataStore } from "@/stores/metadataStore";
import { useVideoMetadataStore } from "@/stores/videoMetadataStore";
import { useSessionStore } from "@/stores/sessionStore";

export function StatusBar() {
  const { t } = useTranslation();
  const { files, selectedFileIds, isScanning, scanProgress } = useLibraryStore();
  const music = useMetadataStore();
  const video = useVideoMetadataStore();
  const modifiedPaths = useSessionStore((s) => s.modifiedPaths);

  // Either editor can hold unsaved work, so the bar reflects both.
  const isDirty = music.isDirty || video.isDirty;
  const isSaving = music.isSaving || video.isSaving;
  const activePath = music.currentPath ?? video.currentPath;

  return (
    <footer className="flex h-7 shrink-0 items-center gap-4 border-t px-3 text-xs text-muted-foreground">
      <div className="flex min-w-0 items-center gap-3">
        {isScanning ? (
          <span className="flex items-center gap-1.5">
            <Loader2 className="h-3 w-3 animate-spin" />
            {scanProgress
              ? `Scanning — ${scanProgress.scanned} files`
              : t("status.scanning")}
          </span>
        ) : (
          <span className="tabular-nums">
            {files.length > 0 ? `${files.length} files` : t("status.ready")}
          </span>
        )}

        {selectedFileIds.length > 0 && (
          <span className="tabular-nums">{selectedFileIds.length} selected</span>
        )}

        {modifiedPaths.size > 0 && (
          <span className="tabular-nums">
            {modifiedPaths.size} edited this session
          </span>
        )}
      </div>

      <div className="ml-auto flex shrink-0 items-center gap-3">
        {activePath && (
          <span className="hidden max-w-[40vw] truncate font-mono lg:inline">
            {activePath}
          </span>
        )}
        {isSaving ? (
          <span className="flex items-center gap-1.5">
            <Loader2 className="h-3 w-3 animate-spin" />
            {t("status.saving")}
          </span>
        ) : isDirty ? (
          <span className="flex items-center gap-1.5 text-amber-600 dark:text-amber-400">
            <CircleDot className="h-3 w-3" />
            Unsaved changes
          </span>
        ) : activePath ? (
          <span className="flex items-center gap-1.5">
            <Check className="h-3 w-3" />
            Saved
          </span>
        ) : null}
      </div>
    </footer>
  );
}
