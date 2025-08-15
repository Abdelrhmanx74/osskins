import { useEffect, useState } from "react";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { useDataUpdate } from "./use-data-update";
import { toast } from "sonner";

export function useInitialization() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);
  const { updateData } = useDataUpdate();
  const { setLeaguePath, selectSkin, setFavorites } = useGameStore();

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
            latestUpstream
          );
        } catch (e) {
          console.warn("[Init] Could not fetch latest upstream commit", e);
        }

        if (league_path) {
          setLeaguePath(league_path);

          // preload skin selections
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
                fantome?: string;
              };
              selectSkin(
                skinObj.champion_id,
                skinObj.skin_id,
                skinObj.chroma_id,
                skinObj.fantome
              );
            }
          });

          // Load favorites
          if (favorites) {
            setFavorites(new Set(favorites));
          }

          // start watcher
          void invoke("start_auto_inject", { leaguePath: league_path });
        }

        // Only check for updates if we haven't already started
        if (!hasStartedUpdate && mounted) {
          try {
            const updateInfo = await invoke<{
              success: boolean;
              updatedChampions?: string[];
            }>("check_data_updates");

            console.log("[Init] check_data_updates ->", updateInfo);

            const hasNew = (updateInfo.updatedChampions?.length ?? 0) > 0;

            if (hasNew) {
              if (auto_update_data !== false) {
                console.log("[Init] Auto update is ON -> starting update");
                setHasStartedUpdate(true);
                await updateData();
              } else {
                console.log(
                  "[Init] Auto update is OFF -> showing toast prompt"
                );
                toast("New data is available", {
                  closeButton: true,
                  duration: 999999,
                  action: {
                    label: "Update data",
                    onClick: () => {
                      setHasStartedUpdate(true);
                      void updateData();
                    },
                  },
                });
              }
            } else {
              console.log("[Init] No new data available");
            }
          } catch (e) {
            console.warn("[Init] check_data_updates failed", e);
          }
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
