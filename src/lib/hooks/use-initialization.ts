import { useEffect, useState } from "react";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { useDataUpdate } from "./use-data-update";

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
        const { league_path, skins, favorites } = cfg as {
          league_path?: string;
          skins?: Array<any>;
          favorites?: number[];
        };

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
          const needsUpdate = !(await invoke<boolean>("check_champions_data"));
          console.log("Needs update:", needsUpdate);

          if (needsUpdate) {
            console.log("Starting data update...");
            setHasStartedUpdate(true);
            await updateData();
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
