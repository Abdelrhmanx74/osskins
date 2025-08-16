import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DataUpdateProgress, DataUpdateResult } from "../types";
import {
  fetchChampionSummaries,
  fetchChampionDetails,
  fetchFantomeFile,
  fetchSkinZip,
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

    // Progress is shown in the DataUpdateModal; avoid toast notifications during download

    try {
      console.log("[Update] Starting data update flow");
      setIsUpdating(true);
      setProgress({
        currentChampion: "",
        totalChampions: 0,
        processedChampions: 0,
        status: "checking",
        progress: 0,
      });

      // Proceed with update unconditionally when triggered (commit-based gating removed)
      // Fetch champion summaries
      const summaries = await fetchChampionSummaries();
      const validSummaries = summaries.filter((summary) => summary.id > 0);
      console.log(
        `[Update] Loaded ${validSummaries.length} champions from CommunityDragon`
      );

      // If we have last commit, get changed champions to narrow updates
      let targetSummaries = validSummaries;
      try {
        const changed = await invoke<string[]>(
          "get_changed_champions_from_config"
        );
        console.log("[Update] Changed champions from config:", changed);
        if (changed.length > 0) {
          const changedSet = new Set(
            changed.map((n) => n.toLowerCase().replace(/%20/g, " "))
          );
          targetSummaries = validSummaries.filter((s) =>
            changedSet.has(s.name.toLowerCase())
          );
          if (targetSummaries.length === 0) {
            console.warn(
              "[Update] Mapping changed champions to summaries yielded 0; falling back to full update"
            );
            targetSummaries = validSummaries;
          }
        }
      } catch (e) {
        console.warn(
          "[Update] Failed to get changed champions from config; defaulting to full update",
          e
        );
      }

      console.log(
        `[Update] Targeting ${targetSummaries.length}/${validSummaries.length} champions`,
        targetSummaries.slice(0, 15).map((s) => s.name)
      );

      const totalChampions = targetSummaries.length;

      setProgress((prev) => ({
        ...prev!,
        totalChampions,
        status: "downloading",
      }));

      // No toasts here; the UI will show the DataUpdateModal when `isUpdating` is true

      // Process champions in larger batches
      const BATCH_SIZE = 10;
      const DELAY_BETWEEN_BATCHES = 500;

      let processedCount = 0;
      for (let i = 0; i < targetSummaries.length; i += BATCH_SIZE) {
        const batch = targetSummaries.slice(i, i + BATCH_SIZE);
        console.log(
          `[Update] Processing batch ${i / BATCH_SIZE + 1} (${
            batch.length
          } champions)`
        );

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
                  // Prepare local vs repo names
                  const localChampionKey = sanitizeForFileName(summary.name);
                  const downloadName = skin.name
                    .replace(/:/g, "")
                    .replace(/\//g, "");
                  const localSkinKey = sanitizeForFileName(downloadName);
                  const repoChampionName = summary.name;
                  const repoSkinName = downloadName;

                  // Attempt to download and save regular skin ZIP
                  const baseSkinId = skin.id % 1000;
                  const zipContent = await fetchSkinZip(
                    repoChampionName,
                    [],
                    repoSkinName
                  );
                  if (zipContent.byteLength > 0) {
                    await invoke("save_zip_file", {
                      championName: localChampionKey,
                      fileName: `${localSkinKey}.zip`,
                      content: Array.from(zipContent),
                    });
                  }

                  // Download and save chroma ZIPs if present
                  if (skin.chromas && skin.chromas.length > 0) {
                    await Promise.all(
                      skin.chromas.map(async (chroma) => {
                        const chromaId = chroma.id;
                        const chromaFileName = `${repoSkinName} ${chromaId}`;
                        const chromaPath = ["chromas", repoSkinName];

                        const chromaContent = await fetchSkinZip(
                          repoChampionName,
                          chromaPath,
                          chromaFileName
                        );

                        if (chromaContent.byteLength > 0) {
                          const chromaFileName = `${localSkinKey}_chroma_${chroma.id}.zip`;
                          await invoke("save_zip_file", {
                            championName: localChampionKey,
                            fileName: chromaFileName,
                            content: Array.from(chromaContent),
                          });
                        }
                      })
                    );
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

            // progress shown in modal; no toast updates
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

      // Data update finished successfully; modal will close via isUpdating state

      // No commit recording in legacy/manual flow
    } catch (err) {
      console.error("Data update failed:", err);
      try {
        toast.error("Failed to update data");
      } catch (e) {
        // ignore toast failures in non-browser contexts
      }
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
