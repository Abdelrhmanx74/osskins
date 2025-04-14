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

export default function Home() {
  const { isUpdating, progress, updateData } = useDataUpdate();
  const { leaguePath } = useGameStore();
  const router = useRouter();
  const [isInitialized, setIsInitialized] = useState(false);
  const [hasStartedUpdate, setHasStartedUpdate] = useState(false);

  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        if (!leaguePath || hasStartedUpdate) {
          return;
        }

        // Check if data exists and is up to date
        const dataExists = await invoke<boolean>("check_champions_data");
        const updateResult = await invoke<DataUpdateResult>(
          "check_data_updates"
        );

        if (
          dataExists &&
          (!updateResult.updatedChampions ||
            updateResult.updatedChampions.length === 0)
        ) {
          // Data is already up to date, proceed to champions page
          if (mounted) {
            setIsInitialized(true);
            router.push("/champions");
          }
          return;
        }

        // Data needs updating
        setHasStartedUpdate(true);
        await updateData();

        if (mounted) {
          setIsInitialized(true);
          router.push("/champions");
        }
      } catch (error) {
        console.error("Failed to initialize:", error);
        setHasStartedUpdate(false);
      }
    }

    void initialize();

    return () => {
      mounted = false;
    };
  }, [leaguePath, updateData, router, hasStartedUpdate]);

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
          <h1 className="text-2xl font-bold">Initializing...</h1>
          <p className="text-muted-foreground">
            Please wait while we prepare your champion data
          </p>
          <DataUpdateModal isOpen={isUpdating} progress={progress} />
        </div>
      )}
      <Toaster />
    </main>
  );
}
