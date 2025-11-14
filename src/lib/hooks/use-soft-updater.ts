// Updater was removed - provide a harmless no-op hook for compatibility
import { useCallback } from "react";

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

export function useSoftUpdater() {
  // Provide a no-op updater interface so the rest of the app doesn't break
  const noOp = useCallback(() => null, []);
  return {
    status: "idle",
    updateHandle: null,
    checkForUpdates: noOp,
    downloadUpdate: noOp,
    installUpdate: noOp,
    dismissBanner: noOp,
    showBanner: noOp,
  } as const;
}
