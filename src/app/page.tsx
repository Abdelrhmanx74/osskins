"use client";

import { useState, Suspense, useCallback, useEffect } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useChampions } from "@/lib/hooks/use-champions";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore, MiscItemType } from "@/lib/store";
import { Loader2 } from "lucide-react";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useChampionPersistence } from "@/lib/hooks/use-champion-persistence";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";
import { filterAndSortChampions } from "@/lib/utils/champion-utils";
import { ChampionGrid } from "@/components/ChampionGrid";
import { SkinGrid } from "@/components/SkinGrid";
import { CustomSkinList } from "@/components/CustomSkinList";
import { MiscItemView } from "@/components/MiscItemView";
import { TopBar } from "@/components/layout/TopBar";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import { DataUpdateModal } from "@/components/DataUpdateModal";

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
  const { champions, loading, error, hasData } = useChampions();
  const { updateData, isUpdating, progress } = useDataUpdate();
  const { leaguePath, activeTab, favorites, toggleFavorite } = useGameStore();

  // Initialize app (load league path, etc)
  useInitialization();

  // Load initial config from backend
  useConfigLoader();

  // Save/load selected champions and favorites
  useChampionPersistence();

  const [searchQuery, setSearchQuery] = useState("");
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);
  const [selectedMiscItem, setSelectedMiscItem] = useState<MiscItemType | null>(
    null
  );

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
    favorites
  );

  // Currently selected champion data
  const selectedChampionData = champions.find(
    (champ) => champ.id === selectedChampion
  );

  const handleUpdateData = async () => {
    try {
      // Incremental update without clearing cache
      await updateData();
    } catch (error) {
      console.error("Failed to update data:", error);
      toast.error("Failed to update data");
    }
  };

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

  if (loading) {
    return <ChampionsLoader />;
  }

  if (hasData === false) {
    return (
      <main className="flex min-h-full flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">{t("welcome.title")}</h1>
          <p className="text-muted-foreground">{t("loading.champions_data")}</p>
          <button
            onClick={() => {
              void handleUpdateData();
            }}
            className="bg-primary text-white px-4 py-2 rounded"
          >
            {t("update.action")}
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

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-full w-full">
        <TopBar
          champions={champions}
          selectedChampionId={selectedChampion}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          onChampionSelect={handleChampionSelect}
          onUpdateData={() => {
            void updateData();
          }}
          // Disable button while updating handled inside TopBar via props below
          isUpdating={isUpdating}
        />

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

          {/* Right side - Content based on active tab */}
          <div className="w-3/4 xl:w-4/5 flex justify-center overflow-y-auto p-2 size-full">
            {activeTab === "official" ? (
              <SkinGrid champion={selectedChampionData ?? null} />
            ) : selectedMiscItem ? (
              <MiscItemView type={selectedMiscItem} />
            ) : (
              <CustomSkinList championId={selectedChampion} />
            )}
          </div>
        </div>
      </div>
    </Suspense>
  );
}
