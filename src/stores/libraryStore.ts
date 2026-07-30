import { create } from "zustand";
import { commands } from "@/lib/tauri";
import type {
  LibraryRoot,
  MediaFile,
  MediaKind,
  ScanProgress,
} from "@/lib/types";

/** Which subset of the current directory the file list shows. */
export type StatusFilter = "all" | "modified" | "new";

interface LibraryState {
  roots: LibraryRoot[];
  files: MediaFile[];
  selectedFileIds: string[];
  currentDirectory: string | null;
  currentRootId: string | null;
  scanProgress: ScanProgress | null;
  isScanning: boolean;
  statusFilter: StatusFilter;
  searchFilter: string;

  loadRoots: () => Promise<void>;
  addFolder: (path: string, mediaKind: MediaKind) => Promise<void>;
  setRootMediaKind: (rootId: string, mediaKind: MediaKind) => Promise<void>;
  removeRoot: (id: string) => Promise<void>;
  selectDirectory: (path: string) => Promise<void>;
  selectRoot: (rootId: string) => Promise<void>;
  refreshFiles: () => Promise<void>;
  selectFiles: (ids: string[]) => void;
  toggleFileSelection: (id: string) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  setStatusFilter: (filter: StatusFilter) => void;
  setSearchFilter: (query: string) => void;
  setScanProgress: (progress: ScanProgress | null) => void;
  setIsScanning: (scanning: boolean) => void;
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  roots: [],
  files: [],
  selectedFileIds: [],
  currentDirectory: null,
  currentRootId: null,
  scanProgress: null,
  isScanning: false,
  statusFilter: "all",
  searchFilter: "",

  loadRoots: async () => {
    const roots = await commands.getLibraryRoots();
    set({ roots });
  },

  addFolder: async (path: string, mediaKind: MediaKind) => {
    set({ isScanning: true });
    try {
      await commands.scanDirectory(path, mediaKind);
      await get().loadRoots();
    } finally {
      set({ isScanning: false, scanProgress: null });
    }
  },

  setRootMediaKind: async (rootId: string, mediaKind: MediaKind) => {
    await commands.setRootMediaKind(rootId, mediaKind);
    await get().loadRoots();
  },

  removeRoot: async (id: string) => {
    await commands.removeLibraryRoot(id);
    const { currentRootId } = get();
    if (currentRootId === id) {
      set({ files: [], selectedFileIds: [], currentRootId: null, currentDirectory: null });
    }
    await get().loadRoots();
  },

  selectDirectory: async (path: string) => {
    const files = await commands.getFilesInDirectory(path);
    set({ files, currentDirectory: path, selectedFileIds: [] });
  },

  selectRoot: async (rootId: string) => {
    const files = await commands.getFilesByRoot(rootId);
    set({ files, currentRootId: rootId, currentDirectory: null, selectedFileIds: [] });
  },

  /** Re-read the current view so status flags reflect the latest writes. */
  refreshFiles: async () => {
    const { currentDirectory, currentRootId } = get();
    if (currentDirectory) {
      set({ files: await commands.getFilesInDirectory(currentDirectory) });
    } else if (currentRootId) {
      set({ files: await commands.getFilesByRoot(currentRootId) });
    }
  },

  selectFiles: (ids: string[]) => set({ selectedFileIds: ids }),

  toggleFileSelection: (id: string) => {
    const { selectedFileIds } = get();
    if (selectedFileIds.includes(id)) {
      set({ selectedFileIds: selectedFileIds.filter((i) => i !== id) });
    } else {
      set({ selectedFileIds: [...selectedFileIds, id] });
    }
  },

  selectAll: (ids: string[]) => set({ selectedFileIds: ids }),
  clearSelection: () => set({ selectedFileIds: [] }),

  setStatusFilter: (filter) => set({ statusFilter: filter }),
  setSearchFilter: (query) => set({ searchFilter: query }),

  setScanProgress: (progress) => set({ scanProgress: progress }),
  setIsScanning: (scanning) => set({ isScanning: scanning }),
}));
