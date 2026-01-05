import { StateCreator } from "zustand";

export interface SelectedSkin {
    championId: number;
    skinId: number;
    chromaId?: number;
    skin_file?: string;
}

export interface SkinSlice {
    selectedSkins: Map<number, SelectedSkin>;
    manualSelectedSkins: Map<number, SelectedSkin>;
    customSelectedSkins: Map<number, SelectedSkin[]>;
    manualCustomSelectedSkins: Map<number, SelectedSkin[]>;
    selectSkin: (
        championId: number,
        skinId: number,
        chromaId?: number,
        skin_file?: string,
    ) => void;
    clearSelection: (championId: number) => void;
    clearAllSelections: () => void;
    addCustomSkinSelection: (championId: number, skin: SelectedSkin) => void;
    removeCustomSkinSelection: (championId: number, skin_file: string) => void;
    clearCustomSelections: (championId: number) => void;
    selectManualSkin: (
        championId: number,
        skinId: number,
        chromaId?: number,
        skin_file?: string,
    ) => void;
    clearManualSelection: (championId: number) => void;
    addManualCustomSkinSelection: (championId: number, skin: SelectedSkin) => void;
    removeManualCustomSkinSelection: (championId: number, skin_file: string) => void;
    clearManualCustomSelections: (championId: number) => void;
}

export const createSkinSlice: StateCreator<SkinSlice> = (set) => ({
    selectedSkins: new Map(),
    manualSelectedSkins: new Map(),
    customSelectedSkins: new Map(),
    manualCustomSelectedSkins: new Map(),
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
    addCustomSkinSelection: (championId, skin) => {
        set((state) => {
            const map = new Map(state.customSelectedSkins);
            const existing = map.get(championId) ?? [];
            // prevent duplicates by skin_file
            if (skin.skin_file && existing.some((s) => s.skin_file === skin.skin_file)) {
                return { customSelectedSkins: map };
            }
            map.set(championId, [...existing, skin]);
            return { customSelectedSkins: map };
        });
    },
    removeCustomSkinSelection: (championId, skin_file) => {
        set((state) => {
            const map = new Map(state.customSelectedSkins);
            const existing = map.get(championId) ?? [];
            const filtered = existing.filter((s) => s.skin_file !== skin_file);
            if (filtered.length === 0) map.delete(championId);
            else map.set(championId, filtered);
            return { customSelectedSkins: map };
        });
    },
    clearCustomSelections: (championId) => {
        set((state) => {
            const map = new Map(state.customSelectedSkins);
            map.delete(championId);
            return { customSelectedSkins: map };
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
    addManualCustomSkinSelection: (championId, skin) => {
        set((state) => {
            const map = new Map(state.manualCustomSelectedSkins);
            const existing = map.get(championId) ?? [];
            if (skin.skin_file && existing.some((s) => s.skin_file === skin.skin_file)) {
                return { manualCustomSelectedSkins: map };
            }
            map.set(championId, [...existing, skin]);
            return { manualCustomSelectedSkins: map };
        });
    },
    removeManualCustomSkinSelection: (championId, skin_file) => {
        set((state) => {
            const map = new Map(state.manualCustomSelectedSkins);
            const existing = map.get(championId) ?? [];
            const filtered = existing.filter((s) => s.skin_file !== skin_file);
            if (filtered.length === 0) map.delete(championId);
            else map.set(championId, filtered);
            return { manualCustomSelectedSkins: map };
        });
    },
    clearManualCustomSelections: (championId) => {
        set((state) => {
            const map = new Map(state.manualCustomSelectedSkins);
            map.delete(championId);
            return { manualCustomSelectedSkins: map };
        });
    },
});
