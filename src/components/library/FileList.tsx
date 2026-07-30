import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Music, Pencil, Sparkles } from "lucide-react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useLibraryStore } from "@/stores/libraryStore";
import { useMetadataStore } from "@/stores/metadataStore";
import { useVideoMetadataStore } from "@/stores/videoMetadataStore";
import { useBookMetadataStore } from "@/stores/bookMetadataStore";
import { useSessionStore } from "@/stores/sessionStore";
import { FileListToolbar } from "@/components/library/FileListToolbar";
import type { MediaFile } from "@/lib/types";

function formatDuration(ms: number | null): string {
  if (!ms) return "--:--";
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatAudioFormat(format: MediaFile["audioFormat"]): string {
  if (!format) return "";
  if (typeof format === "string") return format.toUpperCase();
  return Object.values(format)[0]?.toUpperCase() ?? "";
}

function StatusBadge({ file }: { file: MediaFile }) {
  const statusOf = useSessionStore((s) => s.statusOf);
  // Subscribing to the set keeps the badge live as edits land.
  useSessionStore((s) => s.modifiedPaths);
  const status = statusOf(file);

  if (status === "unchanged") return null;

  const config =
    status === "modified"
      ? {
          icon: Pencil,
          label: "Edited",
          tip: "Modified during this session",
          className:
            "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-400",
        }
      : {
          icon: Sparkles,
          label: "New",
          tip: "First seen in the most recent scan",
          className:
            "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
        };

  const Icon = config.icon;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Badge variant="outline" className={`gap-1 text-[10px] ${config.className}`}>
          <Icon className="h-3 w-3 shrink-0" />
          {/* Collapses to the icon alone in a narrow pane; the tooltip
              always carries the full meaning. */}
          <span className="hidden @lg:inline">{config.label}</span>
        </Badge>
      </TooltipTrigger>
      <TooltipContent>{config.tip}</TooltipContent>
    </Tooltip>
  );
}

export function FileList() {
  const { t } = useTranslation("library");
  const {
    files,
    selectedFileIds,
    selectFiles,
    statusFilter,
    searchFilter,
  } = useLibraryStore();
  const { loadMetadata, currentPath } = useMetadataStore();
  const loadVideoMetadata = useVideoMetadataStore((s) => s.loadMetadata);
  const videoPath = useVideoMetadataStore((s) => s.currentPath);
  const loadBookMetadata = useBookMetadataStore((s) => s.loadMetadata);
  const bookPath = useBookMetadataStore((s) => s.currentPath);
  const statusOf = useSessionStore((s) => s.statusOf);
  const modifiedPaths = useSessionStore((s) => s.modifiedPaths);

  // Whichever editor is showing owns the highlight.
  const activePath = currentPath ?? videoPath ?? bookPath;

  const visibleFiles = useMemo(() => {
    const query = searchFilter.trim().toLowerCase();
    return files.filter((file) => {
      if (statusFilter !== "all" && statusOf(file) !== statusFilter) {
        return false;
      }
      if (query && !file.fileName.toLowerCase().includes(query)) {
        return false;
      }
      return true;
    });
    // `modifiedPaths` is read indirectly through `statusOf`; listing it here
    // keeps the memo in step as files are edited.
  }, [files, statusFilter, searchFilter, statusOf, modifiedPaths]);

  const handleRowClick = (file: MediaFile, e: React.MouseEvent) => {
    if (e.shiftKey && selectedFileIds.length > 0) {
      // Range-select from the last anchor through the clicked row.
      const ids = visibleFiles.map((f) => f.id);
      const anchor = ids.indexOf(selectedFileIds[selectedFileIds.length - 1]);
      const target = ids.indexOf(file.id);
      if (anchor !== -1 && target !== -1) {
        const [from, to] = anchor < target ? [anchor, target] : [target, anchor];
        selectFiles(ids.slice(from, to + 1));
        return;
      }
    }

    if (e.ctrlKey || e.metaKey) {
      selectFiles(
        selectedFileIds.includes(file.id)
          ? selectedFileIds.filter((id) => id !== file.id)
          : [...selectedFileIds, file.id],
      );
      return;
    }

    selectFiles([file.id]);
    if (file.mediaType === "video") {
      loadVideoMetadata(file.path);
    } else if (file.mediaType === "comic" || file.mediaType === "book") {
      loadBookMetadata(file.path);
    } else {
      loadMetadata(file.path);
    }
  };

  return (
    <div className="@container flex h-full flex-col">
      <FileListToolbar visibleFiles={visibleFiles} />

      {files.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-muted-foreground">
          <Music className="h-10 w-10" />
          <p className="text-sm">{t("no_files")}</p>
          <p className="text-xs">Select a library folder on the left.</p>
        </div>
      ) : visibleFiles.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center text-muted-foreground">
          <Music className="h-9 w-9" />
          <p className="text-sm">No files match the current filter</p>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto">
          <Table>
            <TableHeader className="sticky top-0 z-10 bg-background">
              <TableRow>
                <TableHead className="w-[38%]">{t("columns.filename")}</TableHead>
                {/* Secondary columns drop as the pane narrows, so the table
                    never needs a horizontal scrollbar to stay usable. */}
                <TableHead className="hidden @2xl:table-cell">
                  {t("columns.title")}
                </TableHead>
                <TableHead className="hidden @3xl:table-cell">
                  {t("columns.artist")}
                </TableHead>
                <TableHead className="hidden w-[70px] text-right @lg:table-cell">
                  {t("columns.duration")}
                </TableHead>
                <TableHead className="hidden w-[60px] @xl:table-cell">
                  {t("columns.format")}
                </TableHead>
                <TableHead className="hidden w-[70px] text-right @md:table-cell">
                  {t("columns.size")}
                </TableHead>
                <TableHead className="w-[72px] text-right">Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleFiles.map((file) => (
                <TableRow
                  key={file.id}
                  className={`cursor-pointer ${
                    selectedFileIds.includes(file.id) ? "bg-accent" : ""
                  } ${activePath === file.path ? "bg-accent/70" : ""}`}
                  onClick={(e) => handleRowClick(file, e)}
                  title={file.fileName}
                >
                  <TableCell className="max-w-0 truncate font-mono text-xs">
                    {file.fileName}
                  </TableCell>
                  <TableCell className="hidden max-w-0 truncate text-xs @2xl:table-cell">
                    —
                  </TableCell>
                  <TableCell className="hidden max-w-0 truncate text-xs @3xl:table-cell">
                    —
                  </TableCell>
                  <TableCell className="hidden text-right text-xs tabular-nums @lg:table-cell">
                    {formatDuration(file.durationMs)}
                  </TableCell>
                  <TableCell className="hidden text-xs @xl:table-cell">
                    {formatAudioFormat(file.audioFormat)}
                  </TableCell>
                  <TableCell className="hidden text-right text-xs tabular-nums @md:table-cell">
                    {formatSize(file.fileSize)}
                  </TableCell>
                  <TableCell className="text-right">
                    <StatusBadge file={file} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}
