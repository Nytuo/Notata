import { useState } from "react";
import {
  Pencil,
  Sparkles,
  ListChecks,
  FileText,
  X,
  Search,
  LayoutList,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useLibraryStore, type StatusFilter } from "@/stores/libraryStore";
import { useSessionStore } from "@/stores/sessionStore";
import { BatchEditDialog } from "@/components/batch/BatchEditDialog";
import { RenameDialog } from "@/components/renamer/RenameDialog";
import type { MediaFile } from "@/lib/types";

const FILTERS: {
  value: StatusFilter;
  label: string;
  icon: typeof LayoutList;
  hint: string;
}[] = [
  { value: "all", label: "All", icon: LayoutList, hint: "Show every file" },
  {
    value: "modified",
    label: "Edited",
    icon: Pencil,
    hint: "Only files modified during this session",
  },
  {
    value: "new",
    label: "New",
    icon: Sparkles,
    hint: "Only files first seen in the most recent scan",
  },
];

export function FileListToolbar({ visibleFiles }: { visibleFiles: MediaFile[] }) {
  const {
    files,
    selectedFileIds,
    statusFilter,
    searchFilter,
    setStatusFilter,
    setSearchFilter,
    selectAll,
    clearSelection,
  } = useLibraryStore();
  const statusOf = useSessionStore((s) => s.statusOf);
  const modifiedPaths = useSessionStore((s) => s.modifiedPaths);

  const [batchOpen, setBatchOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);

  // Referenced so counts recompute as edits land.
  void modifiedPaths;
  const counts: Record<StatusFilter, number> = {
    all: files.length,
    modified: files.filter((f) => statusOf(f) === "modified").length,
    new: files.filter((f) => statusOf(f) === "new").length,
  };

  const selectedPaths = files
    .filter((f) => selectedFileIds.includes(f.id))
    .map((f) => f.path);
  const hasSelection = selectedPaths.length > 0;

  return (
    <>
      {/* @container keys the responsive rules to this pane's width, not the
          window's, so narrowing the divider degrades gracefully. */}
      <div className="@container shrink-0 border-b">
        <div className="flex items-center gap-1.5 px-2 py-1.5">
          <div
            className="flex shrink-0 items-center gap-0.5 rounded-md bg-muted p-0.5"
            role="group"
            aria-label="Filter files by status"
          >
            {FILTERS.map((f) => {
              const Icon = f.icon;
              const active = statusFilter === f.value;
              return (
                <Tooltip key={f.value}>
                  <TooltipTrigger asChild>
                    <button
                      onClick={() => setStatusFilter(f.value)}
                      aria-pressed={active}
                      className={`flex h-6 items-center gap-1 rounded px-1.5 text-xs transition-colors ${
                        active
                          ? "bg-background shadow-sm"
                          : "text-muted-foreground hover:text-foreground"
                      }`}
                    >
                      <Icon className="h-3 w-3 shrink-0" />
                      {/* Label appears once the pane can afford it; the
                          control itself is always reachable. */}
                      <span className="hidden @md:inline">{f.label}</span>
                      <span className="tabular-nums opacity-60">
                        {counts[f.value]}
                      </span>
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>{f.hint}</TooltipContent>
                </Tooltip>
              );
            })}
          </div>

          <div className="relative min-w-0 flex-1">
            <Search className="pointer-events-none absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="Filter by name"
              value={searchFilter}
              onChange={(e) => setSearchFilter(e.target.value)}
              className="h-7 pl-7 text-xs"
              aria-label="Filter files by name"
            />
          </div>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="icon"
                variant="ghost"
                className="h-7 w-7 shrink-0"
                onClick={() => selectAll(visibleFiles.map((f) => f.id))}
                disabled={visibleFiles.length === 0}
                aria-label={`Select all ${visibleFiles.length} files`}
              >
                <ListChecks className="h-3.5 w-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              Select all {visibleFiles.length} shown
            </TooltipContent>
          </Tooltip>
        </div>

        {/* Selection actions get their own row so they never compete for
            space with the filters, at any pane width. */}
        {hasSelection && (
          <div className="flex items-center gap-1.5 border-t bg-accent/40 px-2 py-1.5">
            <Badge variant="secondary" className="shrink-0 tabular-nums">
              {selectedPaths.length}
            </Badge>
            <span className="hidden shrink-0 text-xs text-muted-foreground @sm:inline">
              selected
            </span>

            <div className="ml-auto flex items-center gap-1">
              <Button
                size="sm"
                variant="outline"
                className="h-7 shrink-0 gap-1 px-2 text-xs"
                onClick={() => setBatchOpen(true)}
              >
                <Pencil className="h-3 w-3" />
                <span className="hidden @md:inline">Batch edit</span>
              </Button>
              <Button
                size="sm"
                variant="outline"
                className="h-7 shrink-0 gap-1 px-2 text-xs"
                onClick={() => setRenameOpen(true)}
              >
                <FileText className="h-3 w-3" />
                <span className="hidden @md:inline">Rename</span>
              </Button>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7 shrink-0"
                    onClick={clearSelection}
                    aria-label="Clear selection"
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Clear selection</TooltipContent>
              </Tooltip>
            </div>
          </div>
        )}
      </div>

      <BatchEditDialog
        open={batchOpen}
        onOpenChange={setBatchOpen}
        paths={selectedPaths}
      />
      <RenameDialog
        open={renameOpen}
        onOpenChange={setRenameOpen}
        paths={selectedPaths}
      />
    </>
  );
}
