import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DataUpdateProgress, DataUpdateResult } from "../types";
import {
  fetchChampionSummaries,
  fetchChampionDetails,
  fetchFantomeFile,
  transformChampionData,
} from "../data-utils";

export function useDataUpdate() {
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<DataUpdateProgress | null>(null);

  const updateData = async () => {
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

      // If no data exists or updates are needed, proceed with update
      if (
        !updateResult?.updatedChampions ||
        updateResult.updatedChampions.length > 0
      ) {
        // Fetch champion summaries
        const summaries = await fetchChampionSummaries();
        setProgress((prev) => ({
          ...prev!,
          totalChampions: summaries.length,
          status: "downloading",
        }));

        // Process each champion
        for (let i = 0; i < summaries.length; i++) {
          const summary = summaries[i];

          // Skip champions with invalid IDs
          if (summary.id <= 0) {
            console.warn(
              `Skipping champion with invalid ID: ${summary.name} (ID: ${summary.id})`
            );
            continue;
          }

          setProgress((prev) => ({
            ...prev!,
            currentChampion: summary.name,
            processedChampions: i + 1,
            status: "processing",
            progress: ((i + 1) / summaries.length) * 100,
          }));

          try {
            // Fetch champion details
            const details = await fetchChampionDetails(summary.id);

            // Fetch and save all fantome files for this champion in parallel
            const fantomePromises = details.skins.map(
              async (skin, skinIndex) => {
                // Skip base skin (index 0)
                if (skinIndex === 0) {
                  console.log(`Skipping base skin for ${summary.name}`);
                  return;
                }

                try {
                  // Extract base skin ID by removing the champion ID prefix
                  // For example, if skin.id is 1051 and champion ID is 1, we want 51
                  const baseSkinId = skin.id % 1000;
                  const fantomeContent = await fetchFantomeFile(
                    summary.id,
                    baseSkinId
                  );

                  // Save regular skin fantome file
                  await invoke("save_fantome_file", {
                    championName: summary.name
                      .toLowerCase()
                      .replace(/\s+/g, "_"),
                    skinName: skin.name.toLowerCase().replace(/\s+/g, "_"),
                    isChroma: false,
                    content: Array.from(fantomeContent),
                  });
                  console.log(
                    `Successfully saved fantome file for ${summary.name} ${skin.name}`
                  );

                  // Save chroma fantome files if they exist
                  if (skin.chromas && skin.chromas.length > 0) {
                    const chromaPromises = skin.chromas.map(async (chroma) => {
                      // For chromas, we need to get the base skin ID from the chroma ID
                      const chromaBaseSkinId = chroma.id % 1000;
                      const chromaFantomeContent = await fetchFantomeFile(
                        summary.id,
                        chromaBaseSkinId
                      );
                      await invoke("save_fantome_file", {
                        championName: summary.name
                          .toLowerCase()
                          .replace(/\s+/g, "_"),
                        skinName: skin.name.toLowerCase().replace(/\s+/g, "_"),
                        isChroma: true,
                        chromaId: chroma.id,
                        content: Array.from(chromaFantomeContent),
                      });
                      console.log(
                        `Successfully saved fantome file for ${summary.name} ${skin.name} chroma ${chroma.id}`
                      );
                    });
                    await Promise.all(chromaPromises);
                  }
                } catch (error) {
                  console.error(
                    `Failed to process fantome file for ${summary.name} ${skin.name}:`,
                    error
                  );
                  throw error; // Propagate the error to be caught by the outer try-catch
                }
              }
            );

            try {
              await Promise.all(fantomePromises);
            } catch (error) {
              console.error(
                `Failed to process some fantome files for ${summary.name}:`,
                error
              );
              // Continue with the next champion even if some fantome files failed
            }

            // Only proceed with saving champion data
            const championData = transformChampionData(
              summary,
              details,
              new Map() // We don't need the fantome files in memory anymore
            );

            // Validate champion ID
            if (championData.id <= 0) {
              throw new Error(`Invalid champion ID: ${championData.id}`);
            }

            await invoke("update_champion_data", {
              championName: championData.name
                .toLowerCase()
                .replace(/\s+/g, "_"),
              data: JSON.stringify(championData),
            });

            console.log(`Successfully processed all data for ${summary.name}`);
          } catch (error) {
            console.error(`Failed to process ${summary.name}:`, error);
            toast.error(`Failed to process ${summary.name}`);
            // Continue to next champion even if this one failed
          }
        }

        toast.success(
          `Data update completed successfully (${summaries.length} champions)`
        );
      } else {
        toast.success("Data is up to date");
      }
    } catch (error) {
      console.error("Data update failed:", error);
      toast.error("Failed to update data");
    } finally {
      setIsUpdating(false);
      setProgress(null);
    }
  };

  return {
    isUpdating,
    progress,
    updateData,
  };
}
