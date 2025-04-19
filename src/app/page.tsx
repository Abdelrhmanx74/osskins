"use client";

import { useState, useEffect, Suspense } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { DataUpdateModal } from "@/components/DataUpdateModal";
import { ChampionCard } from "@/components/ChampionCard";
import { SkinCard } from "@/components/SkinCard";
import { useChampions } from "@/lib/hooks/use-champions";
import { SkinInjectionButton } from "@/components/skin-injection/SkinInjectionButton";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
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
  const { isUpdating, progress, updateData } = useDataUpdate();
  const { champions, loading, error, hasData } = useChampions();
  const { leaguePath, setLeaguePath } = useGameStore();
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [favorites, setFavorites] = useState<Set<number>>(new Set());
  const [showOnlyFavorites, setShowOnlyFavorites] = useState(false);

  // Handle initial setup
  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        // Load saved league path first
        const savedPath = await invoke<string>("load_league_path");
        if (savedPath && mounted) {
          console.log("Loaded saved League path:", savedPath);
          setLeaguePath(savedPath);
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

    return () => {
      mounted = false;
    };
  }, [isInitialized, updateData, setLeaguePath, hasStartedUpdate]);

  // Save favorites to localStorage when they change
  useEffect(() => {
    if (typeof window !== "undefined" && favorites.size > 0) {
      localStorage.setItem(
        "championFavorites",
        JSON.stringify(Array.from(favorites))
      );
    }
  }, [favorites]);

  // Toggle champion favorite status
  const toggleFavorite = (championId: number) => {
    setFavorites((prev) => {
      const newFavorites = new Set(prev);
      if (newFavorites.has(championId)) {
        newFavorites.delete(championId);
      } else {
        newFavorites.add(championId);
      }
      return newFavorites;
    });
  };

  // Filter champions based on search and favorites
  const filteredChampions = champions.filter((champion) => {
    const matchesSearch = champion.name
      .toLowerCase()
      .includes(searchQuery.toLowerCase());
    const matchesFavorites = !showOnlyFavorites || favorites.has(champion.id);
    return matchesSearch && matchesFavorites;
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
        {/* Top bar with search and injection button */}
        <div className="flex items-center justify-between p-4 border-b">
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
            <Button
              variant="outline"
              size="icon"
              onClick={() => {
                setShowOnlyFavorites(!showOnlyFavorites);
              }}
              className={showOnlyFavorites ? "bg-primary/20" : ""}
            >
              <Heart
                className={`h-4 w-4 ${showOnlyFavorites ? "fill-primary" : ""}`}
              />
            </Button>
          </div>
          <div className="flex items-center gap-4">
            <Button
              onClick={() => void updateData()}
              variant="outline"
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Update Data
            </Button>
            <SkinInjectionButton />
          </div>
        </div>

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left side - Champions grid */}
          <div className="w-1/3 p-4 overflow-y-auto border-r">
            <div className="grid grid-cols-3 gap-4">
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
                    setSelectedChampion(champion.id);
                  }}
                  className="champion-card"
                />
              ))}
            </div>
            {filteredChampions.length === 0 && (
              <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
                <p>No champions found</p>
                {showOnlyFavorites && (
                  <Button
                    variant="link"
                    onClick={() => {
                      setShowOnlyFavorites(false);
                    }}
                  >
                    Clear favorites filter
                  </Button>
                )}
              </div>
            )}
          </div>

          {/* Right side - Skins grid */}
          <div className="w-2/3 p-4 overflow-y-auto">
            {selectedChampionData ? (
              <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-3 gap-6">
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
