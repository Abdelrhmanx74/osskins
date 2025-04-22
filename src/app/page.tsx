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
import { Loader2, RefreshCw, Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import {
  GameStatusDot,
  // InjectionStatusDot,
} from "@/components/skin-injection/GameStatusDot";
import { toast } from "sonner";
import { OnboardingTour } from "@/components/onboarding/OnboardingTour";
import { HelpButton } from "@/components/onboarding/HelpButton";

// Loading component using React 19 suspense
const ChampionsLoader = () => (
  <div className="flex flex-col items-center justify-center h-screen">
    <Loader2 className="animate-spin size-20 text-muted-foreground" />
  </div>
);

export default function Home() {
  const { isUpdating, progress, updateData } = useDataUpdate();
  const { champions, loading, error, hasData } = useChampions();
  const {
    leaguePath,
    setLeaguePath,
    selectSkin,
    setLcuStatus,
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
          void invoke("start_auto_inject", { leaguePath: league_path }); // Use camelCase parameter name
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
    updateData,
    setLeaguePath,
    hasStartedUpdate,
    selectSkin,
    setLcuStatus,
    setFavorites,
  ]);

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

  if (hasData === false) {
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

  function handleUpdateDataClick() {
    void (async () => {
      try {
        // Delete the existing cache
        await invoke("delete_champions_cache");

        // Force a data update
        await updateData();
      } catch (error) {
        console.error("Failed to update data:", error);
        toast.error("Failed to update data");
      }
    })();
  }

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-screen bg-background">
        {/* Onboarding component */}
        <OnboardingTour />

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
          </div>
          <div className="flex items-center gap-4">
            <HelpButton />
            <Button
              onClick={handleUpdateDataClick}
              variant="outline"
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Update Data
            </Button>
            {/* status dot */}
            <GameStatusDot />
          </div>
        </div>

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden p-2 max-w-7xl w-full mx-auto">
          {/* Left side - Champions grid */}
          <div className="w-1/4 xl:w-1/5 overflow-y-auto border-r min-w-[220px]">
            <div className="w-fit mx-auto grid grid-cols-2 md:grid-cols-3 xl:grid-cols-3 gap-4">
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
          <div className="w-3/4 xl:w-4/5 flex justify-center p-4 overflow-y-auto">
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
