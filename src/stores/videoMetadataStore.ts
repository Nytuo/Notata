import { create } from "zustand";
import { toast } from "sonner";
import { commands } from "@/lib/tauri";
import { useSessionStore } from "./sessionStore";
import type {
  VideoArtwork,
  VideoMetadata,
  VideoProperties,
} from "@/lib/types";

interface VideoMetadataState {
  currentPath: string | null;
  currentMetadata: VideoMetadata | null;
  originalMetadata: VideoMetadata | null;
  properties: VideoProperties | null;
  artwork: VideoArtwork[];
  isDirty: boolean;
  isLoading: boolean;
  isSaving: boolean;

  loadMetadata: (path: string) => Promise<void>;
  updateField: <K extends keyof VideoMetadata>(
    field: K,
    value: VideoMetadata[K],
  ) => void;
  saveMetadata: () => Promise<string>;
  revertChanges: () => void;
  applyFromProvider: (metadata: VideoMetadata) => void;
  setArtwork: (artwork: VideoArtwork[]) => void;
  clear: () => void;
}

export const useVideoMetadataStore = create<VideoMetadataState>((set, get) => ({
  currentPath: null,
  currentMetadata: null,
  originalMetadata: null,
  properties: null,
  artwork: [],
  isDirty: false,
  isLoading: false,
  isSaving: false,

  loadMetadata: async (path: string) => {
    set({ isLoading: true, currentPath: path });
    try {
      // Artwork and properties are best-effort; neither should block the editor.
      const [metadata, properties, artwork] = await Promise.all([
        commands.readVideoMetadata(path),
        commands.getVideoProperties(path).catch(() => null),
        commands.getVideoArtwork(path).catch(() => []),
      ]);
      set({
        currentMetadata: metadata,
        originalMetadata: structuredClone(metadata),
        properties,
        artwork,
        isDirty: false,
        isLoading: false,
      });
    } catch (e) {
      set({ isLoading: false, currentMetadata: null });
      toast.error(`Could not read metadata: ${e}`);
    }
  },

  updateField: (field, value) => {
    const { currentMetadata } = get();
    if (!currentMetadata) return;
    set({
      currentMetadata: { ...currentMetadata, [field]: value },
      isDirty: true,
    });
  },

  saveMetadata: async () => {
    const { currentPath, currentMetadata } = get();
    if (!currentPath || !currentMetadata) {
      throw new Error("Nothing to save");
    }
    set({ isSaving: true });
    try {
      const nfoPath = await commands.writeVideoMetadata(
        currentPath,
        currentMetadata,
      );
      useSessionStore.getState().markModified(currentPath);
      // Record where it landed so the next save updates the same sidecar.
      const saved = { ...currentMetadata, nfoPath, source: "nfo" as const };
      set({
        currentMetadata: saved,
        originalMetadata: structuredClone(saved),
        isDirty: false,
        isSaving: false,
      });
      return nfoPath;
    } catch (e) {
      set({ isSaving: false });
      throw e;
    }
  },

  revertChanges: () => {
    const { originalMetadata } = get();
    if (!originalMetadata) return;
    set({
      currentMetadata: structuredClone(originalMetadata),
      isDirty: false,
    });
  },

  applyFromProvider: (metadata: VideoMetadata) =>
    set({ currentMetadata: metadata, isDirty: true }),

  setArtwork: (artwork: VideoArtwork[]) => set({ artwork }),

  clear: () =>
    set({
      currentPath: null,
      currentMetadata: null,
      originalMetadata: null,
      properties: null,
      artwork: [],
      isDirty: false,
    }),
}));
