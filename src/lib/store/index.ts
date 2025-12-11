import { create } from "zustand";
import { createSkinSlice, SkinSlice } from "./slices/skinSlice";
import { createUISlice, UISlice } from "./slices/uiSlice";
import { createFavoritesSlice, FavoritesSlice } from "./slices/favoritesSlice";
import { createGameSlice, GameSlice } from "./slices/gameSlice";
import {
    createCustomSkinsSlice,
    CustomSkinsSlice,
} from "./slices/customSkinsSlice";
import { createMiscSlice, MiscSlice, MiscItemType } from "./slices/miscSlice";

// Re-export types for backward compatibility
export type { InjectionStatus } from "./slices/gameSlice";
export type { SkinTab } from "./slices/uiSlice";
export type { MiscItemType, MiscItem } from "./slices/miscSlice";

export type GameState = SkinSlice &
    UISlice &
    FavoritesSlice &
    GameSlice &
    CustomSkinsSlice &
    MiscSlice;

export const useGameStore = create<GameState>()((...a) => ({
    ...createSkinSlice(...a),
    ...createUISlice(...a),
    ...createFavoritesSlice(...a),
    ...createGameSlice(...a),
    ...createCustomSkinsSlice(...a),
    ...createMiscSlice(...a),
}));

// Optimized selectors to prevent unnecessary re-renders
export const selectSelectedSkin = (championId: number) => (state: GameState) =>
    state.selectedSkins.get(championId);

export const selectIsFavorite = (championId: number) => (state: GameState) =>
    state.favorites.has(championId);

export const selectCustomSkinsForChampion =
    (championId: number | null) => (state: GameState) => {
        if (championId === null) return [];
        return state.customSkins.get(championId) ?? [];
    };

export const selectMiscItemsForType =
    (type: MiscItemType) => (state: GameState) =>
        state.miscItems.get(type) ?? [];

export const selectSelectedMiscItems =
    (type: MiscItemType) => (state: GameState) =>
        state.selectedMiscItems.get(type) ?? [];
