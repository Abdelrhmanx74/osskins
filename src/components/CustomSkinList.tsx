"use client";

import React, { useState } from "react";
import { useCustomSkins } from "@/lib/hooks/use-custom-skins";
import { CustomSkinItem } from "./CustomSkinItem";
import { Button } from "./ui/button";
import { useChampions } from "@/lib/hooks/use-champions";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Label } from "./ui/label";
import { Input } from "./ui/input";

interface CustomSkinListProps {
  championId: number | null;
}

export function CustomSkinList({ championId }: CustomSkinListProps) {
  const { t } = useI18n();
  const {
    customSkins,
    isLoading,
    error,
    deleteCustomSkin,
    uploadMultipleCustomSkins,
  } = useCustomSkins();
  const { champions } = useChampions();

  // State for uploading
  const [isUploading, setIsUploading] = useState(false);

  // Get the champion data if ID is provided
  const champion = championId
    ? champions.find((c) => c.id === championId)
    : null;

  // Filter skins for the selected champion
  const championCustomSkins =
    championId !== null ? customSkins.get(championId) ?? [] : [];

  // Handler for adding new custom skins (now supports multiple files)
  const handleAddNewSkin = async () => {
    if (!championId) {
      toast.error(t("select.champion_first"));
      return;
    }

    setIsUploading(true);
    try {
      const result = await uploadMultipleCustomSkins(championId);
      if (result && result.length > 0) {
        // Success is already handled in the hook with toast
      }
    } catch (err) {
      console.error("Error uploading skins:", err);
      toast.error(t("upload.error"));
    } finally {
      setIsUploading(false);
    }
  };

  // Handler for uploading multiple skins
  const handleUploadMultipleSkins = async () => {
    if (!championId) {
      toast.error(t("select.champion_first"));
      return;
    }

    setIsUploading(true);
    try {
      const result = await uploadMultipleCustomSkins(championId);
      if (result && result.length > 0) {
        // Success is already handled in the hook with toast
      }
    } catch (err) {
      console.error("Error uploading multiple skins:", err);
      toast.error("Error uploading skins. Please try again.");
    } finally {
      setIsUploading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <p className="text-muted-foreground">{t("loading.custom_skins")}</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full w-full">
        <p className="text-destructive">
          {t("error.loading_custom_skins")}: {error}
        </p>
        <Button
          variant="outline"
          className="mt-4"
          onClick={() => {
            window.location.reload();
          }}
        >
          Retry
        </Button>
      </div>
    );
  }

  // If no champion is selected
  if (!championId) {
    return (
      <div className="flex flex-col items-center justify-center h-full w-full p-4">
        <p className="text-muted-foreground">
          {t("select.champion_to_view_custom_skins")}
        </p>
      </div>
    );
  }

  return (
    <>
      <div className="size-full space-y-3 px-20 py-10">
        {championCustomSkins.map((skin) => (
          <CustomSkinItem
            key={skin.id}
            skin={skin}
            onDelete={deleteCustomSkin}
          />
        ))}

        {championCustomSkins.length === 0 && (
          <div className="flex flex-col items-center mt-8">
            <p className="text-muted-foreground mb-4">
              {t("no_custom_skins_for").replace(
                "{champion}",
                champion?.name ?? t("this_champion")
              )}
            </p>
          </div>
        )}

        {/* Add skin button that supports multiple file selection */}
        <Button
          size={"lg"}
          variant="outline"
          className="w-full border-dashed py-6 mt-1 justify-start"
          onClick={() => {
            void handleAddNewSkin();
          }}
          disabled={isUploading}
        >
          <Plus className="size-8 opacity-50" />
          <span className="text-lg font-medium">
            {isUploading ? t("uploading") : t("add_custom_skins")}
          </span>
        </Button>
      </div>
    </>
  );
}
