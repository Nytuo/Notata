import { create } from "zustand";
import { commands } from "@/lib/tauri";
import { useSessionStore } from "./sessionStore";
import type { BookCover, BookMetadata, BookProperties } from "@/lib/types";

interface BookMetadataState {
  currentPath: string | null;
  currentMetadata: BookMetadata | null;
  originalMetadata: BookMetadata | null;
  properties: BookProperties | null;
  cover: BookCover | null;
  isDirty: boolean;
  isLoading: boolean;
  isSaving: boolean;

  loadMetadata: (path: string) => Promise<void>;
  updateField: <K extends keyof BookMetadata>(
    field: K,
    value: BookMetadata[K],
  ) => void;
  saveMetadata: () => Promise<string>;
  revertChanges: () => void;
  clear: () => void;
}

export const useBookMetadataStore = create<BookMetadataState>((set, get) => ({
  currentPath: null,
  currentMetadata: null,
  originalMetadata: null,
  properties: null,
  cover: null,
  isDirty: false,
  isLoading: false,
  isSaving: false,

  loadMetadata: async (path: string) => {
    set({ isLoading: true, currentPath: path });
    try {
      // Cover and properties are best-effort; neither should block the editor.
      const [metadata, properties, cover] = await Promise.all([
        commands.readBookMetadata(path),
        commands.getBookProperties(path).catch(() => null),
        commands.getBookCover(path).catch(() => null),
      ]);
      set({
        currentMetadata: metadata,
        originalMetadata: structuredClone(metadata),
        properties,
        cover,
        isDirty: false,
        isLoading: false,
      });
    } catch {
      set({ isLoading: false, currentMetadata: null });
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
    if (!currentPath || !currentMetadata) throw new Error("Nothing to save");

    set({ isSaving: true });
    try {
      const entryPath = await commands.writeBookMetadata(
        currentPath,
        currentMetadata,
      );
      useSessionStore.getState().markModified(currentPath);
      const saved: BookMetadata = {
        ...currentMetadata,
        entryPath,
        source: currentMetadata.kind === "comic" ? "comic_info" : "opf",
      };
      set({
        currentMetadata: saved,
        originalMetadata: structuredClone(saved),
        isDirty: false,
        isSaving: false,
      });
      return entryPath;
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

  clear: () =>
    set({
      currentPath: null,
      currentMetadata: null,
      originalMetadata: null,
      properties: null,
      cover: null,
      isDirty: false,
    }),
}));
