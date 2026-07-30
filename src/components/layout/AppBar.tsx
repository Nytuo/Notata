import { useState } from "react";
import { Copy, Settings, Disc3, FolderPlus, Loader2 } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import { UpdaterModal } from "@/components/common/UpdaterModal";
import { DuplicatesDialog } from "@/components/dedup/DuplicatesDialog";
import { LibraryKindDialog } from "@/components/layout/LibraryKindDialog";
import { useLibraryStore } from "@/stores/libraryStore";
import type { MediaKind } from "@/lib/types";

/**
 * Application-level actions.
 *
 * These live in a persistent full-width bar rather than inside a resizable
 * pane so they can never be clipped or wrapped out of reach — the panes only
 * hold actions scoped to their own content.
 */
export function AppBar() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [duplicatesOpen, setDuplicatesOpen] = useState(false);
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const { addFolder, isScanning } = useLibraryStore();

  const handleAddFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setPendingPath(selected);
  };

  const handleConfirmKind = async (kind: MediaKind) => {
    const path = pendingPath;
    setPendingPath(null);
    if (path) await addFolder(path, kind);
  };

  return (
    <>
      <header className="flex h-11 shrink-0 items-center gap-2 border-b px-3">
        <div className="flex items-center gap-2 pr-1">
          <Disc3 className="h-4 w-4 text-primary" />
          <span className="text-sm font-semibold tracking-tight">Notata</span>
        </div>

        <Separator orientation="vertical" className="h-5" />

        <Button
          variant="ghost"
          size="sm"
          className="h-8 gap-1.5"
          onClick={handleAddFolder}
          disabled={isScanning}
        >
          {isScanning ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <FolderPlus className="h-4 w-4" />
          )}
          <span className="hidden sm:inline">Add library</span>
        </Button>

        <div className="ml-auto flex items-center gap-1">
          <AppBarAction
            icon={Copy}
            label="Duplicates"
            hint="Find duplicate tracks across the library"
            onClick={() => setDuplicatesOpen(true)}
          />

          <Separator orientation="vertical" className="mx-1 h-5" />

          <AppBarAction
            icon={Settings}
            label="Settings"
            hint="API keys and preferences"
            onClick={() => setSettingsOpen(true)}
            iconOnly
          />
        </div>
      </header>

      <LibraryKindDialog
        path={pendingPath}
        onCancel={() => setPendingPath(null)}
        onConfirm={handleConfirmKind}
      />
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
      <DuplicatesDialog open={duplicatesOpen} onOpenChange={setDuplicatesOpen} />
      <UpdaterModal />
    </>
  );
}

function AppBarAction({
  icon: Icon,
  label,
  hint,
  onClick,
  iconOnly = false,
}: {
  icon: typeof Copy;
  label: string;
  hint: string;
  onClick: () => void;
  iconOnly?: boolean;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button variant="ghost" size="sm" className="h-8 gap-1.5" onClick={onClick}>
          <Icon className="h-4 w-4" />
          {/* The label drops on narrow windows, but the action never does. */}
          {!iconOnly && <span className="hidden md:inline">{label}</span>}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{hint}</TooltipContent>
    </Tooltip>
  );
}
