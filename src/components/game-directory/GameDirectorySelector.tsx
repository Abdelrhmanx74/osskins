import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export function GameDirectorySelector() {
  const [isLoading, setIsLoading] = useState(false);
  const { leaguePath, setLeaguePath } = useGameStore();

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

  return (
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
        <p className="text-sm text-muted-foreground">Found at: {leaguePath}</p>
      )}
    </div>
  );
}
