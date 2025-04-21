import { create } from "zustand";
import { Champion, Skin } from "./hooks/use-champions";

interface SelectedSkin {
  championId: number;
  skinId: number;
  chromaId?: number;
  fantome?: string; // Add fantome path
}

// Define the possible injection statuses
export type InjectionStatus = "idle" | "injecting" | "success" | "error";

interface GameState {
  leaguePath: string | null;
  lcuStatus: string | null;
  injectionStatus: InjectionStatus; // Add this
  selectedSkins: Map<number, SelectedSkin>;
  favorites: Set<number>;
  setLeaguePath: (path: string) => void;
  setLcuStatus: (status: string) => void;
  setInjectionStatus: (status: InjectionStatus) => void; // Add this
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
  lcuStatus: null,
  injectionStatus: "idle", // Default status
  selectedSkins: new Map(),
  favorites: new Set(),
  setLeaguePath: (path) => {
    set({ leaguePath: path });
  },
  setLcuStatus: (status) => {
    set({ lcuStatus: status });
  },
  setInjectionStatus: (status) => {
    // Add implementation
    set({ injectionStatus: status });
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
      return { favorites: newFavorites };
    });
  },
  setFavorites: (favorites) => {
    set({ favorites });
  },
}));
