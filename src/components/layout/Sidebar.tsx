import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Trash2, RefreshCw, Library, Loader2, ChevronDown } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useLibraryStore } from "@/stores/libraryStore";
import {
  LIBRARY_KINDS,
  libraryKindIcon,
  libraryKindLabel,
} from "@/components/layout/LibraryKindDialog";

export function Sidebar() {
  const { t } = useTranslation("library");
  const {
    roots,
    currentRootId,
    isScanning,
    loadRoots,
    addFolder,
    setRootMediaKind,
    removeRoot,
    selectRoot,
  } = useLibraryStore();

  useEffect(() => {
    loadRoots();
  }, [loadRoots]);

  const handleRescan = async (root: { id: string; path: string; mediaKind: (typeof roots)[number]["mediaKind"] }) => {
    try {
      await addFolder(root.path, root.mediaKind);
      toast.success(`Rescanned ${root.path}`);
    } catch (e) {
      toast.error(`Rescan failed: ${e}`);
    }
  };

  const handleRemove = async (root: { id: string; label: string | null; path: string }) => {
    try {
      await removeRoot(root.id);
      toast.success(`Removed ${root.label || root.path} from the library`);
    } catch (e) {
      toast.error(`Could not remove library: ${e}`);
    }
  };

  const handleKindChange = async (rootId: string, kind: (typeof roots)[number]["mediaKind"]) => {
    try {
      await setRootMediaKind(rootId, kind);
    } catch (e) {
      toast.error(`Could not change library type: ${e}`);
    }
  };

  return (
    <nav className="flex h-full w-full flex-col" aria-label="Library folders">
      <div className="flex h-9 shrink-0 items-center gap-2 border-b px-3">
        <Library className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {t("title")}
        </span>
        {roots.length > 0 && (
          <span className="ml-auto text-xs tabular-nums text-muted-foreground">
            {roots.length}
          </span>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {roots.length === 0 ? (
          <div className="flex flex-col items-center gap-2 p-6 text-center">
            <Library className="h-7 w-7 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{t("no_folders")}</p>
            <p className="text-xs text-muted-foreground">
              Use “Add library” above to get started.
            </p>
          </div>
        ) : (
          <ul className="p-1">
            {roots.map((root) => {
              const KindIcon = libraryKindIcon(root.mediaKind);
              const isActive = currentRootId === root.id;
              return (
                <li key={root.id}>
                  <div
                    className={`group flex items-center gap-1.5 rounded-md px-2 py-1.5 text-sm ${
                      isActive ? "bg-accent" : "hover:bg-accent/60"
                    }`}
                  >
                    {/* The kind is both the icon and the control that changes it. */}
                    <DropdownMenu>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <DropdownMenuTrigger asChild>
                            <button
                              className="flex shrink-0 items-center rounded-sm p-0.5 text-muted-foreground hover:bg-background hover:text-foreground"
                              aria-label={`${libraryKindLabel(root.mediaKind)} library — change type`}
                            >
                              <KindIcon className="h-4 w-4" />
                              <ChevronDown className="h-2.5 w-2.5 opacity-50" />
                            </button>
                          </DropdownMenuTrigger>
                        </TooltipTrigger>
                        <TooltipContent>
                          {libraryKindLabel(root.mediaKind)} library — click to change
                        </TooltipContent>
                      </Tooltip>
                      <DropdownMenuContent align="start">
                        <DropdownMenuLabel className="text-xs">
                          Library type
                        </DropdownMenuLabel>
                        {LIBRARY_KINDS.map((k) => (
                          <DropdownMenuItem
                            key={k.value}
                            className="text-xs"
                            onClick={() => handleKindChange(root.id, k.value)}
                          >
                            <k.icon className="mr-2 h-3 w-3" />
                            {k.label}
                            {root.mediaKind === k.value && (
                              <span className="ml-auto text-muted-foreground">✓</span>
                            )}
                          </DropdownMenuItem>
                        ))}
                      </DropdownMenuContent>
                    </DropdownMenu>

                    <button
                      className="min-w-0 flex-1 truncate text-left"
                      onClick={() => selectRoot(root.id)}
                      title={root.path}
                    >
                      {root.label || root.path}
                    </button>

                    {/* Kept mounted so the row does not reflow on hover. */}
                    <div className="flex shrink-0 gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6"
                            onClick={() => handleRescan(root)}
                            disabled={isScanning}
                          >
                            {isScanning ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              <RefreshCw className="h-3 w-3" />
                            )}
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>{t("rescan")}</TooltipContent>
                      </Tooltip>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 text-destructive"
                            onClick={() => handleRemove(root)}
                          >
                            <Trash2 className="h-3 w-3" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>{t("remove_folder")}</TooltipContent>
                      </Tooltip>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </nav>
  );
}
