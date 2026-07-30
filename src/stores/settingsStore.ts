import { create } from "zustand";
import { commands } from "@/lib/tauri";

/** Light/dark, or follow the OS. */
export type ThemeMode = "light" | "dark" | "system";

/** Accent palette applied on top of the mode. */
export type ThemeAccent =
  | "neutral"
  | "blue"
  | "red"
  | "green"
  | "purple"
  | "orange";

export const THEME_ACCENTS: { value: ThemeAccent; label: string; swatch: string }[] = [
  { value: "neutral", label: "Neutral", swatch: "oklch(0.55 0 0)" },
  { value: "blue", label: "Blue", swatch: "oklch(0.55 0.19 258)" },
  { value: "red", label: "Red", swatch: "oklch(0.58 0.22 27)" },
  { value: "green", label: "Green", swatch: "oklch(0.58 0.16 152)" },
  { value: "purple", label: "Purple", swatch: "oklch(0.55 0.24 300)" },
  { value: "orange", label: "Orange", swatch: "oklch(0.65 0.19 55)" },
];

export const LANGUAGES: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "fr", label: "Français" },
];

const PREF_MODE = "theme_mode";
const PREF_ACCENT = "theme_accent";
const PREF_LANGUAGE = "language";

interface SettingsState {
  mode: ThemeMode;
  accent: ThemeAccent;
  language: string;
  loaded: boolean;

  load: () => Promise<void>;
  setMode: (mode: ThemeMode) => Promise<void>;
  setAccent: (accent: ThemeAccent) => Promise<void>;
  setLanguage: (language: string) => Promise<void>;
}

/** Resolve "system" against the OS preference and stamp the document. */
function applyTheme(mode: ThemeMode, accent: ThemeAccent) {
  const root = document.documentElement;

  const prefersDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches;
  const dark = mode === "dark" || (mode === "system" && prefersDark);

  root.classList.toggle("dark", dark);
  root.dataset.accent = accent;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  mode: "system",
  accent: "neutral",
  language: "en",
  loaded: false,

  load: async () => {
    // Preferences live in SQLite; fall back to defaults before the first save.
    const [mode, accent, language] = await Promise.all([
      commands.getPreference(PREF_MODE).catch(() => null),
      commands.getPreference(PREF_ACCENT).catch(() => null),
      commands.getPreference(PREF_LANGUAGE).catch(() => null),
    ]);

    const next = {
      mode: (mode as ThemeMode) ?? "system",
      accent: (accent as ThemeAccent) ?? "neutral",
      language: language ?? "en",
      loaded: true,
    };

    applyTheme(next.mode, next.accent);
    set(next);
  },

  setMode: async (mode) => {
    applyTheme(mode, get().accent);
    set({ mode });
    await commands.setPreference(PREF_MODE, mode).catch(() => {});
  },

  setAccent: async (accent) => {
    applyTheme(get().mode, accent);
    set({ accent });
    await commands.setPreference(PREF_ACCENT, accent).catch(() => {});
  },

  setLanguage: async (language) => {
    set({ language });
    await commands.setPreference(PREF_LANGUAGE, language).catch(() => {});
  },
}));

/** Keep "system" mode honest when the OS preference changes mid-session. */
export function watchSystemTheme() {
  const media = window.matchMedia?.("(prefers-color-scheme: dark)");
  if (!media) return () => {};

  const handler = () => {
    const { mode, accent } = useSettingsStore.getState();
    if (mode === "system") applyTheme(mode, accent);
  };

  media.addEventListener("change", handler);
  return () => media.removeEventListener("change", handler);
}
