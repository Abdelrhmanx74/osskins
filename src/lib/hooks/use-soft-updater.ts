import { useI18n } from "@/lib/i18n";
import { useAppUpdaterStore } from "@/lib/store/updater";
import { getVersion } from "@tauri-apps/api/app";
import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect } from "react";
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

export function useSoftUpdater(options?: UseSoftUpdaterOptions) {
  const autoCheck = options?.autoCheck ?? false;
  const setStatus = useAppUpdaterStore((state) => state.setStatus);
  const setUpdateHandle = useAppUpdaterStore((state) => state.setUpdateHandle);
  const setInfo = useAppUpdaterStore((state) => state.setInfo);
  const setProgress = useAppUpdaterStore((state) => state.setProgress);
  const setError = useAppUpdaterStore((state) => state.setError);
  const setBannerDismissed = useAppUpdaterStore(
    (state) => state.setBannerDismissed,
  );
  const currentVersion = useAppUpdaterStore((state) => state.currentVersion);
  const updateHandle = useAppUpdaterStore((state) => state.updateHandle);
  const status = useAppUpdaterStore((state) => state.status);
  const { t } = useI18n();
  const isDesktop = isTauriEnvironment();

  const ensureCurrentVersion = useCallback(async () => {
    if (!isDesktop || currentVersion) {
      return currentVersion;
    }
    try {
      const version = await getVersion();
      setInfo({ currentVersion: version });
      return version;
    } catch (error) {
      console.warn("Failed to read app version", error);
      return null;
    }
  }, [currentVersion, isDesktop, setInfo]);

  const checkForUpdates = useCallback(
    async ({ silent = false }: CheckOptions = {}) => {
      if (!isDesktop) {
        if (!silent) {
          toast.info(t("appUpdate.toast.desktopOnly"));
        }
        return null;
      }

      const version = await ensureCurrentVersion();
      console.log(
        `[Updater] Checking for updates. Current version: ${version}`,
      );

      if (!silent) {
        setBannerDismissed(false);
      }
      setStatus("checking");
      setError(null);
      setInfo({
        lastCheckedAt: Date.now(),
      });

      try {
        const update = await check();
        if (!update) {
          console.log("[Updater] No update available. App is up to date.");
          setUpdateHandle(null);
          setInfo({
            availableVersion: null,
            releaseNotes: null,
            pubDate: null,
          });
          setStatus("up-to-date");
          if (!silent) {
            toast.success(t("appUpdate.toast.none"));
          }
          // Reset the banner state after a short delay so the UI can settle.
          setTimeout(() => {
            useAppUpdaterStore.getState().setStatus("idle");
          }, 2500);
          return null;
        }

        console.log(
          `[Updater] Update available: ${update.version} (current: ${update.currentVersion})`,
        );
        console.log(`[Updater] Release date: ${update.date ?? "N/A"}`);
        console.log(
          `[Updater] Release notes length: ${update.body?.length ?? 0} characters`,
        );

        const previousHandle = useAppUpdaterStore.getState().updateHandle;
        if (previousHandle && previousHandle !== update) {
          void previousHandle.close().catch(() => {
            /* ignore close errors */
          });
        }

        setUpdateHandle(update);
        setInfo({
          availableVersion: update.version,
          currentVersion: update.currentVersion,
          releaseNotes: update.body ?? null,
          pubDate: update.date ?? null,
        });
        setBannerDismissed(false);
        setStatus("available");
        if (!silent) {
          toast.info(
            t("appUpdate.toast.available", { version: update.version }),
          );
        }
        return update;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error("[Updater] Failed to check for updates:", error);
        console.error("[Updater] Error details:", message);
        setStatus("error");
        setError(message);
        if (!silent) {
          toast.error(t("appUpdate.toast.error", { message }));
        }
        return null;
      }
    },
    [
      ensureCurrentVersion,
      isDesktop,
      setBannerDismissed,
      setError,
      setInfo,
      setStatus,
      setUpdateHandle,
      t,
    ],
  );

  const downloadUpdate = useCallback(async () => {
    if (!isDesktop) {
      return false;
    }
    const handle = useAppUpdaterStore.getState().updateHandle;
    if (!handle) {
      console.error("[Updater] No update handle available for download");
      return false;
    }

    console.log("[Updater] Starting download...");
    setBannerDismissed(false);
    setStatus("downloading");
    setError(null);
    setProgress(0, null);

    let downloaded = 0;
    let total = 0;

    try {
      await handle.download((event) => {
        switch (event.event) {
          case "Started": {
            total = event.data.contentLength ?? 0;
            console.log(
              `[Updater] Download started. Total size: ${total} bytes`,
            );
            setProgress(0, total || null);
            break;
          }
          case "Progress": {
            downloaded += event.data.chunkLength;
            setProgress(downloaded, total || null);
            break;
          }
          case "Finished": {
            console.log("[Updater] Download finished");
            setProgress(total || downloaded, total || null);
            break;
          }
          default:
            break;
        }
      });

      setStatus("downloaded");
      setProgress(total || downloaded, total || null);
      console.log(
        `[Updater] Download complete. Total downloaded: ${downloaded} bytes`,
      );
      toast.success(t("appUpdate.toast.downloaded"));
      return true;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error("[Updater] Failed to download update:", error);
      console.error("[Updater] Error details:", message);
      setStatus("error");
      setError(message);
      toast.error(t("appUpdate.toast.error", { message }));
      return false;
    }
  }, [isDesktop, setBannerDismissed, setError, setProgress, setStatus, t]);

  const installUpdate = useCallback(async () => {
    if (!isDesktop) {
      return false;
    }
    const handle = useAppUpdaterStore.getState().updateHandle;
    if (!handle) {
      console.error("[Updater] No update handle available for installation");
      return false;
    }

    console.log("[Updater] Starting installation...");
    setBannerDismissed(false);
    setStatus("installing");
    setError(null);
    toast.info(t("appUpdate.toast.installing"));

    try {
      await handle.install();
      console.log("[Updater] Installation successful. App will restart.");
      setStatus("installed");
      setUpdateHandle(null);
      toast.success(t("appUpdate.toast.restarting"));
      return true;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error("[Updater] Failed to install update:", error);
      console.error("[Updater] Error details:", message);
      setStatus("error");
      setError(message);
      toast.error(t("appUpdate.toast.error", { message }));
      return false;
    }
  }, [isDesktop, setBannerDismissed, setError, setStatus, setUpdateHandle, t]);

  const dismissBanner = useCallback(() => {
    setBannerDismissed(true);
  }, [setBannerDismissed]);

  const showBanner = useCallback(() => {
    setBannerDismissed(false);
  }, [setBannerDismissed]);

  useEffect(() => {
    if (!autoCheck) {
      return;
    }
    if (!isDesktop) {
      return;
    }
    void checkForUpdates({ silent: true });
  }, [autoCheck, checkForUpdates, isDesktop]);

  useEffect(() => {
    if (!isDesktop) {
      return;
    }
    if (status === "installed") {
      // Reset progress related fields but keep current version info around.
      setProgress(null, null);
    }
  }, [isDesktop, setProgress, status]);

  return {
    status,
    updateHandle,
    checkForUpdates,
    downloadUpdate,
    installUpdate,
    dismissBanner,
    showBanner,
  };
}
