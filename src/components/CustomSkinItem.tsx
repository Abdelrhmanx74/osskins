"use client";

import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "./ui/button";
import { CustomSkin } from "@/lib/types";
import { shallow } from "zustand/shallow";

import { Trash2, Play, Check } from "lucide-react";
import { useGameStore } from "@/lib/store";
import { toast } from "sonner";

interface CustomSkinCardProps {
  skin: CustomSkin;
  onDelete: (skinId: string) => Promise<boolean>;
}

export function CustomSkinItem({ skin, onDelete }: CustomSkinCardProps) {
  const [isHovering, setIsHovering] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  // Only subscribe to the specific state/actions needed
  const selectedSkins = useGameStore((s) => s.selectedSkins);
  const selectSkin = useGameStore((s) => s.selectSkin);
  const clearSelection = useGameStore((s) => s.clearSelection);

  // Check if this skin is selected
  const isSelected =
    selectedSkins.get(skin.champion_id)?.fantome === skin.file_path;

  // Generate a fake skin ID for custom skins (used for selection tracking)
  const fakeSkinId = parseInt(skin.id.replace(/\D/g, "").slice(0, 8)) || 999999;

  const handleMouseEnter = () => {
    setIsHovering(true);
  };

  const handleMouseLeave = () => {
    setIsHovering(false);
  };

  // Handle delete button click
  const confirmDelete = () => {
    toast.warning(`Delete "${skin.name}"?`, {
      description: "This action cannot be undone.",
      duration: 5000,
      action: {
        label: "Delete",
        onClick: () => {
          setIsDeleting(true);

          toast.promise(
            async () => {
              const success = await onDelete(skin.id);
              if (!success) {
                throw new Error("Failed to delete skin");
              }
              return success;
            },
            {
              loading: "Deleting skin...",
              success: `"${skin.name}" was deleted successfully`,
              error: "Failed to delete skin",
            }
          );
        },
      },
    });
  };

  // Select or deselect this skin
  const handleClick = () => {
    if (isSelected) {
      clearSelection(skin.champion_id);
    } else {
      // For custom skins we use the file path directly
      selectSkin(skin.champion_id, fakeSkinId, undefined, skin.file_path);
    }
  };

  return (
    <Button
      className={cn(
        "w-full py-6 px-1 bg-primary dark:bg-primary/20 gap-0 rounded-lg overflow-hidden transition-all duration-300",
        isSelected ? "ring-2 ring-primary" : ""
      )}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div className="p-2">
        {!isSelected && <Play className="size-8" />}
        {isSelected && <Check className="size-8 text-primary" />}
      </div>

      <div className="flex justify-between gap-2 px-2 items-center w-full">
        <h3 className="text-lg font-semibold text-white drop-shadow-md">
          {skin.name}
        </h3>
        <div
          role="button"
          tabIndex={0}
          aria-label={`Delete ${skin.name}`}
          className="h-8 w-8 rounded-full opacity-80 hover:opacity-100 flex items-center justify-center bg-destructive text-destructive-foreground cursor-pointer focus:outline-none focus:ring-2 focus:ring-destructive"
          onClick={(e) => {
            e.stopPropagation();
            confirmDelete();
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              confirmDelete();
            }
          }}
          style={{
            pointerEvents: isDeleting ? "none" : undefined,
            opacity: isDeleting ? 0.5 : undefined,
          }}
        >
          <Trash2 className="h-4 w-4" />
        </div>
      </div>
    </Button>
  );
}
