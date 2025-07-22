"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "../store";
import { CustomSkin } from "../types";
import { toast } from "sonner";

export function useCustomSkins() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { customSkins, setCustomSkins, addCustomSkin, removeCustomSkin } =
    useGameStore();

  // Load custom skins on initial mount
  useEffect(() => {
    const fetchCustomSkins = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const skins = await invoke<CustomSkin[]>("get_custom_skins");
        setCustomSkins(skins);
      } catch (err) {
        console.error("Failed to load custom skins:", err);
        setError(String(err));
      } finally {
        setIsLoading(false);
      }
    };

    void fetchCustomSkins();
  }, [setCustomSkins]);

  // Function to upload a new custom skin
  const uploadCustomSkin = async (championId: number, skinName: string) => {
    try {
      // Use the invoke function to call a new command that will handle file selection on the backend
      const newSkin = await invoke<CustomSkin>("upload_custom_skin", {
        championId,
        skinName,
      });

      // Add to local state
      addCustomSkin(newSkin);

      return newSkin;
    } catch (err) {
      console.error("Failed to upload custom skin:", err);
      setError(String(err));
      return null;
    }
  };

  // Function to upload multiple custom skins
  const uploadMultipleCustomSkins = async (championId: number) => {
    try {
      const newSkins = await invoke<CustomSkin[]>("upload_multiple_custom_skins", {
        championId,
      });

      // Add each new skin to the store
      newSkins.forEach(skin => {
        addCustomSkin(skin);
      });
      
      toast.success(`Successfully uploaded ${newSkins.length} custom skin(s)`);
      return newSkins;
    } catch (err) {
      console.error("Failed to upload multiple custom skins:", err);
      const errorMessage = err instanceof Error ? err.message : "Failed to upload multiple custom skins";
      toast.error(errorMessage);
      return null;
    }
  };

  // Function to delete a custom skin
  const deleteCustomSkin = async (skinId: string) => {
    try {
      await invoke("delete_custom_skin", { skinId });
      removeCustomSkin(skinId);
      return true;
    } catch (err) {
      console.error("Failed to delete custom skin:", err);
      setError(String(err));
      return false;
    }
  };

  return {
    customSkins,
    isLoading,
    error,
    uploadCustomSkin,
    uploadMultipleCustomSkins,
    deleteCustomSkin,
  };
}
