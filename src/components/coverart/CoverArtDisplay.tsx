import { useTranslation } from "react-i18next";
import { ImageIcon, Pencil, Trash2, Replace } from "lucide-react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { CoverArtData } from "@/lib/types";

interface CoverArtDisplayProps {
  coverArt: CoverArtData | null;
  size?: number;
  onClick?: () => void;
  onRemove?: () => void;
}

export function CoverArtDisplay({
  coverArt,
  size = 180,
  onClick,
  onRemove,
}: CoverArtDisplayProps) {
  const { t } = useTranslation("metadata");

  const content = coverArt ? (
    <img
      src={`data:${coverArt.mimeType};base64,${coverArt.data}`}
      alt="Cover art"
      className="rounded-md border object-cover"
      style={{ width: size, height: size }}
    />
  ) : (
    <div
      className="flex flex-col items-center justify-center rounded-md border bg-muted"
      style={{ width: size, height: size }}
    >
      <ImageIcon className="h-8 w-8 text-muted-foreground" />
      <span className="mt-1 text-xs text-muted-foreground">
        {t("cover_art.no_art")}
      </span>
    </div>
  );

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          className="group relative shrink-0 cursor-pointer"
          onClick={onClick}
        >
          {content}
          <div className="absolute inset-0 flex items-center justify-center rounded-md bg-black/0 opacity-0 transition-all group-hover:bg-black/40 group-hover:opacity-100">
            <Pencil className="h-6 w-6 text-white drop-shadow" />
          </div>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={onClick}>
          <Replace className="mr-2 h-4 w-4" />
          Change cover art
        </ContextMenuItem>
        {coverArt && onRemove && (
          <ContextMenuItem onClick={onRemove} className="text-destructive">
            <Trash2 className="mr-2 h-4 w-4" />
            Remove cover art
          </ContextMenuItem>
        )}
      </ContextMenuContent>
    </ContextMenu>
  );
}
