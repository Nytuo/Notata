import { create } from "zustand";
import type { MediaFile } from "@/lib/types";

/** Wall-clock time this app session began. */
const SESSION_STARTED_AT = Math.floor(Date.now() / 1000);

export type FileStatus = "modified" | "new" | "unchanged";

interface SessionState {
  sessionStartedAt: number;
  /** Paths written during this session, tracked client-side for instant feedback. */
  modifiedPaths: Set<string>;

  markModified: (path: string) => void;
  markManyModified: (paths: string[]) => void;
  clearModified: () => void;
  isModified: (file: MediaFile) => boolean;
  statusOf: (file: MediaFile) => FileStatus;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessionStartedAt: SESSION_STARTED_AT,
  modifiedPaths: new Set<string>(),

  markModified: (path: string) =>
    set((state) => {
      const next = new Set(state.modifiedPaths);
      next.add(path);
      return { modifiedPaths: next };
    }),

  markManyModified: (paths: string[]) =>
    set((state) => {
      const next = new Set(state.modifiedPaths);
      paths.forEach((p) => next.add(p));
      return { modifiedPaths: next };
    }),

  clearModified: () => set({ modifiedPaths: new Set<string>() }),

  /**
   * A file counts as modified either because this session wrote it, or because
   * the backend stamped it after the session began — the latter covers writes
   * that happened in a window this store did not observe.
   */
  isModified: (file: MediaFile) => {
    const { modifiedPaths, sessionStartedAt } = get();
    if (modifiedPaths.has(file.path)) return true;
    return (
      file.lastModifiedByApp !== null &&
      file.lastModifiedByApp >= sessionStartedAt
    );
  },

  // "Modified" wins over "new": an edit is the more actionable signal.
  statusOf: (file: MediaFile) => {
    if (get().isModified(file)) return "modified";
    if (file.isNew) return "new";
    return "unchanged";
  },
}));
