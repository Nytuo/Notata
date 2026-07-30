import { create } from "zustand";

type Panel = "metadata" | "search" | "coverart";

interface UIState {
  sidebarCollapsed: boolean;
  activePanel: Panel;

  toggleSidebar: () => void;
  setActivePanel: (panel: Panel) => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  activePanel: "metadata",

  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setActivePanel: (activePanel) => set({ activePanel }),
}));
