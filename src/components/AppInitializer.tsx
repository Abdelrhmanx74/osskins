"use client";

import React from "react";
import Splash from "./Splash";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";
import { useGameStore } from "@/lib/store";

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
  const { isInitialized } = useInitialization();
  // Also trigger config loader hook which sets misc selections
  useConfigLoader();

  if (!isInitialized) {
    return <Splash />;
  }

  return <>{children}</>;
}
