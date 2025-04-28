"use client";

import { useState, Suspense, useCallback, useEffect } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useChampions } from "@/lib/hooks/use-champions";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore } from "@/lib/store";
import { Loader2 } from "lucide-react";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useChampionPersistence } from "@/lib/hooks/use-champion-persistence";
import { filterAndSortChampions } from "@/lib/utils/champion-utils";
import { ChampionGrid } from "@/components/ChampionGrid";
import { SkinGrid } from "@/components/SkinGrid";
import { CustomSkinList } from "@/components/CustomSkinList";
import { TopBar } from "@/components/layout/TopBar";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DataUpdateModal } from "@/components/DataUpdateModal";

// Loading component using React 19 suspense
const ChampionsLoader = () => (
  <div className="flex items-center justify-center h-screen w-full flex-col gap-4">
    <Loader2 className="h-12 w-12 animate-spin" />
    <p className="text-muted-foreground text-lg">Loading champions data...</p>
  </div>
);

export default function Home() {
  const { champions, loading, error, hasData } = useChampions();
  const { updateData, isUpdating, progress } = useDataUpdate();
  const { leaguePath, activeTab, favorites, toggleFavorite } = useGameStore();

  // Initialize app (load league path, etc)
  useInitialization();

  // Save/load selected champions and favorites
  useChampionPersistence();

  const [searchQuery, setSearchQuery] = useState("");
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);

  // Filter champions based on search
  const filteredChampions = filterAndSortChampions(
    champions,
    searchQuery,
    favorites
  );

  // Currently selected champion data
  const selectedChampionData = champions.find(
    (champ) => champ.id === selectedChampion
  );

  const handleUpdateData = async () => {
    try {
      await invoke("delete_champions_cache");
      await updateData();
    } catch (error) {
      console.error("Failed to update data:", error);
      toast.error("Failed to update data");
    }
  };

  // If no League path is selected, show directory selector
  if (!leaguePath) {
    return (
      <main className="flex min-h-full flex-col items-center justify-center p-24">
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
      <main className="flex min-h-full flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center justify-center h-full gap-4">
          <div className="text-destructive">Error: {error}</div>
        </div>
      </main>
    );
  }

  if (loading) {
    return <ChampionsLoader />;
  }

  if (hasData === false) {
    return (
      <main className="flex min-h-full flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Welcome to League Skin Manager</h1>
          <p className="text-muted-foreground">
            We need to download champion data before you can use the app
          </p>
          <button
            onClick={() => {
              void handleUpdateData();
            }}
            className="bg-primary text-white px-4 py-2 rounded"
          >
            Download Champion Data
          </button>
        </div>
      </main>
    );
  }

  if (isUpdating) {
    return (
      <Suspense fallback={<ChampionsLoader />}>
        <main className="flex min-h-full flex-col items-center justify-center p-24">
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

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-full w-full">
        <TopBar
          champions={champions}
          selectedChampionId={selectedChampion}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          onChampionSelect={setSelectedChampion}
          onUpdateData={() => {
            void updateData();
          }}
        />

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden w-full mx-auto">
          {/* Left side - Champions grid - Always visible */}
          <div className="w-1/4 xl:w-1/5 overflow-y-auto scrollbar-hide bg-primary/10 border-r p-2">
            <ChampionGrid
              champions={filteredChampions}
              selectedChampion={selectedChampion}
              favorites={favorites}
              onSelectChampion={setSelectedChampion}
              onToggleFavorite={toggleFavorite}
            />
          </div>

          {/* Right side - Content based on active tab */}
          <div className="w-3/4 xl:w-4/5 flex justify-center overflow-y-auto p-2 size-full">
            {activeTab === "official" ? (
              <SkinGrid champion={selectedChampionData ?? null} />
            ) : (
              <CustomSkinList championId={selectedChampion} />
            )}
          </div>
        </div>
      </div>
    </Suspense>
  );
}
