"use client";

import { useEffect, useState } from "react";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { DataUpdateModal } from "@/components/DataUpdateModal";
import { Toaster } from "sonner";
import { useRouter } from "next/navigation";
import { GameDirectorySelector } from "@/components/game-directory/GameDirectorySelector";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { DataUpdateResult } from "@/lib/types";
import { Button } from "@/components/ui/button";

export default function Home() {
  const { isUpdating, progress, updateData } = useDataUpdate();
  const { leaguePath, setLeaguePath } = useGameStore();
  const router = useRouter();
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);

  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        // Load saved league path first
        const savedPath = await invoke<string>("load_league_path");
        if (savedPath && mounted) {
          console.log("Loaded saved League path:", savedPath);
          setLeaguePath(savedPath);
        }

        // Only check for updates if we haven't already started
        if (!hasStartedUpdate && mounted) {
          const needsUpdate = !(await invoke<boolean>("check_champions_data"));
          console.log("Needs update:", needsUpdate);

          if (needsUpdate) {
            console.log("Starting data update...");
            setHasStartedUpdate(true);
            await updateData();
          }
        }

        if (mounted) {
          setIsInitialized(true);
        }
      } catch (error) {
        console.error("Failed to initialize:", error);
        if (mounted) {
          setIsInitialized(true); // Still mark as initialized so UI isn't stuck
        }
      }
    }

    // Only initialize if not already done
    if (!isInitialized) {
      void initialize();
    }

    return () => {
      mounted = false;
    };
  }, [isInitialized, updateData, setLeaguePath, hasStartedUpdate]);

  // If updating, show the modal regardless of league path
  if (isUpdating) {
    return (
      <main className="flex min-h-screen flex-col items-center justify-center p-24">
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Initializing...</h1>
          <p className="text-muted-foreground">
            Please wait while we prepare your champion data
          </p>
          <DataUpdateModal isOpen={true} progress={progress} />
        </div>
        <Toaster />
      </main>
    );
  }

  // Otherwise show normal UI
  return (
    <main className="flex min-h-screen flex-col items-center justify-center p-24">
      {!leaguePath ? (
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Welcome to League Skin Manager</h1>
          <p className="text-muted-foreground">
            Please select your League of Legends installation directory to
            continue
          </p>
          <GameDirectorySelector />
        </div>
      ) : (
        <div className="flex flex-col items-center gap-8">
          <h1 className="text-2xl font-bold">Ready!</h1>
          <p className="text-muted-foreground">
            Click the button below to proceed to champion selection
          </p>
          <Button
            onClick={() => {
              router.push("/champions");
            }}
          >
            View Champions
          </Button>
        </div>
      )}
      <Toaster />
    </main>
  );
}
