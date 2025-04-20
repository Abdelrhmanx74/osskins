"use client";

import { useState, useEffect, Suspense } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { DataUpdateModal } from "@/components/DataUpdateModal";
import { ChampionCard } from "@/components/ChampionCard";
import { SkinCard } from "@/components/SkinCard";
import { useChampions } from "@/lib/hooks/use-champions";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { RefreshCw, Search, Heart } from "lucide-react";
import { Input } from "@/components/ui/input";

// Loading component using React 19 suspense
const ChampionsLoader = () => (
  <div className="flex flex-col items-center justify-center h-screen">
    <div className="text-2xl font-bold mb-4">Loading Champions</div>
    <div className="animate-spin w-12 h-12 border-4 border-primary border-t-transparent rounded-full"></div>
  </div>
);

export default function Home() {
  const { isUpdating, progress } = useDataUpdate();
  const { champions, loading, error, hasData } = useChampions();
  const {
    leaguePath,
    setLeaguePath,
    selectSkin,
    setLcuStatus,
    lcuStatus,
    selectedSkins,
    favorites,
    toggleFavorite,
    setFavorites,
  } = useGameStore();
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  // Handle initial setup
  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        // Load saved config (path + skins)
        const cfg = await invoke<unknown>("load_config");
        const { league_path, skins } = cfg as {
          league_path?: string;
          skins?: Array<any>;
        };
        if (league_path && mounted) {
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
          // start watcher
          void invoke("start_auto_inject", { leaguePath: league_path });
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

    // Load favorites from localStorage
    if (typeof window !== "undefined") {
      const storedFavorites = localStorage.getItem("championFavorites");
      if (storedFavorites) {
        try {
          const parsedFavorites = JSON.parse(storedFavorites) as number[];
          setFavorites(new Set<number>(parsedFavorites));
        } catch (err) {
          console.error("Failed to parse favorites:", err);
        }
      }
    }

    // listen for LCU status events
    const unlisten = listen<string>("lcu-status", (event) => {
      console.log(`LCU status changed to: ${event.payload}`);
      setLcuStatus(event.payload);
    });

    // prepare cleanup
    return () => {
      mounted = false;
      void unlisten.then((f) => {
        f();
      });
    };
  }, [
    isInitialized,
    setLeaguePath,
    hasStartedUpdate,
    selectSkin,
    setLcuStatus,
    setFavorites,
  ]);

  // Persist configuration (league path + selected skins) on change
  useEffect(() => {
    if (!leaguePath) return;
    // prepare skins array from Map
    const skins = Array.from(selectedSkins.values()).map((s) => ({
      champion_id: s.championId,
      skin_id: s.skinId,
      chroma_id: s.chromaId,
      fantome: s.fantome,
    }));

    void invoke("save_selected_skins", {
      leaguePath: leaguePath,
      skins,
    }).catch((error: unknown) => {
      console.error(error);
    });
  }, [leaguePath, selectedSkins]);

  // Save favorites to localStorage when they change
  useEffect(() => {
    if (typeof window !== "undefined" && favorites.size > 0) {
      localStorage.setItem(
        "championFavorites",
        JSON.stringify(Array.from(favorites))
      );
    }
  }, [favorites]);

  // Sort champions: favorites at the top, then alphabetically
  const filteredChampions = champions
    .filter((champion) =>
      champion.name.toLowerCase().includes(searchQuery.toLowerCase())
    )
    .sort((a, b) => {
      const aFav = favorites.has(a.id);
      const bFav = favorites.has(b.id);
      if (aFav && !bFav) return -1;
      if (!aFav && bFav) return 1;
      return a.name.localeCompare(b.name);
    });

  // If updating, show the modal
  if (isUpdating) {
    return (
      <Suspense fallback={<ChampionsLoader />}>
        <main className="flex min-h-screen flex-col items-center justify-center p-24">
          <div className="flex flex-col items-center gap-8">
            <h1 className="text-2xl font-bold">Initializing...</h1>
            <p className="text-muted-foreground">
              Please wait while we prepare your champion data
            </p>
            <DataUpdateModal isOpen={true} progress={progress} />
          </div>
        </main>
      </Suspense>
    );
  }

  // If no League path is selected, show directory selector
  if (!leaguePath) {
    return (
      <main className="flex min-h-screen flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Welcome to League Skin Manager</h1>
          <p className="text-muted-foreground">
            Please select your League of Legends installation directory to
            continue
          </p>
          <GameDirectorySelector />
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <div className="text-destructive">Error: {error}</div>
      </div>
    );
  }

  if (loading) {
    return <ChampionsLoader />;
  }

  if (!hasData) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">Updating champion data...</div>
      </div>
    );
  }

  if (champions.length === 0) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">No champions found</div>
      </div>
    );
  }

  const selectedChampionData =
    selectedChampion !== null
      ? champions.find((c) => c.id === selectedChampion)
      : null;

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-screen bg-background">
        {/* Top bar with search and injection status dot */}
        <div className="flex items-center justify-between p-4 border-b max-w-7xl w-full mx-auto">
          <div className="flex items-center gap-4 flex-1 max-w-md">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                className="pl-8"
                placeholder="Search champions..."
                value={searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                }}
              />
            </div>
            {/* Removed favorites filter button */}
          </div>
          <div className="flex items-center gap-4">
            {/* Removed Update Data button */}
            {/* status dot */}
            <div
              className={`h-3 w-3 rounded-full ${
                lcuStatus === "ChampSelect"
                  ? "bg-yellow-500"
                  : lcuStatus === "InProgress"
                  ? "bg-green-500"
                  : lcuStatus === "Queue"
                  ? "bg-yellow-300"
                  : "bg-red-500"
              }`}
              title={lcuStatus ?? "Unknown"}
            />
          </div>
        </div>

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden p-2 max-w-7xl w-full mx-auto">
          {/* Left side - Champions grid */}
          <div className="w-1/4 xl:w-1/5 overflow-y-auto border-r min-w-[220px]">
            <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-3 gap-4">
              {filteredChampions.map((champion) => (
                <ChampionCard
                  key={champion.id}
                  champion={champion}
                  isSelected={selectedChampion === champion.id}
                  isFavorite={favorites.has(champion.id)}
                  onToggleFavorite={() => {
                    toggleFavorite(champion.id);
                  }}
                  onClick={() => {
                    console.log(
                      `Selected champion: ${champion.name} (ID: ${champion.id})`
                    );
                    setSelectedChampion(champion.id);
                  }}
                  className="champion-card"
                />
              ))}
            </div>
            {filteredChampions.length === 0 && (
              <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
                <p>No champions found</p>
              </div>
            )}
          </div>

          {/* Right side - Skins grid */}
          <div className="w-3/4 xl:w-4/5 p-4 overflow-y-auto">
            {selectedChampionData ? (
              <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-8">
                {selectedChampionData.skins
                  .filter((skin) => !skin.isBase)
                  .map((skin) => (
                    <SkinCard
                      key={skin.id}
                      championId={selectedChampionData.id}
                      skin={skin}
                    />
                  ))}
              </div>
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                Select a champion to view their skins
              </div>
            )}
          </div>
        </div>
      </div>
    </Suspense>
  );
}
