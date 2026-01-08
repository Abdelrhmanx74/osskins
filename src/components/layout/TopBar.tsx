"use client";

import { ChampionSearch } from "@/components/ChampionSearch";
import { DownloadingModal } from "@/components/download/DownloadingModal";
import { InjectionStatusDot } from "@/components/InjectionStatusDot";
import { ButtonInjection } from "@/components/button-injection";
import PartyModeDialog from "@/components/PartyModeDialog";
// Print logs moved into Settings dialog
import { SettingsDialog } from "@/components/SettingsDialog";
// manual injection control is accessible from the official tab select
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { TitleBar } from "@/components/ui/titlebar/TitleBar";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useI18n } from "@/lib/i18n";
import { type SkinTab, useGameStore } from "@/lib/store";
import { usePartyModeStore } from "@/lib/store/party-mode";
import type { Champion, DataUpdateProgress } from "@/lib/types";
import {
  Menu,
  RefreshCw,
  Users2Icon,
  ArrowDownToLine,
  Sparkles,
  Check,
  Zap,
  Hand,
  ChevronDown,
} from "lucide-react";
import { useEffect, useState } from "react";
import CslolManagerModal from "@/components/CslolManagerModal";
import { Badge } from "../ui/badge";

interface TopBarProps {
  champions: Champion[];
  selectedChampionId: number | null;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onChampionSelect: (id: number) => void;
  onUpdateData: () => Promise<void>;
  onReinstallData: () => Promise<void>;
  isUpdating?: boolean;
  progress: DataUpdateProgress | null;
}

export function TopBar({
  champions,
  selectedChampionId,
  searchQuery,
  onSearchChange,
  onChampionSelect,
  onUpdateData,
  onReinstallData,
  isUpdating = false,
  progress,
}: TopBarProps) {
  const [showDownloadingModal, setShowDownloadingModal] = useState(false);
  const [showCslolModal, setShowCslolModal] = useState(false);
  // Get tab state from the store
  const activeTab = useGameStore((state) => state.activeTab);
  const setActiveTab = useGameStore((state) => state.setActiveTab);
  const manualInjectionMode = useGameStore(
    (state) => state.manualInjectionMode
  );
  const setManualInjectionMode = useGameStore(
    (state) => state.setManualInjectionMode
  );
  const pairedFriendsCount = usePartyModeStore((s) => s.pairedFriends.length);
  // Updater removed: no updater store or hook

  // Load saved tab preference from localStorage
  useEffect(() => {
    if (typeof window !== "undefined") {
      const savedTab = localStorage.getItem("activeSkinsTab") as SkinTab | null;
      if (savedTab) {
        setActiveTab(savedTab);
      }
    }
  }, [setActiveTab]);

  // Load manual injection mode from backend on mount
  useEffect(() => {
    const loadManualMode = async () => {
      try {
        const { manualInjectionApi } = await import("@/lib/api/manual-injection");
        const backendMode = await manualInjectionApi.getManualInjectionMode();
        // Only update if different to avoid unnecessary re-renders
        if (backendMode !== manualInjectionMode) {
          setManualInjectionMode(backendMode);
        }
      } catch (error) {
        console.error("Failed to load manual injection mode from backend:", error);
      }
    };

    void loadManualMode();
  }, []); // Only on mount

  // Load paired friends count
  useEffect(() => {
    // Party mode is now handled by the provider, no need for manual loading
  }, []);

  const updateDisabled = isUpdating;
  const { t } = useI18n();

  return (
    <div
      data-tauri-drag-region
      onMouseDown={(e) => {
        if (
          (e.target as HTMLElement).closest("[data-tauri-drag-region]") &&
          !(e.target as HTMLElement).closest(
            "button, input, [role='button'], [role='combobox']"
          )
        ) {
          // Use the WebviewWindow API for window dragging
          import("@tauri-apps/api/webviewWindow")
            .then(({ getCurrentWebviewWindow }) => {
              const appWindow = getCurrentWebviewWindow();
              appWindow.startDragging().catch((error: unknown) => {
                console.error("Failed to start dragging:", error);
              });
            })
            .catch((error: unknown) => {
              console.error(error);
            });
        }
      }}
      className="flex flex-col w-full mx-auto bg-primary/10 border-b"
    >
      <div className="flex items-center justify-between gap-4 p-2 w-full mx-auto">
        <div className="relative flex items-center w-1/3 xl:w-1/4">
          <ChampionSearch
            champions={champions}
            onSelect={onChampionSelect}
            selectedChampionId={selectedChampionId}
            searchQuery={searchQuery}
            onSearchChange={onSearchChange}
          />
        </div>
        <div className="w-full flex items-center gap-4 z-10">
          <Tabs
            value={activeTab}
            onValueChange={(value) => {
              setActiveTab(value as SkinTab);
            }}
            className="w-full justify-center items-center"
          >
            <TabsList className="flex items-center gap-1 pr-1">
              <TabsTrigger value="official">{t("tabs.official")}</TabsTrigger>
              <TabsTrigger value="custom">{t("tabs.custom")}</TabsTrigger>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-full w-fit px-1 text-muted-foreground hover:bg-primary/30 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
                    aria-label="Toggle injection mode"
                  >
                    <ChevronDown className="size-5" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent className="w-40">
                  <DropdownMenuItem
                    onSelect={async () => {
                      setManualInjectionMode(false);
                      try {
                        const { manualInjectionApi } = await import("@/lib/api/manual-injection");
                        await manualInjectionApi.setManualInjectionMode(false);
                      } catch (error) {
                        console.error("Failed to sync manual injection mode to backend:", error);
                      }
                    }}
                  >
                    <Zap className="h-4 w-4 mr-2" />
                    Auto
                    {!manualInjectionMode && (
                      <Check className="h-4 w-4 ml-auto" />
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onSelect={async () => {
                      setManualInjectionMode(true);
                      try {
                        const { manualInjectionApi } = await import("@/lib/api/manual-injection");
                        await manualInjectionApi.setManualInjectionMode(true);
                      } catch (error) {
                        console.error("Failed to sync manual injection mode to backend:", error);
                      }
                    }}
                  >
                    <Hand className="h-4 w-4 mr-2" />
                    Manual
                    {manualInjectionMode && (
                      <Check className="h-4 w-4 ml-auto" />
                    )}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </TabsList>
          </Tabs>
          {/* Show status dot in auto mode, injection button in manual mode */}
          {manualInjectionMode ? (
            <ButtonInjection />
          ) : (
            <InjectionStatusDot bordered />
          )}
          {/* Party Mode indicator */}
          {pairedFriendsCount > 0 && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Badge
                    variant="default"
                    className="gap-2 text-sm font-bold cursor-default"
                  >
                    {pairedFriendsCount}
                    <Users2Icon className="size-4" />
                  </Badge>
                </TooltipTrigger>
                <TooltipContent>
                  <p>
                    {pairedFriendsCount}{" "}
                    {pairedFriendsCount === 1
                      ? t("party.pairedFriend")
                      : t("party.pairedFriends")}
                  </p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
          {/* actions are available inside the menu only (no external buttons) */}

          {/* dropdown opens on hover */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                aria-label="Menu"
                className="relative"
              >
                <Menu className="h-5 w-5" />
                {/* Updater removed */}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="min-w-50" align="end">
              <PartyModeDialog />
              <DropdownMenuItem
                onSelect={(event: Event) => {
                  event.preventDefault();
                  setShowDownloadingModal(true);
                }}
                disabled={updateDisabled}
              >
                <RefreshCw className="h-4 w-4" />
                {t("menu.checkDataUpdates")}
              </DropdownMenuItem>
              {/* CSLOL Manager button removed */}
              {/* Updater menu items removed */}
              <SettingsDialog />
            </DropdownMenuContent>
          </DropdownMenu>
          <TitleBar />
        </div>
      </div>
      <DownloadingModal
        isOpen={showDownloadingModal}
        onClose={() => {
          setShowDownloadingModal(false);
        }}
        progress={progress}
        onUpdateData={onUpdateData}
        onReinstallData={onReinstallData}
        isUpdating={isUpdating}
      />
      {/* CslolManagerModal removed */}
    </div>
  );
}
