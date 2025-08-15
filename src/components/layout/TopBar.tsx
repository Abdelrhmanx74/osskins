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
import { useEffect, useMemo, useState } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Champion } from "@/lib/types";
import { Badge } from "../ui/badge";

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
  const { activeTab, setActiveTab } = useGameStore();
  const pairedFriendsCount = usePartyModeStore((s) => s.pairedFriends.length);

  // Track update availability
  const [isChecking, setIsChecking] = useState(false);
  const [isUpToDate, setIsUpToDate] = useState<boolean | null>(null);

  const refreshUpdateAvailability = useMemo(
    () => async () => {
      try {
        setIsChecking(true);
        const info = await invoke<{
          success: boolean;
          updatedChampions?: string[];
        }>("check_data_updates");
        const hasNew = (info.updatedChampions?.length ?? 0) > 0;
        setIsUpToDate(!hasNew);
      } catch (e) {
        // On failure, do not disable the button
        setIsUpToDate(null);
      } finally {
        setIsChecking(false);
      }
    },
    []
  );

  useEffect(() => {
    void refreshUpdateAvailability();
  }, [refreshUpdateAvailability]);

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
      // After update, refresh availability to reflect up-to-date state
      await refreshUpdateAvailability();
    } catch (error) {
      console.error("Error during manual update:", error);
      toast.error("Failed to update data");
    }
  }

  const updateDisabled =
    activeTab === "custom" || isUpdating || isChecking || isUpToDate === true;

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
                {isUpToDate === true
                  ? "Up to date"
                  : isChecking
                  ? "Checking..."
                  : isUpdating
                  ? "Updating..."
                  : "Update Data"}
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
