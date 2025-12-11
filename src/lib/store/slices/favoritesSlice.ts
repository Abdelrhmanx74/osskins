import { StateCreator } from "zustand";

export interface FavoritesSlice {
    favorites: Set<number>;
    toggleFavorite: (championId: number) => void;
    setFavorites: (favorites: Set<number>) => void;
}

export const createFavoritesSlice: StateCreator<FavoritesSlice> = (set) => ({
    favorites: new Set(),
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
});
