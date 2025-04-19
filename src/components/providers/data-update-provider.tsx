"use client";

import {
  createContext,
  useContext,
  useState,
  useTransition,
  useCallback,
} from "react";
import { DataUpdateProgress } from "@/lib/types";
import { toast } from "sonner";
import { useFormStatus } from "react-dom";
import { invoke } from "@tauri-apps/api/core";

interface DataUpdateContextType {
  isUpdating: boolean;
  progress: DataUpdateProgress | null;
  updateData: () => Promise<void>;
}

const DataUpdateContext = createContext<DataUpdateContextType | null>(null);

export function DataUpdateProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<DataUpdateProgress | null>(null);
  const [isPending, startTransition] = useTransition();
  const { pending: formPending } = useFormStatus();

  // Function to check for updates without starting the update process
  // const checkForUpdatesOnly = useCallback(async (): Promise<boolean> => {
  //   try {
  //     // Implementation goes here
  //   } catch (error) {
  //     console.error("Failed to check for updates:", error);
  //     // Don't show error toast during initial app load as it may be expected
  //     if (isUpdating) {
  //       toast.error("Failed to check for updates");
  //     }
  //     return false;
  //   }
  // }, [isUpdating]);

  // Main update function
  const updateData = useCallback(async () => {
    if (isUpdating) {
      return;
    }

    const loadingToastId = toast("Checking for updates...");

    try {
      setIsUpdating(true);
      setProgress({
        currentChampion: "",
        totalChampions: 0,
        processedChampions: 0,
        status: "checking",
        progress: 0,
      });

      // Start the update process with server actions
      startTransition(async () => {
        try {
          // First check if any data exists at all
          const dataExists = await invoke<boolean>("check_champions_data");

          // Check if updates are available
          // const { needsUpdate, updatedChampions } = await checkForUpdates();

          // Commented out for fix: checkForUpdates is not defined
          // if (!dataExists) {
          //   // If no data exists, we need to force an update regardless of what checkForUpdates says
          //   toast.dismiss(loadingToastId);
          //   toast.info("No champion data found. Starting download...");
          // } else if (!needsUpdate) {
          //   // Only show "up to date" if data actually exists and no updates are needed
          //   toast.dismiss(loadingToastId);
          //   toast.success("Champion data is already up to date");
          //   setIsUpdating(false);
          //   setProgress(null);
          //   return;
          // } else {
          //   // Data exists but needs update
          //   toast.dismiss(loadingToastId);
          //   toast.info("Champion data needs updating. Starting download...");
          // }

          // Update loading toast with download info
          // const downloadToastId = toast("Downloading champion data");

          // Start server action to update data with progress callback
          // const updateSuccess = await updateAllChampionsData(
          //   (updateProgress) => {
          //     setProgress(updateProgress);

          //     // Update toast with progress
          //     const progressPercent = Math.floor(updateProgress.progress);

          //     if (progressPercent % 10 === 0) {
          //       // Update toast every 10%
          //       toast.dismiss(downloadToastId);
          //       toast(
          //         `${updateProgress.processedChampions}/${
          //           updateProgress.totalChampions
          //         } (${progressPercent}%) - Current: ${
          //           updateProgress.currentChampion || "Preparing"
          //         }`
          //       );
          //     }
          //   }
          // );

          // toast.dismiss(downloadToastId);

          // if (updateSuccess) {
          //   toast.success("Data updated successfully");
          // } else if (dataExists) {
          //   toast.info("No updates were needed");
          // } else {
          //   toast.success("Champion data downloaded successfully");
          // }
        } catch (error) {
          console.error("Data update failed:", error);
          toast.error("Failed to update data");
        } finally {
          setIsUpdating(false);
          setProgress(null);
        }
      });
    } catch (error) {
      toast.dismiss(loadingToastId);
      console.error("Failed to initiate update:", error);
      toast.error("Failed to initiate data update");
      setIsUpdating(false);
      setProgress(null);
    }
  }, [isUpdating]);

  const value = {
    isUpdating: isUpdating || isPending || formPending,
    progress,
    updateData,
  };

  return (
    <DataUpdateContext.Provider value={value}>
      {children}
    </DataUpdateContext.Provider>
  );
}

export function useDataUpdater() {
  const context = useContext(DataUpdateContext);
  if (!context) {
    throw new Error("useDataUpdater must be used within a DataUpdateProvider");
  }
  return context;
}
