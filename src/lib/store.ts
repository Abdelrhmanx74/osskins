import { create } from "zustand";
import { Champion, Skin } from "./hooks/use-champions";

interface SelectedSkin {
  championId: number;
  skinId: number;
  chromaId?: number;
  fantome?: string; // Add fantome path
}

interface GameState {
  leaguePath: string | null;
  lcuStatus: string | null; // add LCU status
  isInjecting: boolean;
  selectedSkins: Map<number, SelectedSkin>; // Map of championId to selected skin
  favorites: Set<number>;
  setLeaguePath: (path: string) => void;
  setLcuStatus: (status: string) => void; // add setter
  setInjecting: (isInjecting: boolean) => void;
  selectSkin: (
    championId: number,
    skinId: number,
    chromaId?: number,
    fantome?: string
  ) => void;
  clearSelection: (championId: number) => void;
  clearAllSelections: () => void;
  toggleFavorite: (championId: number) => void;
  setFavorites: (favorites: Set<number>) => void;
}

export const useGameStore = create<GameState>((set) => ({
  leaguePath: null,
  lcuStatus: null, // default no status
  isInjecting: false,
  selectedSkins: new Map(),
  favorites: new Set(),
  setLeaguePath: (path) => {
    set({ leaguePath: path });
  },
  setLcuStatus: (status) => {
    // implementation
    set({ lcuStatus: status });
  },
  setInjecting: (isInjecting) => {
    set({ isInjecting });
  },
  selectSkin: (championId, skinId, chromaId, fantome) => {
    set((state) => {
      const newSelectedSkins = new Map(state.selectedSkins);
      newSelectedSkins.set(championId, {
        championId,
        skinId,
        chromaId,
        fantome,
      });
      return { selectedSkins: newSelectedSkins };
    });
  },
  clearSelection: (championId) => {
    set((state) => {
      const newSelectedSkins = new Map(state.selectedSkins);
      newSelectedSkins.delete(championId);
      return { selectedSkins: newSelectedSkins };
    });
  },
  clearAllSelections: () => {
    set({ selectedSkins: new Map() });
  },
  toggleFavorite: (championId) => {
    set((state) => {
      const newFavorites = new Set(state.favorites);
      if (newFavorites.has(championId)) {
        newFavorites.delete(championId);
      } else {
        newFavorites.add(championId);
      }
      // Persist to localStorage
      if (typeof window !== "undefined") {
        localStorage.setItem(
          "championFavorites",
          JSON.stringify(Array.from(newFavorites))
        );
      }
      return { favorites: newFavorites };
    });
  },
  setFavorites: (favorites) => {
    set({ favorites });
  },
}));
