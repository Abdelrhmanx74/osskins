import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import {
  fetchChampionDetails,
  fetchChampionSummaries,
  fetchSkinZip,
  getLegacyDownloadUrl,
  getLolSkinsManifest,
  getLolSkinsManifestCommit,
  resetLolSkinsManifestCache,
  sanitizeForFileName,
  transformChampionData,
} from "../data-utils";
import type { LolSkinsManifestItem } from "../data-utils";
import type { DataUpdateProgress, EnsureModToolsResult } from "../types";
import { useToolsStore } from "../store/tools";

// Helper function to delay execution
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

interface ManifestCollections {
  skins: Map<string, LolSkinsManifestItem>;
  chromas: Map<string, LolSkinsManifestItem>;
}

interface UpdateTuning {
  championConcurrency: number;
  skinConcurrency: number;
  perSkinDelayMs: number;
  perChampionDelayMs: number;
}

type BinaryBuffer = Uint8Array;

const normalizeManifestKey = (value: string): string =>
  value
    .toLowerCase()
    .replace(/['’`]/g, "")
    .replace(/[^a-z0-9]/g, "");

const stripZipExtension = (value: string): string =>
  value.toLowerCase().endsWith(".zip") ? value.slice(0, -4) : value;

const createManifestIndex = (
  items: LolSkinsManifestItem[],
): Map<string, ManifestCollections> => {
  const index = new Map<string, ManifestCollections>();

  for (const item of items) {
    const segments = item.path.split("/");
    if (segments.length < 3) {
      continue;
    }

    if (normalizeManifestKey(segments[0]) !== "skins") {
      continue;
    }

    const championKey = normalizeManifestKey(segments[1]);
    if (!index.has(championKey)) {
      index.set(championKey, {
        skins: new Map<string, LolSkinsManifestItem>(),
        chromas: new Map<string, LolSkinsManifestItem>(),
      });
    }

    const manifestEntry = index.get(championKey);
    if (!manifestEntry) {
      continue;
    }

    const lastSegment = stripZipExtension(segments[segments.length - 1]);
    const targetKey = normalizeManifestKey(lastSegment);

    if (segments.length >= 4 && normalizeManifestKey(segments[2]) === "chromas") {
      if (!manifestEntry.chromas.has(targetKey)) {
        manifestEntry.chromas.set(targetKey, item);
      }
    } else if (!manifestEntry.skins.has(targetKey)) {
      manifestEntry.skins.set(targetKey, item);
    }
  }

  return index;
};

const getHardwareConcurrency = (): number => {
  if (
    typeof navigator !== "undefined" &&
    typeof navigator.hardwareConcurrency === "number" &&
    Number.isFinite(navigator.hardwareConcurrency)
  ) {
    return navigator.hardwareConcurrency;
  }
  return 6;
};

const deriveUpdateTuning = (): UpdateTuning => {
  const threads = Math.min(Math.max(Math.floor(getHardwareConcurrency()), 2), 16);

  if (threads <= 4) {
    return {
      championConcurrency: 1,
      skinConcurrency: 2,
      perSkinDelayMs: 12,
      perChampionDelayMs: 60,
    };
  }

  if (threads <= 8) {
    return {
      championConcurrency: 2,
      skinConcurrency: 3,
      perSkinDelayMs: 6,
      perChampionDelayMs: 35,
    };
  }

  return {
    championConcurrency: 3,
    skinConcurrency: 4,
    perSkinDelayMs: 0,
    perChampionDelayMs: 15,
  };
};

const runWithConcurrency = async <T>(
  items: T[],
  requestedConcurrency: number,
  worker: (item: T, index: number) => Promise<void>,
): Promise<void> => {
  const effectiveConcurrency = Math.max(1, Math.min(items.length, Math.floor(requestedConcurrency)));

  if (effectiveConcurrency === 1) {
    for (let index = 0; index < items.length; index += 1) {
      await worker(items[index], index);
    }
    return;
  }

  let cursor = 0;
  const getNextIndex = (): number | null => {
    if (cursor >= items.length) {
      return null;
    }
    const currentIndex = cursor;
    cursor += 1;
    return currentIndex;
  };

  const workers = Array.from({ length: effectiveConcurrency }, async () => {
    for (let index = getNextIndex(); index !== null; index = getNextIndex()) {
      await worker(items[index], index);
    }
  });

  await Promise.all(workers);
};

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
      resetLolSkinsManifestCache();
      setProgress({
        currentChampion: "",
        totalChampions: 0,
        processedChampions: 0,
        status: "checking",
        progress: 0,
      });

      // Ensure CSLOL tools are present before fetching champion data
      const toolsStore = useToolsStore.getState();
      toolsStore.clearProgress("auto");
      const toolsResult = await (async () => {
        try {
          return await invoke<EnsureModToolsResult>("ensure_mod_tools", {
            force: false,
          });
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          toolsStore.mergeProgress("auto", { phase: "error", error: message });
          throw err;
        }
      })();

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

      const manifest = await getLolSkinsManifest();
      const manifestIndex = manifest
        ? createManifestIndex(manifest.items)
        : null;
      const manifestCommit = manifest
        ? getLolSkinsManifestCommit(manifest)
        : null;

      if (manifest) {
        console.log(
          `[Update] Using lol-skins manifest generated at ${manifest.generated_at} (commit ${manifestCommit ?? "unknown"})`,
        );
      } else {
        console.warn(
          "[Update] Manifest unavailable; falling back to legacy GitHub file resolution",
        );
      }

      // Proceed with update unconditionally when triggered (commit-based gating removed)
      // Fetch champion summaries
      const summaries = await fetchChampionSummaries();
      // Filter out invalid IDs (<=0) and test/PBE champions (Doom Bots with IDs >= 66600)
      const validSummaries = summaries.filter(
        (summary) => summary.id > 0 && summary.id < 66600,
      );
      console.log(
        `[Update] Loaded ${validSummaries.length} champions from CommunityDragon`,
      );

      // If we have last commit, get changed champions to narrow updates
      let targetSummaries = validSummaries;
      try {
        const changed = await invoke<string[]>(
          "get_changed_champions_from_config",
        );
        console.log("[Update] Changed champions from config:", changed);
        if (changed.length > 0) {
          const changedSet = new Set(
            changed.map((n) => n.toLowerCase().replace(/%20/g, " ")),
          );
          targetSummaries = validSummaries.filter((s) =>
            changedSet.has(s.name.toLowerCase()),
          );
          if (targetSummaries.length === 0) {
            console.warn(
              "[Update] Mapping changed champions to summaries yielded 0; falling back to full update",
            );
            targetSummaries = validSummaries;
          }
        }
      } catch (e) {
        console.warn(
          "[Update] Failed to get changed champions from config; defaulting to full update",
          e,
        );
      }

      console.log(
        `[Update] Targeting ${targetSummaries.length}/${validSummaries.length} champions`,
        targetSummaries.slice(0, 15).map((s) => s.name),
      );

      const totalChampions = targetSummaries.length;

      setProgress((prev) => {
        if (!prev) return null;
        return {
          ...prev,
          totalChampions,
          status: "downloading",
        };
      });

      // No toasts here; the UI will show the DataUpdateModal when `isUpdating` is true

      let processedCount = 0;
      const tuning = deriveUpdateTuning();
      console.log("[Update] Concurrency profile", tuning);

      const championManifestCache = manifestIndex ?? null;

      const processChampion = async (summary: (typeof targetSummaries)[number]) => {
        setProgress((prev) => {
          if (!prev) return null;
          return {
            ...prev,
            currentChampion: summary.name,
            status: "processing",
          };
        });

        try {
          const details = await fetchChampionDetails(summary.id);

          const localChampionKey = sanitizeForFileName(summary.name);
          const championKey = normalizeManifestKey(summary.name);
          const manifestEntry = championManifestCache?.get(championKey) ?? null;

          const skins = details.skins.filter((_, index) => index > 0);

          const processSkin = async (skin: (typeof skins)[number]) => {
            const localSkinKey = sanitizeForFileName(skin.name);
            const skinKey = normalizeManifestKey(skin.name);
            const skinManifestEntry = manifestEntry?.skins.get(skinKey) ?? null;

            let hasZipContent = false;
            if (skinManifestEntry) {
              try {
                // Download directly to disk via backend - memory efficient!
                await invoke("download_and_save_file", {
                  url: skinManifestEntry.url,
                  championName: localChampionKey,
                  fileName: `${localSkinKey}.zip`,
                });
                hasZipContent = true;
              } catch (error) {
                console.warn(
                  `[Manifest] Download failed for ${skinManifestEntry.url}:`,
                  error,
                );
              }
            }

            if (!hasZipContent) {
              // Fallback: try to get URL from legacy method and use backend download
              try {
                // Try to construct the legacy URL and download via backend
                const legacyUrl = await getLegacyDownloadUrl(
                  summary.name,
                  skin.name,
                  [],
                );
                if (legacyUrl) {
                  await invoke("download_and_save_file", {
                    url: legacyUrl,
                    championName: localChampionKey,
                    fileName: `${localSkinKey}.zip`,
                  });
                  hasZipContent = true;
                } else {
                  // Last resort: use old method (loads into memory)
                  const zipContent = (await fetchSkinZip(
                    summary.name,
                    skin.name,
                  )) as BinaryBuffer;
                  if (zipContent.byteLength > 0) {
                    await invoke("save_zip_file", {
                      championName: localChampionKey,
                      fileName: `${localSkinKey}.zip`,
                      content: Array.from(zipContent),
                    });
                    hasZipContent = true;
                  }
                }
              } catch (error) {
                console.warn(
                  `[Manifest] Fallback download failed for ${summary.name} / ${skin.name}:`,
                  error,
                );
              }
            }

            if (skin.chromas && skin.chromas.length > 0) {
              for (const chroma of skin.chromas) {
                try {
                  const chromaKey = normalizeManifestKey(`${skin.name} ${chroma.id}`);
                  const fallbackChromaKey = normalizeManifestKey(chroma.name);
                  const chromaManifestEntry =
                    manifestEntry?.chromas.get(chromaKey) ??
                    manifestEntry?.chromas.get(fallbackChromaKey) ??
                    null;

                  let hasChromaContent = false;
                  if (chromaManifestEntry) {
                    try {
                      // Download directly to disk via backend - memory efficient!
                      const chromaFileName = `${localSkinKey}_chroma_${chroma.id}.zip`;
                      await invoke("download_and_save_file", {
                        url: chromaManifestEntry.url,
                        championName: localChampionKey,
                        fileName: chromaFileName,
                      });
                      hasChromaContent = true;
                    } catch (error) {
                      console.warn(
                        `[Manifest] Download failed for ${chromaManifestEntry.url}:`,
                        error,
                      );
                    }
                  }

                  if (!hasChromaContent) {
                    // Fallback: try to get URL from legacy method and use backend download
                    try {
                      const legacyUrl = await getLegacyDownloadUrl(
                        summary.name,
                        `${skin.name} ${chroma.id}`,
                        ["chromas", skin.name],
                      );
                      if (legacyUrl) {
                        const chromaFileName = `${localSkinKey}_chroma_${chroma.id}.zip`;
                        await invoke("download_and_save_file", {
                          url: legacyUrl,
                          championName: localChampionKey,
                          fileName: chromaFileName,
                        });
                        hasChromaContent = true;
                      } else {
                        // Last resort: use old method (loads into memory)
                        const chromaContent = (await fetchSkinZip(
                          summary.name,
                          `${skin.name} ${chroma.id}`,
                          ["chromas", skin.name],
                        )) as BinaryBuffer;
                        if (chromaContent.byteLength > 0) {
                          const chromaFileName = `${localSkinKey}_chroma_${chroma.id}.zip`;
                          await invoke("save_zip_file", {
                            championName: localChampionKey,
                            fileName: chromaFileName,
                            content: Array.from(chromaContent),
                          });
                          hasChromaContent = true;
                        }
                      }
                    } catch (error) {
                      console.warn(
                        `[Manifest] Fallback download failed for chroma ${chroma.id}:`,
                        error,
                      );
                    }
                  }
                } catch (err) {
                  console.error(
                    `Failed to process chroma file for ${summary.name} ${skin.name} (${chroma.id}):`,
                    err,
                  );
                }

                if (tuning.perSkinDelayMs > 0) {
                  await delay(tuning.perSkinDelayMs);
                }
              }
            }

            if (tuning.perSkinDelayMs > 0) {
              await delay(tuning.perSkinDelayMs);
            }
          };

          await runWithConcurrency(skins, tuning.skinConcurrency, processSkin);

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
          toast.error(`Failed to process ${summary.name}`);
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

          if (tuning.perChampionDelayMs > 0 && processedCount < totalChampions) {
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

      if (manifest && manifestCommit) {
        try {
          await invoke("set_last_data_commit", {
            sha: manifestCommit,
            manifest_json: JSON.stringify(manifest),
          });
        } catch (err) {
          console.warn("[Update] Failed to persist manifest state", err);
        }
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
