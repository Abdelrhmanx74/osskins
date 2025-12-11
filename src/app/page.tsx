"use client";

import { ChampionGrid } from "@/components/ChampionGrid";
import { CustomSkinList } from "@/components/CustomSkinList";
import { DataUpdateModal } from "@/components/download/DataUpdateModal";
import { ManualSkinGrid } from "@/components/ManualSkinGrid";
import { MiscItemView } from "@/components/MiscItemView";
import { SkinGrid } from "@/components/SkinGrid";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { TopBar } from "@/components/layout/TopBar";
import { useChampionPersistence } from "@/lib/hooks/use-champion-persistence";
import { useChampions } from "@/lib/hooks/use-champions";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useI18n } from "@/lib/i18n";
import { type MiscItemType, useGameStore } from "@/lib/store";
import { filterAndSortChampions } from "@/lib/utils/champion-utils";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "framer-motion";
import { Loader2, RefreshCw } from "lucide-react";
import { Suspense, useCallback, useEffect, useState, useMemo } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

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
  // Use individual selectors to avoid creating new objects on every render
  const leaguePath = useGameStore((state) => state.leaguePath);
  const activeTab = useGameStore((state) => state.activeTab);
  const favorites = useGameStore((state) => state.favorites);
  const toggleFavorite = useGameStore((state) => state.toggleFavorite);
  const manualInjectionMode = useGameStore((state) => state.manualInjectionMode);
  const showUpdateModal = useGameStore((state) => state.showUpdateModal);
  const setShowUpdateModal = useGameStore((state) => state.setShowUpdateModal);

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
  // If an initial update fails we allow the UI to proceed. This flag is set
  // when an update attempt errors so rendering can avoid blocking the user.
  const [updateFailed, setUpdateFailed] = useState(false);

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

  // Filter champions based on search - memoized
  const filteredChampions = useMemo(
    () => filterAndSortChampions(champions, searchQuery, favorites),
    [champions, searchQuery, favorites]
  );

  // Currently selected champion data - memoized
  const selectedChampionData = useMemo(
    () => champions.find((champ) => champ.id === selectedChampion),
    [champions, selectedChampion]
  );

  const handleUpdateData = useCallback(async () => {
    // Clear previous failure state when starting a new attempt
    setUpdateFailed(false);
    try {
      await updateData();
      await refreshChampions();
      // Success: ensure failure flag is cleared
      setUpdateFailed(false);
    } catch (error) {
      // Mark that update failed so UI won't remain blocked
      setUpdateFailed(true);
      console.error("Failed to update data:", error);
      // Show an actionable message but don't rethrow - letting callers continue
      const msg = error instanceof Error ? error.message : String(error);
      toast.error("Failed to update data: " + msg);
      // Do not rethrow here so that initial startup and modal triggers
      // won't lock the renderer into a blocking state.
    }
  }, [refreshChampions, updateData]);

  const handleReinstallData = useCallback(async () => {
    setUpdateFailed(false);
    try {
      // Clear the stored commit so update check doesn't skip
      try {
        await invoke("set_last_data_commit", { sha: null, manifestJson: null });
      } catch (e) {
        console.warn("Failed to clear last commit:", e);
      }
      await invoke("delete_champions_cache");
      // Force update to bypass any cached state checks
      await updateData(undefined, { force: true });
      await refreshChampions();
      setUpdateFailed(false);
    } catch (error) {
      setUpdateFailed(true);
      console.error("Failed to reinstall data:", error);
      const msg = error instanceof Error ? error.message : String(error);
      toast.error("Failed to reinstall data: " + msg);
    }
  }, [updateData, refreshChampions]);

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

  // While an update is in progress we show the progress modal, but if the
  // update has already failed we allow the rest of the app to render so the
  // user is not locked out. The modal UI is still available via the TopBar.
  if (isUpdating && !updateFailed) {
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

  // If there is no existing data we normally show the loader. However if an
  // update attempt failed, allow the user to proceed into the app (so they
  // can fix settings/select directory) while the error is shown via toast.
  if (hasData === false && !updateFailed) {
    return (
      <div className="flex items-center justify-center h-screen w-full flex-col gap-4">
        <Loader2 className="h-12 w-12 animate-spin" />
        <p className="text-muted-foreground text-lg">
          {t("loading.champions_data")}
        </p>
        {initialUpdateTriggered && !isUpdating && (
          <div className="flex flex-col items-center gap-2 mt-4">
            <p className="text-sm text-muted-foreground">
              {t("update.stuck_hint") || "Update seems stuck?"}
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setInitialUpdateTriggered(false);
                setUpdateFailed(false);
                void handleReinstallData();
              }}
            >
              <RefreshCw className="h-4 w-4 mr-2" />
              {t("update.retry") || "Retry Download"}
            </Button>
          </div>
        )}
      </div>
    );
  }

  return (
    <Suspense fallback={<ChampionsLoader />}>
      <div className="flex flex-col h-full w-full squircle overflow-hidden bg-background text-foreground">
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
        {/* AppUpdateBanner removed */}

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
              disableLayout={searchQuery.trim().length > 0}
            />
          </div>

          {/* Right side - Content: if a misc item type is selected show its view, otherwise show tab-specific content */}
          <div className="w-3/4 xl:w-4/5 flex justify-center overflow-y-auto p-2 size-full">
            <AnimatePresence mode="wait">
              {selectedMiscItem ? (
                <motion.div
                  key={`misc-${selectedMiscItem}`}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.2 }}
                  style={{ width: "100%" }}
                >
                  <MiscItemView type={selectedMiscItem} />
                </motion.div>
              ) : activeTab === "official" ? (
                <motion.div
                  key={`official-${selectedChampion ?? "none"}-${manualInjectionMode}`}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.2 }}
                  style={{ width: "100%" }}
                >
                  {manualInjectionMode ? (
                    <ManualSkinGrid
                      champion={selectedChampionData ?? null}
                      searchQuery={searchQuery}
                    />
                  ) : (
                    <SkinGrid
                      champion={selectedChampionData ?? null}
                      searchQuery={searchQuery}
                    />
                  )}
                </motion.div>
              ) : (
                <motion.div
                  key={`custom-${selectedChampion ?? "none"}`}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.2 }}
                  style={{ width: "100%" }}
                >
                  <CustomSkinList championId={selectedChampion} />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>
    </Suspense>
  );
}
