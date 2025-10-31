"use client";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { RefreshCw, Menu, Users, Users2, Users2Icon } from "lucide-react";
import { toast } from "sonner";
import { InjectionStatusDot } from "@/components/InjectionStatusDot";
import { ButtonInjection } from "@/components/button-injection";
import { TitleBar } from "@/components/ui/titlebar/TitleBar";
import { ChampionSearch } from "@/components/ChampionSearch";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
// Print logs moved into Settings dialog
import { SettingsDialog } from "@/components/SettingsDialog";
import PartyModeDialog from "@/components/PartyModeDialog";
import { useGameStore, SkinTab } from "@/lib/store";
import { usePartyModeStore } from "@/lib/store/party-mode";
import { useEffect, useMemo, useState } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Champion } from "@/lib/types";
import { Badge } from "../ui/badge";
import { useI18n } from "@/lib/i18n";

interface TopBarProps {
  champions: Champion[];
  selectedChampionId: number | null;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onChampionSelect: (id: number) => void;
  onUpdateData: () => void | Promise<void>;
  isUpdating?: boolean;
}

export function TopBar({
  champions,
  selectedChampionId,
  searchQuery,
  onSearchChange,
  onChampionSelect,
  onUpdateData,
  isUpdating = false,
}: TopBarProps) {
  // Get tab state from the store
  const { activeTab, setActiveTab, manualInjectionMode } = useGameStore();
  const pairedFriendsCount = usePartyModeStore((s) => s.pairedFriends.length);

  // No availability probe: update button is enabled unless updating or on custom tab
  const isChecking = false;
  const isUpToDate: boolean | null = null;

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

  // Manual update without clearing cache
  async function handleManualUpdate() {
    try {
      await onUpdateData();
      // no availability probe
    } catch (error) {
      console.error("Error during manual update:", error);
      toast.error(t("update.processing_unknown"));
    }
  }

  const updateDisabled = activeTab === "custom" || isUpdating;
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
              <TabsTrigger value="official">{t("tabs.official")}</TabsTrigger>
              <TabsTrigger value="custom">{t("tabs.custom")}</TabsTrigger>
            </TabsList>
          </Tabs>
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
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon" aria-label="Menu">
                <Menu className="h-5 w-5" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="min-w-50" align="end">
              <PartyModeDialog />
              {/* Manual Update Data (incremental) */}
              <DropdownMenuItem
                onClick={() => {
                  void handleManualUpdate();
                }}
                onSelect={(e) => {
                  e.preventDefault();
                }}
                disabled={updateDisabled}
              >
                <RefreshCw className="h-4 w-4" />
                {isUpdating ? t("update.downloading") : t("update.action")}
              </DropdownMenuItem>
              <SettingsDialog />
            </DropdownMenuContent>
          </DropdownMenu>
          <TitleBar />
        </div>
      </div>
    </div>
  );
}
