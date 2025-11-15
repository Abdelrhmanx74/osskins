"use client";

import React from "react";
import Splash from "./Splash";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";
import { useGameStore } from "@/lib/store";
import { ToolsPhase, ToolsSource, useToolsStore } from "@/lib/store/tools";
import { useDownloadsStore } from "@/lib/store/downloads";
import { toast } from "sonner";

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

          // Mirror into unified downloads store
          const upsert = useDownloadsStore.getState().upsert;
          upsert({
            id: `tools-${source}`,
            url: "cslol-manager",
            category: "tools",
            status: phase === "error" ? "failed" : phase === "completed" ? "completed" : phase === "skipped" ? "completed" : phase === "downloading" || phase === "installing" || phase === "checking" ? "downloading" : "queued",
            downloaded: payload.downloaded ?? undefined,
            total: payload.total ?? undefined,
            speed: payload.speed ?? undefined,
            fileName: payload.version ?? null,
            error: payload.error ?? undefined,
          });
        });
        // Unified download-progress listener
        listen<any>("download-progress", (event) => {
          const p = event.payload as {
            id: string;
            status: string;
            url: string;
            category: string;
            downloaded?: number;
            total?: number;
            speed?: number;
            championName?: string | null;
            fileName?: string | null;
            destPath?: string | null;
            error?: string | null;
          };
          const upsert = useDownloadsStore.getState().upsert;
          upsert({
            id: p.id,
            url: p.url,
            category: (p.category as any) ?? "misc",
            status: (p.status as any),
            downloaded: p.downloaded,
            total: p.total,
            speed: p.speed,
            championName: (p as any).champion_name ?? (p as any).championName ?? null,
            fileName: (p as any).file_name ?? (p as any).fileName ?? null,
            destPath: (p as any).dest_path ?? (p as any).destPath ?? null,
            error: p.error ?? undefined,
          });

          // Toasts on terminal states
          if (p.status === "completed") {
            const name = (p as any).file_name ?? p.url;
            try { toast.success(`Downloaded: ${name}`); } catch { }
          } else if (p.status === "failed") {
            const name = (p as any).file_name ?? p.url;
            try { toast.error(`Download failed: ${name}`); } catch { }
          }
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
