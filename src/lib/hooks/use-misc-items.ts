import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MiscItem, MiscItemType, useGameStore } from "@/lib/store";
import { toast } from "sonner";

export function useMiscItems() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const {
    miscItems,
    setMiscItems,
    addMiscItem,
    removeMiscItem,
    selectMultipleMiscItems,
    selectMiscItem,
  } = useGameStore();

  // Load misc items from backend
  const loadMiscItems = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const items = await invoke<MiscItem[]>("get_misc_items");
      setMiscItems(items);
    } catch (err) {
      console.error("Failed to load misc items:", err);
      setError(
        err instanceof Error ? err.message : "Failed to load misc items"
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Upload a new misc item
  const uploadMiscItem = async (
    name: string,
    type: MiscItemType
  ): Promise<boolean> => {
    try {
      const request = {
        name,
        item_type: type,
      };

      const newItem = await invoke<MiscItem>("upload_misc_item", { request });
      addMiscItem(newItem);
      // For single upload, auto-select the new item appropriately
      if (type === "misc") {
        // append to existing selections
        const current =
          useGameStore.getState().selectedMiscItems.get(type) ?? [];
        useGameStore
          .getState()
          .selectMultipleMiscItems(type, [
            ...current.map(String),
            String(newItem.id),
          ]);
      } else {
        useGameStore.getState().selectMiscItem(type, String(newItem.id));
      }
      return true;
    } catch (err) {
      console.error("Failed to upload misc item:", err);
      const errorMessage =
        err instanceof Error ? err.message : "Failed to upload misc item";
      toast.error(errorMessage);
      return false;
    }
  };

  // Upload multiple misc items
  const uploadMultipleMiscItems = async (
    type: MiscItemType
  ): Promise<boolean> => {
    try {
      const newItems = await invoke<MiscItem[]>("upload_multiple_misc_items", {
        itemType: type,
      });

      // Add each new item to the store
      newItems.forEach((item) => {
        addMiscItem(item);
      });

      // Auto-select behavior:
      // - For 'misc' type (multi-select): preserve existing selections, add newly uploaded ids.
      //   If there are no existing selections, select only the newly uploaded items.
      // - For map/font/hud enforce single-select: choose the newest uploaded item.
      const newItemIds = newItems.map((item) => String(item.id));

      if (type === "misc") {
        const currentSelections =
          useGameStore.getState().selectedMiscItems.get(type) ?? [];
        const normalizedCurrent = currentSelections.map(String);

        const merged =
          normalizedCurrent.length > 0
            ? [...normalizedCurrent, ...newItemIds]
            : [...newItemIds];

        if (merged.length > 0) {
          selectMultipleMiscItems(type, merged);
        }
      } else {
        // single-select: pick the last uploaded item if any
        const lastNewId =
          newItemIds.length > 0 ? newItemIds[newItemIds.length - 1] : null;
        if (lastNewId) {
          selectMiscItem(type, lastNewId);
        }
      }

      toast.success(
        `Successfully uploaded ${newItems.length} ${getTypeDisplayName(
          type
        ).toLowerCase()} item(s)`
      );
      return true;
    } catch (err) {
      // If user cancelled the file dialog, backend returns an error string "No files selected".
      // Treat this as a non-fatal cancellation (don't spam the console in dev mode).
      const errMsg = err instanceof Error ? err.message : String(err);
      if (errMsg.includes("No files selected")) {
        // User cancelled the dialog; no need to show an error toast or console spam.
        return false;
      }

      console.error("Failed to upload multiple misc items:", err);
      const errorMessage =
        err instanceof Error ? err.message : "Failed to upload misc items";
      toast.error(errorMessage);
      return false;
    }
  };

  // Helper function to get display name for item type
  const getTypeDisplayName = (type: MiscItemType): string => {
    switch (type) {
      case "map":
        return "Map";
      case "font":
        return "Font";
      case "hud":
        return "HUD";
      case "misc":
        return "Misc";
      default:
        return type;
    }
  };

  // Delete a misc item
  const deleteMiscItem = async (itemId: string): Promise<boolean> => {
    try {
      await invoke("delete_misc_item", { itemId });
      removeMiscItem(itemId);
      return true;
    } catch (err) {
      console.error("Failed to delete misc item:", err);
      const errorMessage =
        err instanceof Error ? err.message : "Failed to delete misc item";
      toast.error(errorMessage);
      return false;
    }
  };

  // Load misc items on mount
  useEffect(() => {
    void loadMiscItems();
  }, []);

  return {
    miscItems,
    isLoading,
    error,
    uploadMiscItem,
    uploadMultipleMiscItems,
    deleteMiscItem,
    getTypeDisplayName,
    reload: loadMiscItems,
  };
}
