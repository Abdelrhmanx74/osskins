"use client";

import { ChampionGrid } from "@/components/ChampionGrid";
import { CustomSkinList } from "@/components/CustomSkinList";
import { DataUpdateModal } from "@/components/DataUpdateModal";
import { ManualSkinGrid } from "@/components/ManualSkinGrid";
import { MiscItemView } from "@/components/MiscItemView";
import { SkinGrid } from "@/components/SkinGrid";
import { AppUpdateBanner } from "@/components/AppUpdateBanner";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { TopBar } from "@/components/layout/TopBar";
import { useChampionPersistence } from "@/lib/hooks/use-champion-persistence";
import { useChampions } from "@/lib/hooks/use-champions";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useI18n } from "@/lib/i18n";
import { type MiscItemType, useGameStore } from "@/lib/store";
import { filterAndSortChampions } from "@/lib/utils/champion-utils";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { Suspense, useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

// Loading component using React 19 suspense
const ChampionsLoader = () => {
  const { t } = useI18n();
  return (
    <div className="flex items-center justify-center h-screen w-full flex-col gap-4">
      <Loader2 className="h-12 w-12 animate-spin" />
      <p className="text-muted-foreground text-lg">
        {t("loading.champions_data")}
      </p>
    </div>
  );
};

export default function Home() {
  const { champions, error, hasData, refreshChampions } = useChampions();
  const { updateData, isUpdating, progress } = useDataUpdate();
  const {
    leaguePath,
    activeTab,
    favorites,
    toggleFavorite,
    manualInjectionMode,
  } = useGameStore();
  const { showUpdateModal, setShowUpdateModal } = useGameStore();

  // Initialize app (load league path, etc)
  // Initialization and config loading are handled globally by AppInitializer

  // Save/load selected champions and favorites
  useChampionPersistence();

  const [searchQuery, setSearchQuery] = useState("");
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);
  const [selectedMiscItem, setSelectedMiscItem] = useState<MiscItemType | null>(
    null,
  );
  const [initialUpdateTriggered, setInitialUpdateTriggered] = useState(false);

  // Handle misc item selection
  const handleMiscItemClick = useCallback((type: MiscItemType) => {
    setSelectedMiscItem(type);
    setSelectedChampion(null); // Clear champion selection when selecting misc item
  }, []);

  // Handle champion selection
  const handleChampionSelect = useCallback((id: number) => {
    setSelectedChampion(id);
    setSelectedMiscItem(null); // Clear misc item selection when selecting champion
  }, []);

  // Filter champions based on search
  const filteredChampions = filterAndSortChampions(
    champions,
    searchQuery,
    favorites,
  );

  // Currently selected champion data
  const selectedChampionData = champions.find(
    (champ) => champ.id === selectedChampion,
  );

  const handleUpdateData = useCallback(async () => {
    try {
      await updateData();
      await refreshChampions();
    } catch (error) {
      console.error("Failed to update data:", error);
      toast.error("Failed to update data");
      throw error instanceof Error ? error : new Error(String(error));
    }
  }, [refreshChampions, updateData]);

  const handleReinstallData = useCallback(async () => {
    await invoke("delete_champions_cache");
    await handleUpdateData();
  }, [handleUpdateData]);

  // If the store requests showing the update modal (e.g., after selecting directory), start the update and show modal
  // We start the update when the flag is set; modal UI is driven by `isUpdating` and `progress`.
  useEffect(() => {
    if (showUpdateModal) {
      // clear the flag and start update
      setShowUpdateModal(false);
      void (async () => {
        try {
          await handleUpdateData();
        } catch (e) {
          console.error("Failed to update data from modal trigger:", e);
        }
      })();
    }
  }, [handleUpdateData, showUpdateModal, setShowUpdateModal]);

  useEffect(() => {
    if (!leaguePath) {
      setInitialUpdateTriggered(false);
      return;
    }

    if (hasData === false && !isUpdating && !initialUpdateTriggered) {
      setInitialUpdateTriggered(true);
      void (async () => {
        try {
          await handleUpdateData();
        } catch (error) {
          console.error("Initial data update failed:", error);
        }
      })();
    }
  }, [
    handleUpdateData,
    hasData,
    initialUpdateTriggered,
    isUpdating,
    leaguePath,
  ]);

  // If no League path is selected, show directory selector
  const { t } = useI18n();

  if (!leaguePath) {
    return (
      <main className="flex min-h-full flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">{t("welcome.title")}</h1>
          <p className="text-muted-foreground">
            {t("welcome.select_league_dir")}
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

  if (isUpdating) {
    return (
      <Suspense fallback={<ChampionsLoader />}>
        <main className="flex min-h-full flex-col items-center justify-center p-24">
          <div className="flex flex-col items-center gap-8">
            <h1 className="text-2xl font-bold">{t("settings.title")}</h1>
            <p className="text-muted-foreground">
              {t("loading.champions_data")}
            </p>
            <DataUpdateModal isOpen={true} progress={progress} />
          </div>
        </main>
      </Suspense>
    );
  }

  if (hasData === false) {
    return <ChampionsLoader />;
  }

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-full w-full">
        <TopBar
          champions={champions}
          selectedChampionId={selectedChampion}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          onChampionSelect={handleChampionSelect}
          onUpdateData={handleUpdateData}
          onReinstallData={handleReinstallData}
          // Disable button while updating handled inside TopBar via props below
          isUpdating={isUpdating}
          progress={progress}
        />
        <AppUpdateBanner />

        {/* Main content */}
        <div className="flex flex-1 overflow-hidden w-full mx-auto">
          {/* Left side - Champions grid - Always visible */}
          <div className="w-1/4 xl:w-1/5 overflow-y-auto scrollbar-hide bg-primary/10 border-r p-2">
            <ChampionGrid
              champions={filteredChampions}
              selectedChampion={selectedChampion}
              favorites={favorites}
              onSelectChampion={handleChampionSelect}
              onToggleFavorite={toggleFavorite}
              isCustomMode={activeTab === "custom"}
              onMiscItemClick={handleMiscItemClick}
            />
          </div>

          {/* Right side - Content: if a misc item type is selected show its view, otherwise show tab-specific content */}
          <div className="w-3/4 xl:w-4/5 flex justify-center overflow-y-auto p-2 size-full">
            {selectedMiscItem ? (
              <MiscItemView type={selectedMiscItem} />
            ) : activeTab === "official" ? (
              manualInjectionMode ? (
                <ManualSkinGrid champion={selectedChampionData ?? null} searchQuery={searchQuery} />
              ) : (
                <SkinGrid champion={selectedChampionData ?? null} searchQuery={searchQuery} />
              )
            ) : (
              <CustomSkinList championId={selectedChampion} />
            )}
          </div>
        </div>
      </div>
    </Suspense>
  );
}
