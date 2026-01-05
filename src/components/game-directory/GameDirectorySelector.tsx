import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";

export function GameDirectorySelector() {
  const [isLoading, setIsLoading] = useState(false);
  const leaguePath = useGameStore((state) => state.leaguePath);
  const setLeaguePath = useGameStore((state) => state.setLeaguePath);
  const setShowUpdateModal = useGameStore((state) => state.setShowUpdateModal);
  const { t } = useI18n();

  const handleSelectDirectory = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        // Open the data update modal so user can start the download explicitly
        setShowUpdateModal(true);
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
        setShowUpdateModal(true);
      }
    } catch (err) {
      console.error("Failed to detect League directory:", err);
      toast.error(t("detect.failed"));
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-4 items-center bg-red-500">
      <div className="flex items-center gap-4">
        <Button
          onClick={() => {
            void handleAutoDetect();
            return undefined;
          }}
          disabled={isLoading}
          variant="default"
        >
          {isLoading ? t("detecting") : t("detect.button")}
        </Button>
        <Button
          onClick={() => {
            void handleSelectDirectory();
            return undefined;
          }}
          disabled={isLoading}
          variant="outline"
        >
          {isLoading ? t("selecting") : t("browse.button")}
        </Button>
      </div>
      {leaguePath && (
        <p className="text-sm text-muted-foreground">Found at: {leaguePath}</p>
      )}
    </div>
  );
}
