import { StateCreator } from "zustand";

interface SelectedSkin {
    championId: number;
    skinId: number;
    chromaId?: number;
    skin_file?: string;
}

export interface SkinSlice {
    selectedSkins: Map<number, SelectedSkin>;
    manualSelectedSkins: Map<number, SelectedSkin>;
    selectSkin: (
        championId: number,
        skinId: number,
        chromaId?: number,
        skin_file?: string,
    ) => void;
    clearSelection: (championId: number) => void;
    clearAllSelections: () => void;
    selectManualSkin: (
        championId: number,
        skinId: number,
        chromaId?: number,
        skin_file?: string,
    ) => void;
    clearManualSelection: (championId: number) => void;
}

export const createSkinSlice: StateCreator<SkinSlice> = (set) => ({
    selectedSkins: new Map(),
    manualSelectedSkins: new Map(),
    selectSkin: (championId, skinId, chromaId, skin_file) => {
        set((state) => {
            const newSelectedSkins = new Map(state.selectedSkins);
            newSelectedSkins.set(championId, {
                championId,
                skinId,
                chromaId,
                skin_file,
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
    selectManualSkin: (championId, skinId, chromaId, skin_file) => {
        set((state) => {
            const newMap = new Map(state.manualSelectedSkins);
            newMap.set(championId, {
                championId,
                skinId,
                chromaId,
                skin_file,
            });
            return { manualSelectedSkins: newMap };
        });
    },
    clearManualSelection: (championId: number) => {
        set((state) => {
            const newMap = new Map(state.manualSelectedSkins);
            newMap.delete(championId);
            return { manualSelectedSkins: newMap };
        });
    },
});
