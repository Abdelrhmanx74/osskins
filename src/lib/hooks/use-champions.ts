import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface Champion {
  id: number;
  name: string;
  iconSrc: string;
  skins: Skin[];
}

export interface Skin {
  id: number;
  name: string;
  skinSrc: string;
  isBase?: boolean;
  isLegacy?: boolean;
  chromas?: CachedChroma[];
  fantome?: string; // Path to the fantome file
}

export interface CachedChroma {
  id: number;
  name: string;
  skinChromaPath: string;
  colors: string[];
  fantome?: string; // Path to the chroma's fantome file
}

export function useChampions() {
  const [champions, setChampions] = useState<Champion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasData, setHasData] = useState<boolean | null>(null);

  useEffect(() => {
    async function checkData() {
      try {
        const dataExists = await invoke<boolean>("check_champions_data");
        setHasData(dataExists);

        if (!dataExists) {
          setError("No champion data found. Please run the data update first.");
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
          throw new Error(
            "Invalid data format: expected an array of champions"
          );
        }

        setChampions(championsData);
        setError(null);
      } catch (error) {
        console.error("Failed to load champions:", error);
        setError(
          error instanceof Error ? error.message : "Failed to load champions"
        );
        setChampions([]);
      } finally {
        setLoading(false);
      }
    }

    void checkData();
  }, []);

  return { champions, loading, error, hasData };
}
