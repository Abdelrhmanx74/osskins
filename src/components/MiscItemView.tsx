"use client";

import React, { useState } from "react";
import { MiscItemType, useGameStore } from "@/lib/store";
import { useMiscItems } from "@/lib/hooks/use-misc-items";
import { Button } from "./ui/button";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";

interface MiscItemViewProps {
  type: MiscItemType;
}

export function MiscItemView({ type }: MiscItemViewProps) {
  const { t } = useI18n();
  const { selectedMiscItems, selectMiscItem, toggleMiscItemSelection } =
    useGameStore();
  const {
    miscItems,
    isLoading,
    error,
    uploadMultipleMiscItems,
    deleteMiscItem,
    getTypeDisplayName,
  } = useMiscItems();

  // State for uploading
  const [isUploading, setIsUploading] = useState(false);

  // Get items for this type
  const typeItems = miscItems.get(type) ?? [];
  const selectedItemIds = selectedMiscItems.get(type) ?? [];

  // Built-in base fonts (static). Files are located in src-tauri/resources/fonts.
  // NOTE: The attachments contain 4 .fantome files; user requested 5 base fonts so
  // we add a sensible default 'default.fantome' as the fifth. If you want a
  // different filename for the fifth built-in, update the id/name/fantome_path below.
  const builtinFonts =
    type === "font"
      ? [
          {
            id: "builtin-font-chinese",
            name: "Chinese",
            item_type: "font",
            fantome_path: "chinese.fantome",
          },
          {
            id: "builtin-font-korean",
            name: "Korean",
            item_type: "font",
            fantome_path: "korean.fantome",
          },
          {
            id: "builtin-font-minecraft",
            name: "Minecraft",
            item_type: "font",
            fantome_path: "minecraft.fantome",
          },
          {
            id: "builtin-font-arcade",
            name: "Arcade",
            item_type: "font",
            fantome_path: "arcade.fantome",
          },
        ]
      : [];

  // Items to display: built-in fonts (if font tab) followed by uploaded items
  const displayItems = [...builtinFonts, ...typeItems];

  // Handler for adding a new misc item (now supports multiple files)
  const handleAddNewItem = async () => {
    setIsUploading(true);
    try {
      const success = await uploadMultipleMiscItems(type);
      if (success) {
        // Success is already handled in the hook with toast
      }
    } catch (err) {
      console.error("Error uploading items:", err);
      toast.error(t("misc.upload_error"));
    } finally {
      setIsUploading(false);
    }
  };

  // Handle item selection: single-select for map/font/hud, multi-select for misc
  const handleItemSelect = (itemId: string) => {
    if (type === "misc") {
      // preserve multi-select behavior for misc tab
      toggleMiscItemSelection(type, itemId);
    } else {
      // enforce single selection for other types
      selectMiscItem(type, itemId);
    }
  };

  // Handle item deletion
  const handleItemDelete = async (itemId: string) => {
    const success = await deleteMiscItem(itemId);
    if (success) {
      toast.success(t("misc.delete_success"));
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <p className="text-muted-foreground">
          {t("misc.loading", { type: getTypeDisplayName(type).toLowerCase() })}
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full w-full">
        <p className="text-destructive">
          {t("misc.error_loading", {
            type: getTypeDisplayName(type).toLowerCase(),
            error,
          })}
        </p>
        <Button
          variant="outline"
          className="mt-4"
          onClick={() => {
            window.location.reload();
          }}
        >
          {t("misc.retry")}
        </Button>
      </div>
    );
  }

  return (
    <>
      <div className="size-full space-y-3 px-20 py-10">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold">
            {t("misc.items_title", { type: getTypeDisplayName(type) })}
          </h2>
        </div>

        {displayItems.map((item) => (
          <div
            key={item.id}
            className={`p-4 border rounded-lg cursor-pointer transition-colors ${
              selectedItemIds.includes(item.id)
                ? "border-primary bg-primary/10"
                : "border-border hover:border-primary/50"
            }`}
            onClick={() => {
              handleItemSelect(item.id);
            }}
          >
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-medium">{item.name}</h3>
                <p className="text-sm text-muted-foreground">
                  {t("misc.type_label")}: {item.item_type}
                </p>
              </div>
              <div className="flex items-center gap-2">
                {selectedItemIds.includes(item.id) && (
                  <div className="px-2 py-1 bg-primary text-primary-foreground text-xs rounded">
                    {t("misc.selected")}
                  </div>
                )}
                {/* Built-in fonts cannot be deleted */}
                {!String(item.id).startsWith("builtin-font-") && (
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleItemDelete(item.id);
                    }}
                  >
                    {t("misc.delete")}
                  </Button>
                )}
                {String(item.id).startsWith("builtin-font-") && (
                  <div className="px-2 py-1 text-xs rounded bg-muted text-muted-foreground">
                    Built-in
                  </div>
                )}
              </div>
            </div>
          </div>
        ))}

        {displayItems.length === 0 && (
          <div className="flex flex-col items-center mt-8">
            <p className="text-muted-foreground mb-4">
              {t("misc.no_items", {
                type: getTypeDisplayName(type).toLowerCase(),
              })}
            </p>
          </div>
        )}

        {/* Add item button that supports multiple file selection */}
        <Button
          size={"lg"}
          variant="outline"
          className="w-full border-dashed py-6 mt-1 justify-start"
          onClick={() => {
            void handleAddNewItem();
          }}
          disabled={isUploading}
        >
          <Plus className="size-8 opacity-50" />
          <span className="text-lg font-medium">
            {isUploading
              ? t("misc.uploading")
              : t("misc.add_items", { type: getTypeDisplayName(type) })}
          </span>
        </Button>
      </div>
    </>
  );
}
