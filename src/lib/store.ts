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
  isInjecting: boolean;
  selectedSkins: Map<number, SelectedSkin>; // Map of championId to selected skin
  setLeaguePath: (path: string) => void;
  setInjecting: (isInjecting: boolean) => void;
  selectSkin: (
    championId: number,
    skinId: number,
    chromaId?: number,
    fantome?: string
  ) => void;
  clearSelection: (championId: number) => void;
  clearAllSelections: () => void;
}

export const useGameStore = create<GameState>((set) => ({
  leaguePath: null,
  isInjecting: false,
  selectedSkins: new Map(),
  setLeaguePath: (path) => {
    set({ leaguePath: path });
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
}));
