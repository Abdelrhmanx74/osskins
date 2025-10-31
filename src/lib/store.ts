import { create } from "zustand";
import { CustomSkin } from "./types";
// import { Champion, Skin } from "./hooks/use-champions";

interface SelectedSkin {
  championId: number;
  skinId: number;
  chromaId?: number;
  fantome?: string; // Add fantome path
}

// Define the possible injection statuses
export type InjectionStatus = "idle" | "injecting" | "success" | "error";

// Custom skin tabs
export type SkinTab = "official" | "custom";

// Misc items types
export type MiscItemType = "map" | "font" | "hud" | "misc";

export interface MiscItem {
  id: string;
  name: string;
  item_type: string; // "map", "font", "blocks", "settings"
  fantome_path: string;
}

interface GameState {
  leaguePath: string | null;
  lcuStatus: string | null;
  injectionStatus: InjectionStatus; // Add this
  selectedSkins: Map<number, SelectedSkin>;
  favorites: Set<number>;
  hasCompletedOnboarding: boolean;
  activeTab: SkinTab;
  customSkins: Map<number, CustomSkin[]>;
  miscItems: Map<MiscItemType, MiscItem[]>;
  selectedMiscItems: Map<MiscItemType, string[]>; // Multiple selected misc items per type
  showUpdateModal: boolean;
  setShowUpdateModal: (v: boolean) => void;
  setLeaguePath: (path: string) => void;
  setLcuStatus: (status: string) => void;
  setInjectionStatus: (status: InjectionStatus) => void; // Add this
  // Manual injection mode state
  manualInjectionMode: boolean;
  setManualInjectionMode: (v: boolean) => void;
  manualSelectedSkins: Map<number, SelectedSkin>; // championId -> selection
  selectManualSkin: (
    championId: number,
    skinId: number,
    chromaId?: number,
    fantome?: string,
  ) => void;
  clearManualSelection: (championId: number) => void;
  selectSkin: (
    championId: number,
    skinId: number,
    chromaId?: number,
    fantome?: string,
  ) => void;
  clearSelection: (championId: number) => void;
  clearAllSelections: () => void;
  toggleFavorite: (championId: number) => void;
  setFavorites: (favorites: Set<number>) => void;
  setHasCompletedOnboarding: (completed: boolean) => void;
  setActiveTab: (tab: SkinTab) => void;
  addCustomSkin: (skin: CustomSkin) => void;
  removeCustomSkin: (skinId: string) => void;
  setCustomSkins: (skins: CustomSkin[]) => void;
  addMiscItem: (item: MiscItem) => void;
  removeMiscItem: (itemId: string) => void;
  setMiscItems: (items: MiscItem[]) => void;
  selectMiscItem: (type: MiscItemType, itemId: string | null) => void;
  selectMultipleMiscItems: (type: MiscItemType, itemIds: string[]) => void;
  toggleMiscItemSelection: (type: MiscItemType, itemId: string) => void;
  setSelectedMiscItems: (selections: Record<string, string[]>) => void;
}

export const useGameStore = create<GameState>((set) => ({
  leaguePath: null,
  lcuStatus: null,
  injectionStatus: "idle", // Default status
  selectedSkins: new Map(),
  favorites: new Set(),
  hasCompletedOnboarding: false,
  activeTab: "official", // Default to official skins tab
  customSkins: new Map(),
  miscItems: new Map(),
  selectedMiscItems: new Map(),
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
  // Manual injection mode controls
  manualInjectionMode: false,
  setManualInjectionMode: (v: boolean) => {
    set({ manualInjectionMode: v });
  },
  manualSelectedSkins: new Map(),
  selectManualSkin: (championId, skinId, chromaId, fantome) => {
    set((state) => {
      const newMap = new Map(state.manualSelectedSkins);
      newMap.set(championId, {
        championId,
        skinId,
        chromaId,
        fantome,
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
  setHasCompletedOnboarding: (completed) => {
    set({ hasCompletedOnboarding: completed });
    if (typeof window !== "undefined") {
      localStorage.setItem("hasCompletedOnboarding", completed.toString());
    }
  },
  // Control whether the data-update modal should be shown (triggered after selecting directory)
  showUpdateModal: false,
  setShowUpdateModal: (v: boolean) => {
    set(() => ({ showUpdateModal: v }));
  },
  setActiveTab: (tab) => {
    set({ activeTab: tab });
    if (typeof window !== "undefined") {
      localStorage.setItem("activeSkinsTab", tab);
    }
  },
  addCustomSkin: (skin) => {
    set((state) => {
      const newCustomSkins = new Map(state.customSkins);
      const championId = skin.champion_id;
      const existingSkins = newCustomSkins.get(championId) ?? [];
      newCustomSkins.set(championId, [...existingSkins, skin]);
      return { customSkins: newCustomSkins };
    });
  },
  removeCustomSkin: (skinId) => {
    set((state) => {
      const newCustomSkins = new Map(state.customSkins);

      // Find which champion has this skin
      for (const [championId, skins] of newCustomSkins.entries()) {
        const updatedSkins = skins.filter((skin) => skin.id !== skinId);

        if (updatedSkins.length !== skins.length) {
          // We found and removed the skin
          if (updatedSkins.length === 0) {
            newCustomSkins.delete(championId);
          } else {
            newCustomSkins.set(championId, updatedSkins);
          }
          break;
        }
      }

      return { customSkins: newCustomSkins };
    });
  },
  setCustomSkins: (skins) => {
    set(() => {
      const customSkinsMap = new Map<number, CustomSkin[]>();

      // Group skins by champion ID
      skins.forEach((skin) => {
        const championId = skin.champion_id;
        const existingSkins = customSkinsMap.get(championId) ?? [];
        customSkinsMap.set(championId, [...existingSkins, skin]);
      });

      return { customSkins: customSkinsMap };
    });
  },
  addMiscItem: (item) => {
    set((state) => {
      const newMiscItems = new Map(state.miscItems);
      const itemType = item.item_type as MiscItemType;

      const existingItems = newMiscItems.get(itemType) ?? [];
      // Normalize stored item ids to strings to avoid mismatches with selection ids
      const storedItem = { ...item, id: String(item.id) } as MiscItem;
      const updatedItems = [...existingItems, storedItem];
      newMiscItems.set(itemType, updatedItems);

      // Do not modify selectedMiscItems here; selection is managed by upload handlers.
      return {
        miscItems: newMiscItems,
      };
    });
  },
  removeMiscItem: (itemId) => {
    set((state) => {
      const newMiscItems = new Map(state.miscItems);
      const newSelectedMiscItems = new Map(state.selectedMiscItems);

      // Find and remove the item from the appropriate type
      for (const [type, items] of newMiscItems.entries()) {
        // Ensure comparison is done using strings (backend may return numbers)
        const updatedItems = items.filter(
          (item) => String(item.id) !== String(itemId),
        );
        if (updatedItems.length !== items.length) {
          // We found and removed the item
          if (updatedItems.length === 0) {
            newMiscItems.delete(type);
          } else {
            newMiscItems.set(type, updatedItems);
          }

          // If this was a selected item, remove it from the selection
          const currentSelections = (newSelectedMiscItems.get(type) ?? []).map(
            String,
          );
          const updatedSelections = currentSelections.filter(
            (id) => id !== String(itemId),
          );
          if (updatedSelections.length === 0) {
            newSelectedMiscItems.delete(type);
          } else if (updatedSelections.length !== currentSelections.length) {
            newSelectedMiscItems.set(type, updatedSelections);
          }
          break;
        }
      }

      return {
        miscItems: newMiscItems,
        selectedMiscItems: newSelectedMiscItems,
      };
    });
  },
  setMiscItems: (items) => {
    set((state) => {
      const miscItemsMap = new Map<MiscItemType, MiscItem[]>();

      // Group items by type
      // Normalize item ids to strings when populating the map
      items.forEach((item) => {
        const existingItems =
          miscItemsMap.get(item.item_type as MiscItemType) ?? [];
        const storedItem = { ...item, id: String(item.id) } as MiscItem;
        miscItemsMap.set(item.item_type as MiscItemType, [
          ...existingItems,
          storedItem,
        ]);
      });

      // Load saved selections from backend config instead of localStorage
      const newSelectedMiscItems = new Map(state.selectedMiscItems);

      // For initial load, selections will be loaded from config.json via the persistence hook
      // We don't need to load from localStorage anymore since everything goes through config.json

      return {
        miscItems: miscItemsMap,
        selectedMiscItems: newSelectedMiscItems,
      };
    });
  },
  selectMiscItem: (type, itemId) => {
    set((state) => {
      const newSelectedMiscItems = new Map(state.selectedMiscItems);
      // Debug
      try {
        console.debug("store.selectMiscItem called", {
          type,
          itemId,
          before: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }
      if (itemId === null) {
        newSelectedMiscItems.delete(type);
      } else {
        // For backward compatibility, selecting a single item replaces all selections for that type
        newSelectedMiscItems.set(type, [String(itemId)]);
      }
      try {
        console.debug("store.selectMiscItem result", {
          type,
          after: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }

      return { selectedMiscItems: newSelectedMiscItems };
    });
  },
  selectMultipleMiscItems: (type, itemIds) => {
    set((state) => {
      const newSelectedMiscItems = new Map(state.selectedMiscItems);
      try {
        console.debug("store.selectMultipleMiscItems called", {
          type,
          itemIds,
          before: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }
      if (itemIds.length === 0) {
        newSelectedMiscItems.delete(type);
      } else {
        newSelectedMiscItems.set(type, itemIds.map(String));
      }
      try {
        console.debug("store.selectMultipleMiscItems result", {
          type,
          after: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }

      return { selectedMiscItems: newSelectedMiscItems };
    });
  },
  setSelectedMiscItems: (selections: Record<string, string[]>) => {
    set(() => {
      const newSelectedMiscItems = new Map<MiscItemType, string[]>();

      for (const [type, itemIds] of Object.entries(selections)) {
        if (Array.isArray(itemIds)) {
          // Normalize IDs to strings to avoid type mismatches (backend may store numbers)
          newSelectedMiscItems.set(type as MiscItemType, itemIds.map(String));
        }
      }

      try {
        console.debug("store.setSelectedMiscItems", {
          selections,
          result: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }

      return { selectedMiscItems: newSelectedMiscItems };
    });
  },
  toggleMiscItemSelection: (type, itemId) => {
    set((state) => {
      const newSelectedMiscItems = new Map(state.selectedMiscItems);
      // Normalize stored ids to strings to avoid mismatches between number/string ids
      const normalizedId = String(itemId);
      const currentSelections = (newSelectedMiscItems.get(type) ?? []).map(
        String,
      );
      try {
        console.debug("store.toggleMiscItemSelection called", {
          type,
          itemId: normalizedId,
          before: currentSelections,
        });
      } catch (e) {
        console.error(e);
      }

      if (currentSelections.includes(normalizedId)) {
        // Remove if already selected
        const filtered = currentSelections.filter((id) => id !== normalizedId);
        if (filtered.length === 0) {
          newSelectedMiscItems.delete(type);
        } else {
          newSelectedMiscItems.set(type, filtered);
        }
      } else {
        // Add to selection
        newSelectedMiscItems.set(type, [...currentSelections, normalizedId]);
      }

      try {
        console.debug("store.toggleMiscItemSelection result", {
          type,
          after: Array.from(newSelectedMiscItems.entries()),
        });
      } catch (e) {
        console.error(e);
      }

      return { selectedMiscItems: newSelectedMiscItems };
    });
  },
}));
