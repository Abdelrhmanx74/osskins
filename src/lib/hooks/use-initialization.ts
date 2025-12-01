import { useEffect, useState } from "react";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { useDataUpdate } from "./use-data-update";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";

export function useInitialization() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);
  const { updateData } = useDataUpdate();
  const { setLeaguePath, selectSkin, setFavorites } = useGameStore();
  const { t } = useI18n();

  // Handle initial setup
  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        // Load saved config (path + skins + favorites)
        const cfg = await invoke<unknown>("load_config");
        const {
          league_path,
          skins,
          favorites,
          auto_update_data,
          last_data_commit,
        } = cfg as {
          league_path?: string;
          skins?: Array<any>;
          favorites?: number[];
          auto_update_data?: boolean;
          last_data_commit?: string | null;
        };

        console.log("[Init] Loaded config:", {
          league_path,
          skins_count: skins?.length ?? 0,
          favorites_count: favorites?.length ?? 0,
          auto_update_data: auto_update_data !== false,
          last_data_commit,
        });

        // Probe latest upstream commit for visibility during testing
        try {
          const latestUpstream = await invoke<string>("get_latest_data_commit");
          console.log(
            "[Init] Commits -> last_saved:",
            last_data_commit,
            "| latest_upstream:",
            latestUpstream,
          );
        } catch (e) {
          console.warn("[Init] Could not fetch latest upstream commit", e);
        }

        if (league_path) {
          setLeaguePath(league_path);

          // preload skin selections
          const loadedSelections: Array<{
            champion_id: number;
            skin_id: number;
            chroma_id?: number;
            skin_file?: string;
          }> = [];

          (skins ?? []).forEach((s: unknown) => {
            if (
              typeof s === "object" &&
              s !== null &&
              "champion_id" in s &&
              "skin_id" in s
            ) {
              const skinObj = s as {
                champion_id: number;
                skin_id: number;
                chroma_id?: number;
                skin_file?: string;
              };
              loadedSelections.push(skinObj);
              selectSkin(
                skinObj.champion_id,
                skinObj.skin_id,
                skinObj.chroma_id,
                skinObj.skin_file,
              );
            }
          });

          // Enrich any loaded selections missing skin_file using stored champion data
          try {
            const dataExists = await invoke<boolean>("check_champions_data");
            if (dataExists && loadedSelections.length > 0) {
              const raw = await invoke<string>("get_champion_data", {
                championId: 0,
              });
              const champions = JSON.parse(raw) as Array<{
                id: number;
                skins: Array<{
                  id: number;
                  skin_file?: string;
                  chromas?: Array<{ id: number; skin_file?: string }>;
                }>;
                name: string;
              }>;

              for (const sel of loadedSelections) {
                if (!sel.skin_file) {
                  const champ = champions.find((c) => c.id === sel.champion_id);
                  const skin = champ?.skins.find((sk) => sk.id === sel.skin_id);
                  let resolved: string | undefined = undefined;
                  if (
                    sel.chroma_id &&
                    skin?.chromas &&
                    skin.chromas.length > 0
                  ) {
                    resolved = skin.chromas.find(
                      (c) => c.id === sel.chroma_id,
                    )?.skin_file;
                  }
                  resolved ??= skin?.skin_file;
                  if (resolved) {
                    // Update store selection with resolved path
                    selectSkin(
                      sel.champion_id,
                      sel.skin_id,
                      sel.chroma_id,
                      resolved,
                    );
                  }
                }
              }
            }
          } catch (e) {
            console.warn(
              "[Init] Failed to enrich selections with skin_file",
              e,
            );
          }

          // Load favorites
          if (favorites) {
            setFavorites(new Set(favorites));
          }

          // start watcher
          void invoke("start_auto_inject", { leaguePath: league_path });
        }

        // Auto-update: if enabled in config, check for updates and update automatically on start
        try {
          const auto = auto_update_data !== false;
          if (auto && !hasStartedUpdate) {
            console.log("[Init] Auto-update enabled, triggering update check...");
            setHasStartedUpdate(true);
            // Start update silently; UI DownloadingModal can still be opened by the user
            // The updateData function now handles "up-to-date" checks internally
            void updateData();
          }
        } catch (e) {
          console.warn("[Init] Auto-update trigger failed", e);
        }

        if (mounted) {
          setIsInitialized(true);
        }
      } catch (error) {
        console.error("Failed to initialize:", error);
        if (mounted) {
          setIsInitialized(true); // Still mark as initialized so UI isn't stuck
        }
      }
    }

    // Only initialize if not already done
    if (!isInitialized) {
      void initialize();
    }

    return () => {
      mounted = false;
    };
  }, [
    isInitialized,
    hasStartedUpdate,
    updateData,
    setLeaguePath,
    selectSkin,
    setFavorites,
  ]);

  return { isInitialized, hasStartedUpdate, setHasStartedUpdate };
}
