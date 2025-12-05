import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import {
  fetchChampionDetails,
  fetchChampionSummaries,
  buildSkinDownloadUrl,
  sanitizeForFileName,
  transformChampionData,
} from "../data-utils";
import type { DataUpdateProgress, EnsureModToolsResult } from "../types";
import { useToolsStore } from "../store/tools";

// Helper function to delay execution
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

interface UpdateTuning {
  championConcurrency: number;
  skinConcurrency: number;
  perSkinDelayMs: number;
  perChampionDelayMs: number;
}

// Run async tasks with limited concurrency
async function runWithConcurrency<T, R>(
  items: T[],
  concurrency: number,
  handler: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const results: R[] = [];
  const queue = [...items.entries()];

  const worker = async () => {
    while (queue.length > 0) {
      const entry = queue.shift();
      if (!entry) break;
      const [index, item] = entry;
      results[index] = await handler(item, index);
    }
  };

  await Promise.all(Array.from({ length: concurrency }, () => worker()));
  return results;
}

/**
 * Hook for managing champion data updates
 * Uses the new LeagueSkins repository with ID-based structure
 */
export function useDataUpdate() {
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<DataUpdateProgress | null>(null);

  const updateData = useCallback(
    async (
      championsToUpdate?: string[],
      options?: {
        force?: boolean;
        onProgress?: (progress: DataUpdateProgress) => void;
      },
    ) => {
      if (isUpdating) return;

      const force = options?.force ?? false;
      setIsUpdating(true);
      setProgress({
        currentChampion: "",
        totalChampions: 0,
        processedChampions: 0,
        status: "checking",
        progress: 0,
      });

      // Performance tuning
      const tuning: UpdateTuning = {
        championConcurrency: 3,
        skinConcurrency: 4,
        perSkinDelayMs: 0,
        perChampionDelayMs: 50,
      };

      const toolsStore = useToolsStore.getState();

      try {
        // Ensure mod tools are available (run in background)
        void (async () => {
          try {
            toolsStore.mergeProgress("auto", {
              phase: "checking",
            });
            const toolsResult = await invoke<EnsureModToolsResult>(
              "ensure_mod_tools",
              { source: "auto" },
            );
            toolsStore.mergeProgress("auto", {
              phase: toolsResult.installed || toolsResult.updated ? "completed" : "idle",
              version: toolsResult.version,
            });
            const hasUpdate = Boolean(
              toolsResult.latestVersion &&
              toolsResult.version &&
              toolsResult.version !== toolsResult.latestVersion,
            );
            toolsStore.updateStatus({
              installed: toolsResult.installed,
              version: toolsResult.version ?? null,
              latestVersion: toolsResult.latestVersion ?? null,
              hasUpdate,
              path: toolsResult.path ?? null,
              lastChecked: Date.now(),
            });
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            toolsStore.mergeProgress("auto", {
              phase: "error",
              error: message,
            });
          }
        })();

        // Fetch latest commit SHA for tracking
        let latestCommit: string | null = null;
        try {
          latestCommit = await invoke<string>("get_latest_data_commit");
          console.log(`[Update] Latest commit: ${latestCommit}`);
        } catch (err) {
          console.warn("[Update] Failed to fetch latest commit:", err);
        }

        // Check if we are already up to date (skip if not forced)
        // But always update if data doesn't exist locally
        if (!force && latestCommit) {
          try {
            const dataExists = await invoke<boolean>("check_champions_data");
            if (!dataExists) {
              console.log("[Update] No local data exists, proceeding with download");
            } else {
              const config = await invoke<{ last_data_commit?: string }>("load_config");
              const storedCommit = config.last_data_commit;
              console.log(`[Update] Stored commit: ${storedCommit ?? "none"}, Latest: ${latestCommit}`);
              if (storedCommit && storedCommit === latestCommit) {
                console.log("[Update] Local data is up to date. Skipping update.");
                setIsUpdating(false);
                setProgress(null);
                return;
              }
              console.log("[Update] Update needed - commits differ or no stored commit");
            }
          } catch (err) {
            console.warn("[Update] Failed to check config (will proceed with update):", err);
          }
        }

        // Fetch champion summaries from Community Dragon
        console.log("[Update] Fetching champion summaries from Community Dragon...");
        const allSummaries = await fetchChampionSummaries();
        // Filter out non-playable champions:
        // - ID <= 0 are placeholders/none
        // - ID >= 1000 are doom bots (66xxx), test champions, etc.
        // Real playable champions have IDs from 1-999
        const validSummaries = allSummaries.filter(
          (s) => s.id > 0 && s.id < 1000,
        );

        // Determine which champions to update
        let targetSummaries = validSummaries;

        if (championsToUpdate && championsToUpdate.length > 0) {
          const hasAllMarker = championsToUpdate.includes("all");
          const hasRepoMarker = championsToUpdate.includes("repo");

          if (!hasAllMarker && !hasRepoMarker) {
            // Filter to specific champions (by ID or name)
            const champSet = new Set(
              championsToUpdate.map((c) => c.toLowerCase()),
            );
            targetSummaries = validSummaries.filter(
              (s) =>
                champSet.has(s.id.toString()) ||
                champSet.has(s.name.toLowerCase()) ||
                champSet.has(s.alias.toLowerCase()),
            );
          }
        }

        const totalChampions = targetSummaries.length;
        console.log(`[Update] Processing ${totalChampions} champions...`);

        setProgress({
          currentChampion: "",
          totalChampions,
          processedChampions: 0,
          status: "downloading",
          progress: 0,
        });

        let processedCount = 0;

        const processChampion = async (
          summary: (typeof targetSummaries)[0],
        ) => {
          try {
            setProgress((prev) => {
              if (!prev) return null;
              return {
                ...prev,
                currentChampion: summary.name,
                status: "downloading",
              };
            });

            // Fetch champion details from Community Dragon
            const details = await fetchChampionDetails(summary.id);
            const skins = details.skins.filter((s) => !s.isBase);

            // Download skins using the new ID-based system
            // The LeagueSkins repo has mixed extensions (.zip and .fantome)
            const downloadSkin = async (skin: (typeof skins)[0]) => {
              try {
                const championName = sanitizeForFileName(summary.name);
                const baseName = sanitizeForFileName(skin.name);

                // Try .zip first, then .fantome if 404
                let downloaded = false;
                for (const ext of ["zip", "fantome"] as const) {
                  if (downloaded) break;
                  const url = buildSkinDownloadUrl(summary.id, skin.id, undefined, undefined, ext);
                  const fileName = `${baseName}.${ext}`;

                  try {
                    await invoke("download_file_to_champion_with_progress", {
                      url,
                      championName,
                      fileName,
                    });
                    downloaded = true;
                  } catch (err) {
                    // If it's a 404, try the next extension
                    const errStr = String(err);
                    if (errStr.includes("404") && ext === "zip") {
                      continue; // Try .fantome
                    }
                    // For non-404 errors on .zip, or any error on .fantome, skip this skin
                    if (ext === "fantome" || !errStr.includes("404")) {
                      // Skin doesn't exist in repo - this is expected for some skins
                      return;
                    }
                  }
                }

                if (!downloaded) {
                  // Skin not available in either format - silently skip
                  return;
                }

                // Download chromas if present
                if (skin.chromas && skin.chromas.length > 0) {
                  for (const chroma of skin.chromas) {
                    // Try both extensions for chromas too
                    let chromaDownloaded = false;
                    for (const ext of ["zip", "fantome"] as const) {
                      if (chromaDownloaded) break;
                      const chromaUrl = buildSkinDownloadUrl(
                        summary.id,
                        skin.id,
                        chroma.id,
                        undefined,
                        ext,
                      );
                      const chromaFileName = `${baseName}_chroma_${chroma.id}.${ext}`;

                      try {
                        await invoke("download_file_to_champion_with_progress", {
                          url: chromaUrl,
                          championName,
                          fileName: chromaFileName,
                        });
                        chromaDownloaded = true;
                      } catch {
                        // Try next extension or skip if chroma doesn't exist
                        continue;
                      }
                    }

                    if (tuning.perSkinDelayMs > 0) {
                      await delay(tuning.perSkinDelayMs);
                    }
                  }
                }

                if (tuning.perSkinDelayMs > 0) {
                  await delay(tuning.perSkinDelayMs);
                }
              } catch {
                // Skin download failed or skin doesn't exist - silently skip
                // Many skins from Community Dragon don't exist in LeagueSkins repo
              }
            };

            await runWithConcurrency(skins, tuning.skinConcurrency, downloadSkin);

            // Save champion metadata
            const championData = transformChampionData(
              summary,
              details,
              new Map(),
            );

            if (championData.id <= 0) {
              throw new Error(`Invalid champion ID: ${championData.id}`);
            }

            await invoke("update_champion_data", {
              championName: sanitizeForFileName(championData.name),
              data: JSON.stringify(championData),
            });
          } catch (err) {
            console.error(`Failed to process ${summary.name}:`, err);
          } finally {
            processedCount += 1;
            setProgress((prev) => {
              if (!prev) return null;
              const processedChampions = Math.min(
                prev.totalChampions,
                prev.processedChampions + 1,
              );
              const progressValue =
                prev.totalChampions === 0
                  ? 0
                  : (processedChampions / prev.totalChampions) * 100;
              return {
                ...prev,
                currentChampion: summary.name,
                processedChampions,
                status: "processing",
                progress: progressValue,
              };
            });

            if (
              tuning.perChampionDelayMs > 0 &&
              processedCount < totalChampions
            ) {
              await delay(tuning.perChampionDelayMs);
            }
          }
        };

        await runWithConcurrency(
          targetSummaries,
          tuning.championConcurrency,
          processChampion,
        );

        if (processedCount !== totalChampions) {
          console.warn(
            `Processed ${processedCount} out of ${totalChampions} champions`,
          );
        }

        // Save the commit SHA for future update checks
        if (latestCommit) {
          try {
            await invoke("set_last_data_commit", {
              sha: latestCommit,
              manifestJson: null,
            });
          } catch (err) {
            console.warn("[Update] Failed to save commit SHA:", err);
          }
        }

        console.log("[Update] Data update completed successfully");
      } catch (err) {
        console.error("Data update failed:", err);

        setProgress((prev) => {
          if (!prev) return null;
          return { ...prev, currentChampion: "" };
        });

        console.error("Data update failed:", err);

        throw err;
      } finally {
        setIsUpdating(false);
        setProgress(null);
      }
    },
    [isUpdating],
  );

  return {
    isUpdating,
    progress,
    updateData,
  };
}
