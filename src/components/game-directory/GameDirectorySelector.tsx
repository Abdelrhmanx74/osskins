import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useInitialization } from "@/lib/hooks/use-initialization";

export function GameDirectorySelector() {
  const [isLoading, setIsLoading] = useState(false);
  const { isInitialized } = useInitialization();
  const leaguePath = useGameStore((s) => s.leaguePath);
  const setLeaguePath = useGameStore((s) => s.setLeaguePath);

  const handleSelectDirectory = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        toast.success("League of Legends directory selected successfully");
      }
    } catch (err) {
      console.error("Failed to select League directory:", err);
      toast.error("Failed to select directory");
    } finally {
      setIsLoading(false);
    }
  };

  const handleAutoDetect = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("auto_detect_league");
      if (path) {
        setLeaguePath(path);
        toast.success("League of Legends installation found");
      }
    } catch (err) {
      console.error("Failed to detect League directory:", err);
      toast.error(
        "Could not find League of Legends installation automatically"
      );
    } finally {
      setIsLoading(false);
    }
  };

  // Don't show anything while initializing to prevent flash
  if (!isInitialized) {
    return null;
  }

  return (
    <main className="flex min-h-full flex-col items-center justify-center p-24">
      <div className="flex flex-col items-center gap-8">
        <h1 className="text-2xl font-bold">Welcome to League Skin Manager</h1>
        <p className="text-muted-foreground">
          Please select your League of Legends installation directory to
          continue
        </p>
        <div className="flex flex-col gap-4 items-center">
          <div className="flex items-center gap-4">
            <Button
              onClick={() => void handleAutoDetect()}
              disabled={isLoading}
              variant="default"
            >
              {isLoading ? "Detecting..." : "Auto-Detect"}
            </Button>
            <Button
              onClick={() => void handleSelectDirectory()}
              disabled={isLoading}
              variant="outline"
            >
              {isLoading ? "Selecting..." : "Browse"}
            </Button>
          </div>
          {leaguePath && (
            <p className="text-sm text-muted-foreground">
              Found at: {leaguePath}
            </p>
          )}
        </div>
      </div>
    </main>
  );
}
