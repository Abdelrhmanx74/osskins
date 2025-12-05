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
  onUpdateData: (championsToUpdate?: string[]) => Promise<void>;
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
  const {
    activeTab,
    setActiveTab,
    manualInjectionMode,
    setManualInjectionMode,
  } = useGameStore();
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
            "button, input, [role='button'], [role='combobox']",
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
            <TabsList>
              <div className="relative flex items-center group">
                <TabsTrigger value="official" className="relative pr-8">
                  {t("tabs.official")}
                </TabsTrigger>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      size="icon"
                      variant="ghost"
                      disabled={activeTab !== "official"}
                      className="absolute right-1 top-1/2 -translate-y-1/2 h-5 w-5 p-2 text-white hover:bg-primary dark:hover:bg-primary group-hover:bg-primary"
                    >
                      <ChevronDown className="size-5" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent className="w-40">
                    <DropdownMenuItem
                      onSelect={() => {
                        setManualInjectionMode(false);
                      }}
                    >
                      <Zap className="h-4 w-4 mr-2" />
                      Auto
                      {!manualInjectionMode && (
                        <Check className="h-4 w-4 ml-auto" />
                      )}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onSelect={() => {
                        setManualInjectionMode(true);
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
              </div>
              <TabsTrigger value="custom">{t("tabs.custom")}</TabsTrigger>
            </TabsList>
          </Tabs>
          {/* Show status dot in auto mode, injection button in manual mode */}
          {manualInjectionMode ? (
            <ButtonInjection />
          ) : (
            <InjectionStatusDot showLabel bordered />
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
              <DropdownMenuItem
                onSelect={(event: Event) => {
                  event.preventDefault();
                  setShowCslolModal(true);
                }}
              >
                <RefreshCw className="h-4 w-4" />
                CSLOL Manager
              </DropdownMenuItem>
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
      <CslolManagerModal
        isOpen={showCslolModal}
        onClose={() => {
          setShowCslolModal(false);
        }}
      />
    </div>
  );
}
