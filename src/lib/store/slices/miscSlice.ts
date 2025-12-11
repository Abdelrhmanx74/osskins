import { StateCreator } from "zustand";

export type MiscItemType = "map" | "font" | "hud" | "misc";

export interface MiscItem {
    id: string;
    name: string;
    item_type: string;
    skin_file_path: string;
}

export interface MiscSlice {
    miscItems: Map<MiscItemType, MiscItem[]>;
    selectedMiscItems: Map<MiscItemType, string[]>;
    addMiscItem: (item: MiscItem) => void;
    removeMiscItem: (itemId: string) => void;
    setMiscItems: (items: MiscItem[]) => void;
    selectMiscItem: (type: MiscItemType, itemId: string | null) => void;
    selectMultipleMiscItems: (type: MiscItemType, itemIds: string[]) => void;
    toggleMiscItemSelection: (type: MiscItemType, itemId: string) => void;
    setSelectedMiscItems: (selections: Record<string, string[]>) => void;
}

export const createMiscSlice: StateCreator<MiscSlice> = (set) => ({
    miscItems: new Map(),
    selectedMiscItems: new Map(),
    addMiscItem: (item) => {
        set((state) => {
            const newMiscItems = new Map(state.miscItems);
            const itemType = item.item_type as MiscItemType;

            const existingItems = newMiscItems.get(itemType) ?? [];
            const storedItem = { ...item, id: String(item.id) } as MiscItem;
            const updatedItems = [...existingItems, storedItem];
            newMiscItems.set(itemType, updatedItems);

            return { miscItems: newMiscItems };
        });
    },
    removeMiscItem: (itemId) => {
        set((state) => {
            const newMiscItems = new Map(state.miscItems);
            const newSelectedMiscItems = new Map(state.selectedMiscItems);

            for (const [type, items] of newMiscItems.entries()) {
                const updatedItems = items.filter(
                    (item) => String(item.id) !== String(itemId),
                );
                if (updatedItems.length !== items.length) {
                    if (updatedItems.length === 0) {
                        newMiscItems.delete(type);
                    } else {
                        newMiscItems.set(type, updatedItems);
                    }

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

            items.forEach((item) => {
                const existingItems =
                    miscItemsMap.get(item.item_type as MiscItemType) ?? [];
                const storedItem = { ...item, id: String(item.id) } as MiscItem;
                miscItemsMap.set(item.item_type as MiscItemType, [
                    ...existingItems,
                    storedItem,
                ]);
            });

            const newSelectedMiscItems = new Map(state.selectedMiscItems);

            return {
                miscItems: miscItemsMap,
                selectedMiscItems: newSelectedMiscItems,
            };
        });
    },
    selectMiscItem: (type, itemId) => {
        set((state) => {
            const newSelectedMiscItems = new Map(state.selectedMiscItems);
            if (itemId === null) {
                newSelectedMiscItems.delete(type);
            } else {
                newSelectedMiscItems.set(type, [String(itemId)]);
            }

            return { selectedMiscItems: newSelectedMiscItems };
        });
    },
    selectMultipleMiscItems: (type, itemIds) => {
        set((state) => {
            const newSelectedMiscItems = new Map(state.selectedMiscItems);
            if (itemIds.length === 0) {
                newSelectedMiscItems.delete(type);
            } else {
                newSelectedMiscItems.set(type, itemIds.map(String));
            }

            return { selectedMiscItems: newSelectedMiscItems };
        });
    },
    setSelectedMiscItems: (selections: Record<string, string[]>) => {
        set(() => {
            const newSelectedMiscItems = new Map<MiscItemType, string[]>();

            for (const [type, itemIds] of Object.entries(selections)) {
                if (Array.isArray(itemIds)) {
                    newSelectedMiscItems.set(type as MiscItemType, itemIds.map(String));
                }
            }

            return { selectedMiscItems: newSelectedMiscItems };
        });
    },
    toggleMiscItemSelection: (type, itemId) => {
        set((state) => {
            const newSelectedMiscItems = new Map(state.selectedMiscItems);
            const normalizedId = String(itemId);
            const currentSelections = (newSelectedMiscItems.get(type) ?? []).map(
                String,
            );

            if (currentSelections.includes(normalizedId)) {
                const filtered = currentSelections.filter((id) => id !== normalizedId);
                if (filtered.length === 0) {
                    newSelectedMiscItems.delete(type);
                } else {
                    newSelectedMiscItems.set(type, filtered);
                }
            } else {
                newSelectedMiscItems.set(type, [...currentSelections, normalizedId]);
            }

            return { selectedMiscItems: newSelectedMiscItems };
        });
    },
});
