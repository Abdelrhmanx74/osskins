import { create } from "zustand";
import type { Update } from "@tauri-apps/plugin-updater";

export type AppUpdateStatus =
    | "idle"
    | "checking"
    | "available"
    | "downloading"
    | "downloaded"
    | "installing"
    | "installed"
    | "up-to-date"
    | "error";

interface AppUpdateState {
    status: AppUpdateStatus;
    currentVersion: string | null;
    availableVersion: string | null;
    releaseNotes: string | null;
    pubDate: string | null;
    lastCheckedAt: number | null;
    updateHandle: Update | null;
    progress: number | null;
    downloadedBytes: number | null;
    totalBytes: number | null;
    error: string | null;
    bannerDismissed: boolean;
    setStatus: (status: AppUpdateStatus) => void;
    setUpdateHandle: (handle: Update | null) => void;
    setInfo: (
        info: Partial<
            Pick<
                AppUpdateState,
                "currentVersion" | "availableVersion" | "releaseNotes" | "pubDate" | "lastCheckedAt"
            >
        >,
    ) => void;
    setProgress: (downloadedBytes: number | null, totalBytes: number | null) => void;
    setError: (message: string | null) => void;
    setBannerDismissed: (dismissed: boolean) => void;
    reset: () => void;
}

const initialState: Omit<
    AppUpdateState,
    "setStatus" | "setUpdateHandle" | "setInfo" | "setProgress" | "setError" | "setBannerDismissed" | "reset"
> = {
    status: "idle",
    currentVersion: null,
    availableVersion: null,
    releaseNotes: null,
    pubDate: null,
    lastCheckedAt: null,
    updateHandle: null,
    progress: null,
    downloadedBytes: null,
    totalBytes: null,
    error: null,
    bannerDismissed: false,
};

export const useAppUpdaterStore = create<AppUpdateState>((set, _get) => ({
    ...initialState,
    setStatus: (status) => {
        set({ status });
    },
    setUpdateHandle: (handle) => {
        set({ updateHandle: handle });
    },
    setInfo: (info) => {
        set(info);
    },
    setProgress: (downloadedBytes, totalBytes) => {
        const progress = 
            downloadedBytes !== null && totalBytes !== null && totalBytes > 0
                ? Math.round((downloadedBytes / totalBytes) * 100)
                : null;
        set({ downloadedBytes, totalBytes, progress });
    },
    setError: (message) => {
        set({ error: message, status: message ? "error" : _get().status });
    },
    setBannerDismissed: (dismissed) => {
        set({ bannerDismissed: dismissed });
    },
    reset: () => {
        set({ ...initialState });
    },
}));
