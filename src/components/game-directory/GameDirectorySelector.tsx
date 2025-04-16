import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
        toast.success("League directory selected successfully");
      }
    } catch (error) {
      toast.error("Failed to select League directory");
      console.error("Failed to select League directory:", error);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="flex items-center gap-4">
        <Input
          type="text"
          value={leaguePath ?? ""}
          placeholder="League of Legends directory"
          readOnly
          className="flex-1"
        />
        <Button
          onClick={() => {
            void handleSelectDirectory();
          }}
          disabled={isLoading}
          variant="outline"
        >
          {isLoading ? "Selecting..." : "Select Directory"}
        </Button>
      </div>
      {leaguePath && (
        <div className="text-sm text-muted-foreground">
          Selected directory: {leaguePath}
        </div>
      )}
    </div>
  );
}
