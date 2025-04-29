import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DataUpdateProgress, DataUpdateResult } from "../types";
import {
  fetchChampionSummaries,
  fetchChampionDetails,
  fetchFantomeFile,
  transformChampionData,
  sanitizeForFileName,
} from "../data-utils";

// Helper function to delay execution
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export function useDataUpdate() {
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<DataUpdateProgress | null>(null);

  const updateData = useCallback(async () => {
    if (isUpdating) {
      return;
    }

    const loadingToastId = toast("Updating data...");

    try {
      setIsUpdating(true);
      setProgress({
        currentChampion: "",
        totalChampions: 0,
        processedChampions: 0,
        status: "checking",
        progress: 0,
      });

      // Check for updates
      const updateResult = await invoke<DataUpdateResult>("check_data_updates");
      const dataExists = await invoke<boolean>("check_champions_data");

      // If updates are needed or no data exists, proceed with update
      if (
        !dataExists ||
        (updateResult.updatedChampions &&
          updateResult.updatedChampions.length > 0)
      ) {
        // Fetch champion summaries
        const summaries = await fetchChampionSummaries();
        const validSummaries = summaries.filter((summary) => summary.id > 0);
        const totalChampions = validSummaries.length;

        setProgress((prev) => ({
          ...prev!,
          totalChampions,
          status: "downloading",
        }));

        // Update loading toast with download info
        toast.dismiss(loadingToastId);
        const downloadToastId = toast("Downloading champions data");

        // Process champions in larger batches
        const BATCH_SIZE = 10;
        const DELAY_BETWEEN_BATCHES = 500;

        let processedCount = 0;
        for (let i = 0; i < validSummaries.length; i += BATCH_SIZE) {
          const batch = validSummaries.slice(i, i + BATCH_SIZE);

          // Process each champion in the batch
          const batchPromises = batch.map(async (summary) => {
            try {
              setProgress((prev) => ({
                ...prev!,
                currentChampion: summary.name,
                processedChampions: processedCount,
                status: "processing",
                progress: (processedCount / totalChampions) * 100,
              }));

              // Fetch champion details
              const details = await fetchChampionDetails(summary.id);

              // Process skins in parallel
              const skinPromises = details.skins
                .filter((_, index) => index > 0) // Skip base skin
                .map(async (skin) => {
                  try {
                    // Extract base skin ID
                    const baseSkinId = skin.id % 1000;
                    const fantomeContent = await fetchFantomeFile(
                      summary.id,
                      baseSkinId
                    );

                    // Save regular skin fantome file
                    await invoke("save_fantome_file", {
                      championName: sanitizeForFileName(summary.name),
                      skinName: sanitizeForFileName(skin.name),
                      isChroma: false,
                      content: Array.from(fantomeContent),
                    });

                    // Process chromas in parallel
                    if (skin.chromas && skin.chromas.length > 0) {
                      const chromaPromises = skin.chromas.map(
                        async (chroma) => {
                          const chromaBaseSkinId = chroma.id % 1000;
                          const chromaFantomeContent = await fetchFantomeFile(
                            summary.id,
                            chromaBaseSkinId
                          );
                          await invoke("save_fantome_file", {
                            championName: sanitizeForFileName(summary.name),
                            skinName: sanitizeForFileName(skin.name),
                            isChroma: true,
                            chromaId: chroma.id,
                            content: Array.from(chromaFantomeContent),
                          });
                        }
                      );
                      await Promise.all(chromaPromises);
                    }
                  } catch (err) {
                    console.error(
                      `Failed to process fantome file for ${summary.name} ${skin.name}:`,
                      err
                    );
                  }
                });

              await Promise.all(skinPromises);

              // Save champion data
              const championData = transformChampionData(
                summary,
                details,
                new Map()
              );

              if (championData.id <= 0) {
                throw new Error(`Invalid champion ID: ${championData.id}`);
              }

              await invoke("update_champion_data", {
                championName: sanitizeForFileName(championData.name),
                data: JSON.stringify(championData),
              });

              processedCount++;
              setProgress((prev) => ({
                ...prev!,
                currentChampion: summary.name,
                processedChampions: processedCount,
                status: "processing",
                progress: (processedCount / totalChampions) * 100,
              }));

              // Update the download toast every 5 champions
              if (processedCount % 5 === 0) {
                toast.dismiss(downloadToastId);
                const progressPercentage = Math.floor(
                  (processedCount / totalChampions) * 100
                );
                const newToastId = toast(
                  `${processedCount}/${totalChampions} (${progressPercentage}%) - Current: ${summary.name}`
                );
              }
            } catch (err) {
              console.error(`Failed to process ${summary.name}:`, err);
              toast.error(`Failed to process ${summary.name}`);
            }
          });

          await Promise.all(batchPromises);

          // Add small delay between batches
          if (i + BATCH_SIZE < validSummaries.length) {
            await delay(DELAY_BETWEEN_BATCHES);
          }
        }

        if (processedCount !== totalChampions) {
          console.warn(
            `Processed ${processedCount} out of ${totalChampions} champions`
          );
        }

        toast.dismiss(downloadToastId);
        toast.success("Data updated successfully");
      } else {
        toast.dismiss(loadingToastId);
        toast.success("Champion data is already up to date");
      }
    } catch (err) {
      toast.dismiss(loadingToastId);
      console.error("Data update failed:", err);
      toast.error("Failed to update data");
    } finally {
      setIsUpdating(false);
      setProgress(null);
    }
  }, [isUpdating]);

  return {
    isUpdating,
    progress,
    updateData,
  };
}
