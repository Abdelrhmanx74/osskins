import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export function useLeagueDirectory(setLeaguePath: (path: string) => void) {
  const [isLoading, setIsLoading] = useState(false);

  const handleSelectDirectory = useCallback(async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        toast.success("League of Legends directory updated successfully");
      }
    } catch (err) {
      console.error("Failed to select League directory:", err);
      toast.error("Failed to select directory");
    } finally {
      setIsLoading(false);
    }
  }, [setLeaguePath]);

  const handleAutoDetect = useCallback(async () => {
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
  }, [setLeaguePath]);

  return { isLoading, handleSelectDirectory, handleAutoDetect };
}
