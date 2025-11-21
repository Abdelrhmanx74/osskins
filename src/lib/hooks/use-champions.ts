import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { Champion } from "../types";

export function useChampions() {
  const { leaguePath } = useGameStore();
  const [champions, setChampions] = useState<Champion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasData, setHasData] = useState<boolean | null>(null);

  const loadChampions = useCallback(async () => {
    if (!leaguePath) {
      setChampions([]);
      setHasData(null);
      setLoading(false);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const dataExists = await invoke<boolean>("check_champions_data");
      setHasData(dataExists);

      if (!dataExists) {
        setChampions([]);
        setLoading(false);
        return;
      }

      const data = await invoke<string>("get_champion_data", {
        championId: 0,
      });

      if (!data) {
        throw new Error("No data received from the backend");
      }

      const championsData = JSON.parse(data) as Champion[];

      if (!Array.isArray(championsData)) {
        throw new Error("Invalid data format: expected an array of champions");
      }

      setChampions(championsData);
      setError(null);
    } catch (error) {
      console.error("Failed to load champions:", error);
      setError(
        error instanceof Error ? error.message : "Failed to load champions",
      );
      setChampions([]);
    } finally {
      setLoading(false);
    }
  }, [leaguePath]);

  useEffect(() => {
    void loadChampions();
  }, [loadChampions]);

  return {
    champions,
    loading,
    error,
    hasData,
    refreshChampions: loadChampions,
  };
}
