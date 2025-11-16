import { useCallback, useEffect } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useAppUpdaterStore } from "@/lib/store/updater";
import { toast } from "sonner";

interface CheckOptions {
  silent?: boolean;
}

interface UseSoftUpdaterOptions {
  autoCheck?: boolean;
}

function isTauriEnvironment(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const candidate = window as unknown as Record<string, unknown>;
  return Boolean(candidate.__TAURI__);
}

export function useSoftUpdater(options: UseSoftUpdaterOptions = {}) {
  const { autoCheck = false } = options;
  
  const {
    status,
    updateHandle,
    setStatus,
    setUpdateHandle,
    setInfo,
    setProgress,
    setError,
    setBannerDismissed,
  } = useAppUpdaterStore();

  // Check for updates
  const checkForUpdates = useCallback(
    async (checkOptions: CheckOptions = {}) => {
      const { silent = false } = checkOptions;

      if (!isTauriEnvironment()) {
        console.log("[Updater] Not in Tauri environment, skipping update check");
        return;
      }

      try {
        console.log("[Updater] Checking for updates...");
        setStatus("checking");
        setError(null);

        const update = await check();

        if (update) {
          console.log(`[Updater] Update available: ${update.version}`);
          console.log(`[Updater] Current version: ${update.currentVersion}`);
          console.log(`[Updater] Release date: ${update.date}`);
          console.log(`[Updater] Release notes: ${update.body || "No release notes"}`);

          setStatus("available");
          setUpdateHandle(update);
          setInfo({
            currentVersion: update.currentVersion,
            availableVersion: update.version,
            releaseNotes: update.body || null,
            pubDate: update.date || null,
            lastCheckedAt: Date.now(),
          });
          setBannerDismissed(false);

          if (!silent) {
            toast.success(`Update available: v${update.version}`);
          }
        } else {
          console.log("[Updater] No updates available");
          setStatus("up-to-date");
          setInfo({
            lastCheckedAt: Date.now(),
          });

          if (!silent) {
            toast.info("You're up to date!");
          }
        }
      } catch (error) {
        console.error("[Updater] Error checking for updates:", error);
        const errorMessage = error instanceof Error ? error.message : "Failed to check for updates";
        setError(errorMessage);
        setStatus("error");

        if (!silent) {
          toast.error(errorMessage);
        }
      }
    },
    [setStatus, setUpdateHandle, setInfo, setError, setBannerDismissed]
  );

  // Download update
  const downloadUpdate = useCallback(async () => {
    if (!updateHandle) {
      console.error("[Updater] No update handle available");
      toast.error("No update available to download");
      return;
    }

    try {
      console.log("[Updater] Starting download...");
      setStatus("downloading");
      setProgress(0, 100);
      setError(null);

      await updateHandle.downloadAndInstall((progress) => {
        console.log(`[Updater] Download progress: ${progress.downloaded}/${progress.total} bytes`);
        setProgress(progress.downloaded, progress.total);
      });

      console.log("[Updater] Download and installation complete");
      setStatus("downloaded");
      toast.success("Update downloaded successfully!");
    } catch (error) {
      console.error("[Updater] Error downloading update:", error);
      const errorMessage = error instanceof Error ? error.message : "Failed to download update";
      setError(errorMessage);
      setStatus("error");
      toast.error(errorMessage);
    }
  }, [updateHandle, setStatus, setProgress, setError]);

  // Install and relaunch
  const installUpdate = useCallback(async () => {
    try {
      console.log("[Updater] Installing update and relaunching...");
      setStatus("installing");
      toast.info("Restarting application...");
      
      // Relaunch the application
      await relaunch();
    } catch (error) {
      console.error("[Updater] Error installing update:", error);
      const errorMessage = error instanceof Error ? error.message : "Failed to install update";
      setError(errorMessage);
      setStatus("error");
      toast.error(errorMessage);
    }
  }, [setStatus, setError]);

  // Dismiss banner
  const dismissBanner = useCallback(() => {
    setBannerDismissed(true);
  }, [setBannerDismissed]);

  // Show banner
  const showBanner = useCallback(() => {
    setBannerDismissed(false);
  }, [setBannerDismissed]);

  // Auto-check on mount
  useEffect(() => {
    if (autoCheck && isTauriEnvironment()) {
      console.log("[Updater] Auto-checking for updates on mount");
      void checkForUpdates({ silent: true });
    }
  }, [autoCheck, checkForUpdates]);

  return {
    status,
    updateHandle,
    checkForUpdates,
    downloadUpdate,
    installUpdate,
    dismissBanner,
    showBanner,
  } as const;
}
