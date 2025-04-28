import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "@/lib/store";

/**
 * Hook for persisting champion configurations
 */
export function useChampionPersistence() {
  const { leaguePath, selectedSkins, favorites } = useGameStore();

  // Persist configuration (league path + selected skins + favorites) on change
  useEffect(() => {
    if (!leaguePath) return;

    // prepare skins array from Map
    const skins = Array.from(selectedSkins.values()).map((s) => ({
      champion_id: s.championId,
      skin_id: s.skinId,
      chroma_id: s.chromaId,
      fantome: s.fantome,
    }));

    invoke("save_selected_skins", {
      leaguePath: leaguePath,
      skins,
      favorites: Array.from(favorites),
    }).catch((err: unknown) => {
      console.error(err);
    });
  }, [leaguePath, selectedSkins, favorites]);
}
