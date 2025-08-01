"use client";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { RefreshCw, Menu, Users, Users2, Users2Icon } from "lucide-react";
import { toast } from "sonner";
import { InjectionStatusDot } from "@/components/InjectionStatusDot";
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
import { TerminalLogsDialog } from "@/components/TerminalLogsDialog";
import { SettingsDialog } from "@/components/SettingsDialog";
import PartyModeDialog from "@/components/PartyModeDialog";
import { useGameStore, SkinTab } from "@/lib/store";
import { usePartyModeStore } from "@/lib/store/party-mode";
import { useEffect, useState } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Champion } from "@/lib/types";
import { Badge } from "../ui/badge";

interface TopBarProps {
  champions: Champion[];
  selectedChampionId: number | null;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  onChampionSelect: (id: number) => void;
  onUpdateData: () => void;
}

export function TopBar({
  champions,
  selectedChampionId,
  searchQuery,
  onSearchChange,
  onChampionSelect,
  onUpdateData,
}: TopBarProps) {
  // Get tab state from the store
  const { activeTab, setActiveTab } = useGameStore();
  const pairedFriendsCount = usePartyModeStore((s) => s.pairedFriends.length);

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

  // Force update by deleting cache and updating
  async function handleForceUpdateData() {
    try {
      toast.promise(
        async () => {
          // Delete champion cache first
          await invoke("delete_champions_cache");
          // Then run update
          onUpdateData();
        },
        {
          loading: "Clearing cached data...",
          success: "Cache cleared successfully, updating champion data",
          error: "Failed to clear champion cache",
        }
      );
    } catch (error) {
      console.error("Error during force update:", error);
    }
  }

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
            <TabsList>
              <TabsTrigger value="official">Official</TabsTrigger>
              <TabsTrigger value="custom">Custom</TabsTrigger>
            </TabsList>
          </Tabs>
          <InjectionStatusDot />
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
                    {pairedFriendsCount} paired friend
                    {pairedFriendsCount === 1 ? "" : "s"}
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
              {/* Update Data button always visible but disabled in custom tab */}
              <DropdownMenuItem
                onClick={() => {
                  void handleForceUpdateData();
                }}
                onSelect={(e) => {
                  e.preventDefault();
                }}
                disabled={activeTab === "custom"}
              >
                <RefreshCw className="h-4 w-4" />
                Update Data
              </DropdownMenuItem>
              <TerminalLogsDialog />
              <SettingsDialog />
            </DropdownMenuContent>
          </DropdownMenu>
          <TitleBar />
        </div>
      </div>
    </div>
  );
}
