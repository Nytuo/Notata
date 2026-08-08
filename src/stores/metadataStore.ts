import { create } from "zustand";
import { toast } from "sonner";
import { commands } from "@/lib/tauri";
import { useSessionStore } from "./sessionStore";
import type { AudioProperties, CoverArtData, TrackMetadata } from "@/lib/types";

interface MetadataState {
  currentPath: string | null;
  currentMetadata: TrackMetadata | null;
  originalMetadata: TrackMetadata | null;
  audioProperties: AudioProperties | null;
  coverArt: CoverArtData | null;
  isDirty: boolean;
  isLoading: boolean;
  isSaving: boolean;

  loadMetadata: (path: string) => Promise<void>;
  updateField: <K extends keyof TrackMetadata>(
    field: K,
    value: TrackMetadata[K],
  ) => void;
  saveMetadata: () => Promise<void>;
  revertChanges: () => void;
  applyFromProvider: (metadata: TrackMetadata) => void;
  clear: () => void;
}

export const useMetadataStore = create<MetadataState>((set, get) => ({
  currentPath: null,
  currentMetadata: null,
  originalMetadata: null,
  audioProperties: null,
  coverArt: null,
  isDirty: false,
  isLoading: false,
  isSaving: false,

  loadMetadata: async (path: string) => {
    set({ isLoading: true, currentPath: path });
    try {
      const [metadata, properties, coverArt] = await Promise.all([
        commands.readMetadata(path),
        commands.getAudioProperties(path),
        commands.getEmbeddedCoverArt(path),
      ]);
      set({
        currentMetadata: metadata,
        originalMetadata: structuredClone(metadata),
        audioProperties: properties,
        coverArt,
        isDirty: false,
        isLoading: false,
      });
    } catch (e) {
      set({ isLoading: false });
      toast.error(`Could not read metadata: ${e}`);
    }
  },

  updateField: (field, value) => {
    const { currentMetadata } = get();
    if (!currentMetadata) return;
    const updated = { ...currentMetadata, [field]: value };
    set({ currentMetadata: updated, isDirty: true });
  },

  saveMetadata: async () => {
    const { currentPath, currentMetadata } = get();
    if (!currentPath || !currentMetadata) return;
    set({ isSaving: true });
    try {
      await commands.writeMetadata(currentPath, currentMetadata);
      useSessionStore.getState().markModified(currentPath);
      set({
        originalMetadata: structuredClone(currentMetadata),
        isDirty: false,
        isSaving: false,
      });
    } catch {
      set({ isSaving: false });
      throw new Error("Failed to save metadata");
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

  applyFromProvider: (metadata: TrackMetadata) => {
    set({ currentMetadata: metadata, isDirty: true });
  },

  clear: () => {
    set({
      currentPath: null,
      currentMetadata: null,
      originalMetadata: null,
      audioProperties: null,
      coverArt: null,
      isDirty: false,
    });
  },
}));
