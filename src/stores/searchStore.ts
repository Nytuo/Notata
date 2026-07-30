import { create } from "zustand";
import { commands } from "@/lib/tauri";
import type { ProviderRelease, ProviderSearchResult } from "@/lib/types";

interface SearchState {
  query: string;
  artistFilter: string;
  results: ProviderSearchResult[];
  selectedResult: ProviderSearchResult | null;
  releaseDetails: ProviderRelease | null;
  isSearching: boolean;
  isLoadingDetails: boolean;

  setQuery: (query: string) => void;
  setArtistFilter: (artist: string) => void;
  searchReleases: (provider?: string) => Promise<void>;
  selectResult: (result: ProviderSearchResult) => void;
  loadReleaseDetails: (provider: string, id: string) => Promise<void>;
  clearResults: () => void;
}

export const useSearchStore = create<SearchState>((set, get) => ({
  query: "",
  artistFilter: "",
  results: [],
  selectedResult: null,
  releaseDetails: null,
  isSearching: false,
  isLoadingDetails: false,

  setQuery: (query) => set({ query }),
  setArtistFilter: (artistFilter) => set({ artistFilter }),

  searchReleases: async (provider = "musicbrainz") => {
    const { query, artistFilter } = get();
    if (!query.trim()) return;

    set({ isSearching: true, results: [], selectedResult: null, releaseDetails: null });
    try {
      const results = await commands.searchReleases(
        provider,
        query,
        artistFilter || undefined,
      );
      set({ results, isSearching: false });
    } catch {
      set({ isSearching: false });
    }
  },

  selectResult: (result) => set({ selectedResult: result }),

  loadReleaseDetails: async (provider, id) => {
    set({ isLoadingDetails: true });
    try {
      const releaseDetails = await commands.getReleaseDetails(provider, id);
      set({ releaseDetails, isLoadingDetails: false });
    } catch {
      set({ isLoadingDetails: false });
    }
  },

  clearResults: () => {
    set({
      results: [],
      selectedResult: null,
      releaseDetails: null,
      query: "",
      artistFilter: "",
    });
  },
}));
