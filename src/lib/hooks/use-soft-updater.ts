import { useCallback, useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { toast } from "sonner";
import { useAppUpdaterStore } from "@/lib/store/updater";
import { useI18n } from "@/lib/i18n";

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
    const setBannerDismissed = useAppUpdaterStore((state) => state.setBannerDismissed);
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

            await ensureCurrentVersion();

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
                console.error("Failed to check for updates", error);
                setStatus("error");
                setError(message);
                if (!silent) {
                    toast.error(t("appUpdate.toast.error", { message }));
                }
                return null;
            }
        },
        [
            currentVersion,
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
            return false;
        }

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
                        setProgress(0, total || null);
                        break;
                    }
                    case "Progress": {
                        downloaded += event.data.chunkLength;
                        setProgress(downloaded, total || null);
                        break;
                    }
                    case "Finished": {
                        setProgress(total || downloaded, total || null);
                        break;
                    }
                    default:
                        break;
                }
            });

            setStatus("downloaded");
            setProgress(total || downloaded, total || null);
            toast.success(t("appUpdate.toast.downloaded"));
            return true;
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            console.error("Failed to download update", error);
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
            return false;
        }

        setBannerDismissed(false);
        setStatus("installing");
        setError(null);
        toast.info(t("appUpdate.toast.installing"));

        try {
            await handle.install();
            setStatus("installed");
            setUpdateHandle(null);
            toast.success(t("appUpdate.toast.restarting"));
            return true;
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            console.error("Failed to install update", error);
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
