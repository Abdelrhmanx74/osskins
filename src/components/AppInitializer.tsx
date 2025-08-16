"use client";

import React from "react";
import Splash from "./Splash";
import { useInitialization } from "@/lib/hooks/use-initialization";
import { useConfigLoader } from "@/lib/hooks/use-config-loader";

export function AppInitializer({ children }: { children: React.ReactNode }) {
  const { isInitialized } = useInitialization();
  // Also trigger config loader hook which sets misc selections
  useConfigLoader();

  if (!isInitialized) {
    return <Splash />;
  }

  return <>{children}</>;
}
