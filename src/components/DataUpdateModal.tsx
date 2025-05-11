"use client";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { DataUpdateProgress, DataUpdateResult } from "@/lib/types";
import { useEffect, useState, useTransition } from "react";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";

interface DataUpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
  progress: DataUpdateProgress | null;
}

export function DataUpdateModal({
  isOpen,
  onClose,
  progress,
}: DataUpdateModalProps) {
  const [isPending, startTransition] = useTransition();
  const [updateResult, setUpdateResult] = useState<DataUpdateResult | null>(
    null
  );
  const [checkingForUpdates, setCheckingForUpdates] = useState(false);
  const [updatingData, setUpdatingData] = useState(false);

  // Check for GitHub updates when the modal is opened
  useEffect(() => {
    if (isOpen) {
      checkForUpdates();
    }
  }, [isOpen]);

  // Function to check for updates from GitHub
  const checkForUpdates = () => {
    setCheckingForUpdates(true);

    startTransition(async () => {
      try {
        const result = await invoke<DataUpdateResult>("check_github_updates");
        setUpdateResult(result);
        console.log("Update check result:", result);
      } catch (error) {
        console.error("Failed to check for updates:", error);
        toast.error("Failed to check for updates");
      } finally {
        setCheckingForUpdates(false);
      }
    });
  };

  // Function to pull updates from GitHub
  const pullUpdates = () => {
    setUpdatingData(true);

    startTransition(async () => {
      try {
        const updateToast = toast.loading("Updating data from GitHub...");
        const result = await invoke<DataUpdateResult>(
          "update_champion_data_from_github"
        );

        toast.dismiss(updateToast);

        if (result.success) {
          toast.success(
            `Update completed successfully! ${result.updated_champions.length} champions updated.`
          );
          setUpdateResult(result);
        } else {
          toast.error(`Update failed: ${result.error || "Unknown error"}`);
        }
      } catch (error) {
        console.error("Failed to update data:", error);
        toast.error(`Failed to update data: ${error}`);
      } finally {
        setUpdatingData(false);
      }
    });
  };

  const getStatusMessage = () => {
    if (checkingForUpdates) return "Checking for updates...";
    if (updatingData) return "Updating data...";
    if (!progress) return "Initializing...";

    switch (progress.status) {
      case "checking":
        return "Checking for updates...";
      case "downloading":
        return "Downloading updates...";
      case "processing":
        return `Processing ${progress.currentChampion}...`;
      default:
        return "Processing updates...";
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-md">
        <div className="flex flex-col space-y-4">
          <DialogHeader>
            <DialogTitle>Data Updates</DialogTitle>
            <p className="text-sm text-muted-foreground animate-in fade-in-50 duration-300">
              {getStatusMessage()}
            </p>
          </DialogHeader>

          {/* Regular progress indicator for initial data download */}
          {progress && !updateResult && (
            <div className="space-y-2 animate-in fade-in-50 duration-300">
              <Progress
                value={progress.progress}
                className={`transition-all duration-300 ${
                  isPending ? "opacity-50" : "opacity-100"
                }`}
              />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{Math.round(progress.progress)}%</span>
                <span>
                  {progress.processedChampions} of {progress.totalChampions}{" "}
                  champions
                </span>
              </div>
              <p className="text-xs text-muted-foreground text-right animate-in slide-in-from-right-5">
                {progress.currentChampion &&
                  `Currently processing: ${progress.currentChampion}`}
              </p>
            </div>
          )}

          {/* GitHub update UI */}
          {updateResult && !progress && (
            <div className="space-y-3 animate-in fade-in-50 duration-300">
              <div className="grid grid-cols-2 gap-2 text-sm">
                <span className="text-muted-foreground">Current Version:</span>
                <span className="font-mono">
                  {updateResult.current_version || "Not installed"}
                </span>

                <span className="text-muted-foreground">Latest Version:</span>
                <span className="font-mono">
                  {updateResult.available_version || "Unknown"}
                </span>
              </div>

              {updateResult.has_update ? (
                <div className="rounded-md bg-muted/50 p-3 border border-border">
                  <p className="text-sm font-medium mb-1">Update Available</p>
                  <p className="text-xs text-muted-foreground mb-3">
                    {updateResult.update_message ||
                      "New data updates are available for download."}
                  </p>
                  <Button
                    size="sm"
                    className="w-full"
                    disabled={updatingData}
                    onClick={pullUpdates}
                  >
                    {updatingData ? "Updating..." : "Update Now"}
                  </Button>
                </div>
              ) : (
                <div className="rounded-md bg-muted/50 p-3 border border-border">
                  <p className="text-sm font-medium flex items-center gap-2">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className="text-green-500"
                    >
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                    Data is up to date
                  </p>
                  <p className="text-xs text-muted-foreground mt-1">
                    You have the latest champion data installed.
                  </p>
                </div>
              )}
            </div>
          )}

          {/* Loading state */}
          {(checkingForUpdates || updatingData) &&
            !progress &&
            !updateResult && (
              <div className="flex flex-col items-center justify-center py-4">
                <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-primary mb-2"></div>
                <p className="text-sm text-muted-foreground">
                  {checkingForUpdates
                    ? "Checking for updates..."
                    : "Updating data..."}
                </p>
              </div>
            )}
        </div>

        <DialogFooter className="mt-4">
          <Button
            variant="outline"
            size="sm"
            onClick={checkForUpdates}
            disabled={checkingForUpdates || updatingData}
          >
            Check for Updates
          </Button>
          <Button size="sm" onClick={onClose} disabled={updatingData}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
