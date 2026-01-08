"use client";

import React from "react";
import Splash from "./Splash";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";
import { useGameStore } from "@/lib/store";
import { useDownloadsStore } from "@/lib/store/downloads";
import { invoke } from "@tauri-apps/api/core";

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

  // CSLOL warmup removed: app uses bundled tools from resources

  React.useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    // Attach unified download-progress listener only
    import("@tauri-apps/api/event")
      .then(({ listen }) => {
        void listen<any>("download-progress", (event) => {
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
            status: p.status as any,
            downloaded: p.downloaded,
            total: p.total,
            speed: p.speed,
            championName:
              (p as any).champion_name ?? (p as any).championName ?? null,
            fileName: (p as any).file_name ?? (p as any).fileName ?? null,
            destPath: (p as any).dest_path ?? (p as any).destPath ?? null,
            error: p.error ?? undefined,
          });
        });
      })
      .catch((error: unknown) => {
        console.error("Failed to attach download-progress listener", error);
      });

    // No cleanup required for the simple listener attachment above
    return () => {
      /* no-op cleanup */
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
