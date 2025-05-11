"use client";

import { useState, useEffect, Suspense } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useChampions } from "@/lib/hooks/use-champions";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore } from "@/lib/store";
import { Loader2 } from "lucide-react";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useChampionPersistence } from "@/lib/hooks/use-champion-persistence";
import { filterAndSortChampions } from "@/lib/utils/champion-utils";
import ChampionGrid from "@/components/ChampionGrid";
import { SkinGrid } from "@/components/SkinGrid";
import { CustomSkinList } from "@/components/CustomSkinList";
import { TopBar } from "@/components/layout/TopBar";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { DataUpdateModal } from "@/components/DataUpdateModal";
import { Button } from "@/components/ui/button";

interface UpdateInfo {
  has_update: boolean;
}

function PageLoading() {
  return (
    <div className="flex items-center justify-center h-screen w-full flex-col gap-4">
      <Loader2 className="h-12 w-12 animate-spin" />
      <p className="text-muted-foreground text-lg">Loading champions data...</p>
    </div>
  );
}

export default function Home() {
  const { champions, loading, error, hasData } = useChampions();
  const { updateData, isUpdating, progress } = useDataUpdate();
  const { leaguePath, activeTab, favorites, toggleFavorite } = useGameStore();
  const [isUpdateModalOpen, setIsUpdateModalOpen] = useState(false);
  const autoUpdateData = useGameStore((s) => s.autoUpdateData);

  // Initialize app
  useInitialization();
  useChampionPersistence();

  const [searchQuery, setSearchQuery] = useState("");
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);

  const filteredChampions = filterAndSortChampions(
    champions,
    searchQuery,
    favorites
  );
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

  const handleCheckForUpdates = () => {
    setIsUpdateModalOpen(true);
  };

  useEffect(() => {
    if (autoUpdateData) {
      // Only run on mount
      void (async () => {
        try {
          const updateResult: UpdateInfo = await invoke("check_github_updates");
          if (updateResult.has_update) {
            await invoke("update_champion_data_from_github");
            toast.success("Champion data auto-updated!");
          }
        } catch (err) {
          // Optionally show a toast for errors
        }
      })();
    }
  }, [autoUpdateData]);

  // Initial setup states
  if (!leaguePath) {
    return <GameDirectorySelector />;
  }

  if (hasData === false) {
    return (
      <main className="flex min-h-full flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Welcome to League Skin Manager</h1>
          <p className="text-muted-foreground">
            We need to download champion data before you can use the app
          </p>
          <Button onClick={() => void handleUpdateData()}>
            Download Champion Data
          </Button>
        </div>
      </main>
    );
  }

  return (
    <div className="flex flex-col h-full w-full">
      <TopBar
        champions={champions}
        selectedChampionId={selectedChampion}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        onChampionSelect={setSelectedChampion}
        onUpdateData={handleCheckForUpdates}
      />

      {/* Update Modal - visible when updating or when manually opened */}
      <DataUpdateModal
        isOpen={isUpdating || isUpdateModalOpen}
        progress={progress}
        onClose={() => {
          setIsUpdateModalOpen(false);
        }}
      />

      <Suspense fallback={<PageLoading />}>
        <div className="flex flex-1 overflow-hidden w-full mx-auto">
          <div className="w-1/4 xl:w-1/5 overflow-y-auto scrollbar-hide bg-primary/10 border-r p-2">
            <ChampionGrid
              champions={filteredChampions}
              selectedChampion={selectedChampion}
              favorites={favorites}
              onSelectChampion={setSelectedChampion}
              onToggleFavorite={toggleFavorite}
            />
          </div>

          <div className="w-3/4 xl:w-4/5 flex justify-center overflow-y-auto p-2 size-full">
            {activeTab === "official" ? (
              <SkinGrid champion={selectedChampionData ?? null} />
            ) : (
              <CustomSkinList championId={selectedChampion} />
            )}
          </div>
        </div>
      </Suspense>
    </div>
  );
}
