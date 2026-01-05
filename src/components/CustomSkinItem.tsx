"use client";

import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "./ui/button";
import { CustomSkin } from "@/lib/types";

import { Trash2, Play, Check, MoreVertical, Pencil } from "lucide-react";
import { useGameStore } from "@/lib/store";
import { toast } from "sonner";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";

interface CustomSkinCardProps {
  skin: CustomSkin;
  onDelete: (skinId: string) => Promise<boolean>;
  onRename: () => void;
}

export function CustomSkinItem({
  skin,
  onDelete,
  onRename,
}: CustomSkinCardProps) {
  const [isHovering, setIsHovering] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const manualInjectionMode = useGameStore(
    (state) => state.manualInjectionMode,
  );
  const customSelectedSkins = useGameStore(
    (state) => state.customSelectedSkins,
  );
  const manualCustomSelectedSkins = useGameStore(
    (state) => state.manualCustomSelectedSkins,
  );
  const addCustomSkinSelection = useGameStore(
    (state) => state.addCustomSkinSelection,
  );
  const removeCustomSkinSelection = useGameStore(
    (state) => state.removeCustomSkinSelection,
  );
  const addManualCustomSkinSelection = useGameStore(
    (state) => state.addManualCustomSkinSelection,
  );
  const removeManualCustomSkinSelection = useGameStore(
    (state) => state.removeManualCustomSkinSelection,
  );

  // Check if this skin is selected
  const isSelected = manualInjectionMode
    ? (manualCustomSelectedSkins.get(skin.champion_id) ?? []).some(
        (s) => s.skin_file === skin.file_path,
      )
    : (customSelectedSkins.get(skin.champion_id) ?? []).some(
        (s) => s.skin_file === skin.file_path,
      );

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
            (async () => {
              const success = await onDelete(skin.id);
              if (!success) {
                throw new Error("Failed to delete skin");
              }
              return success;
            })(),
            {
              loading: "Deleting skin...",
              success: `"${skin.name}" was deleted successfully`,
              error: "Failed to delete skin",
            },
          );
        },
      },
    });
  };

  // Select or deselect this skin
  const handleClick = () => {
    if (manualInjectionMode) {
      if (isSelected) {
        removeManualCustomSkinSelection(skin.champion_id, skin.file_path);
      } else {
        addManualCustomSkinSelection(skin.champion_id, {
          championId: skin.champion_id,
          skinId: fakeSkinId,
          skin_file: skin.file_path,
        });
      }
      return;
    }

    if (isSelected) {
      removeCustomSkinSelection(skin.champion_id, skin.file_path);
    } else {
      // For custom skins we use the file path directly and allow multi-select
      addCustomSkinSelection(skin.champion_id, {
        championId: skin.champion_id,
        skinId: fakeSkinId,
        skin_file: skin.file_path,
      });
    }
  };

  return (
    <div
      role="button"
      tabIndex={0}
      className={cn(
        "w-full bg-primary dark:bg-primary/20 gap-0 rounded-lg overflow-hidden transition-all duration-300 flex items-center",
        isSelected ? "ring-2 ring-primary" : "",
      )}
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleClick();
        }
      }}
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
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="secondary"
              size="icon"
              className="h-8 w-8 rounded-full"
              onClick={(e) => e.stopPropagation()}
              disabled={isDeleting}
            >
              <MoreVertical className="h-4 w-4 text-primary" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onSelect={(e) => {
                e.preventDefault();
                onRename();
              }}
            >
              <Pencil className="h-4 w-4 mr-2" /> Edit name
            </DropdownMenuItem>
            <DropdownMenuItem
              className="text-destructive focus:text-destructive"
              onSelect={(e) => {
                e.preventDefault();
                confirmDelete();
              }}
            >
              <Trash2 className="h-4 w-4 mr-2" /> Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
