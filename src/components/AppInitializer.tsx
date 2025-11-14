"use client";

import React from "react";
import Splash from "./Splash";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";
import { useGameStore } from "@/lib/store";
import { ToolsPhase, ToolsSource, useToolsStore } from "@/lib/store/tools";

export function AppInitializer({ children }: { children: React.ReactNode }) {
  // Hydrate manualInjectionMode from localStorage on client mount
  React.useEffect(() => {
    if (typeof window !== "undefined") {
      const stored = localStorage.getItem("manualInjectionMode");
      if (stored !== null) {
        useGameStore.getState().setManualInjectionMode(stored === "true");
      }
    }
  }, []);

  React.useEffect(() => {
    interface ToolsProgressPayload {
      phase?: string;
      progress?: number;
      downloaded?: number;
      total?: number;
      speed?: number;
      message?: string | null;
      version?: string | null;
      error?: string | null;
      source?: string;
    }

    if (typeof window === "undefined") {
      return;
    }

    let cancelled = false;
    let unlistenPromise: Promise<() => void> | null = null;

    import("@tauri-apps/api/event")
      .then(({ listen }) => {
        if (cancelled) {
          return;
        }

        unlistenPromise = listen<ToolsProgressPayload>("cslol-tools-progress", (event) => {
          const payload = event.payload;

          const mergeProgress = useToolsStore.getState().mergeProgress;
          const source = (payload.source === "manual" ? "manual" : "auto") as ToolsSource;
          const phase = (payload.phase ?? "idle") as ToolsPhase;
          const rawProgress = typeof payload.progress === "number" ? payload.progress : undefined;
          const clampedProgress =
            typeof rawProgress === "number"
              ? Math.min(100, Math.max(0, rawProgress))
              : undefined;

          mergeProgress(source, {
            phase,
            progress: clampedProgress,
            message: payload.message ?? undefined,
            downloaded: payload.downloaded ?? undefined,
            total: payload.total ?? undefined,
            speed: payload.speed ?? undefined,
            version: payload.version ?? undefined,
            error: payload.error ?? undefined,
          });
        });
      })
      .catch((error: unknown) => {
        console.error("Failed to attach CSLOL tools progress listener", error);
      });

    return () => {
      cancelled = true;
      if (unlistenPromise) {
        unlistenPromise
          .then((unlisten) => {
            unlisten();
          })
          .catch(() => {
            /* ignore */
          });
      }
    };
  }, []);

  const { isInitialized } = useInitialization();
  // Updater removed: no auto-check
  // Also trigger config loader hook which sets misc selections
  useConfigLoader();

  if (!isInitialized) {
    return <Splash />;
  }

  return <>{children}</>;
}
