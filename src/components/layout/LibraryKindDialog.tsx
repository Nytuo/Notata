import { Music, Film, Tv, Library } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { MediaKind } from "@/lib/types";

export const LIBRARY_KINDS: {
  value: MediaKind;
  label: string;
  blurb: string;
  icon: typeof Music;
}[] = [
  {
    value: "music",
    label: "Music",
    blurb: "Tracks and albums — edits write embedded audio tags.",
    icon: Music,
  },
  {
    value: "movies",
    label: "Movies",
    blurb: "Feature films — edits write NFO sidecars and posters.",
    icon: Film,
  },
  {
    value: "series",
    label: "TV Series",
    blurb: "Episodes by season — edits write NFO sidecars and posters.",
    icon: Tv,
  },
  {
    value: "books",
    label: "Comics & Books",
    blurb:
      "CBZ and EPUB — edits write ComicInfo.xml or the OPF inside the file.",
    icon: Library,
  },
];

export function libraryKindIcon(kind: MediaKind) {
  return LIBRARY_KINDS.find((k) => k.value === kind)?.icon ?? Music;
}

export function libraryKindLabel(kind: MediaKind) {
  return LIBRARY_KINDS.find((k) => k.value === kind)?.label ?? "Music";
}

/**
 * Asked once, before the first scan, because the answer decides which metadata
 * workflow every file under the folder gets.
 */
export function LibraryKindDialog({
  path,
  onConfirm,
  onCancel,
}: {
  path: string | null;
  onConfirm: (kind: MediaKind) => void;
  onCancel: () => void;
}) {
  return (
    <Dialog open={path !== null} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>What kind of library is this?</DialogTitle>
          <DialogDescription className="break-all font-mono text-xs">
            {path}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-2">
          {LIBRARY_KINDS.map((k) => (
            <button
              key={k.value}
              className="flex w-full items-start gap-3 rounded-md border p-3 text-left transition-colors hover:border-primary hover:bg-accent"
              onClick={() => onConfirm(k.value)}
            >
              <k.icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
              <div>
                <p className="text-sm font-medium">{k.label}</p>
                <p className="text-xs text-muted-foreground">{k.blurb}</p>
              </div>
            </button>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}
