import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Play, Square } from "lucide-react";
import { useEffect } from "react";
import { appDataDir } from "@tauri-apps/api/path";

export function SkinInjectionButton() {
  const {
    isInjecting,
    setInjecting,
    selectedSkins,
    leaguePath,
    setLeaguePath,
  } = useGameStore();

  // Load saved League path when component mounts
  useEffect(() => {
    async function loadLeaguePath() {
      try {
        const savedPath = await invoke<string>("load_league_path");
        if (savedPath) {
          console.log("Loaded saved League path:", savedPath);
          setLeaguePath(savedPath);
        }
      } catch (error) {
        console.error("Failed to load League path:", error);
      }
    }

    void loadLeaguePath();
  }, [setLeaguePath]);

  const handleInject = async () => {
    if (!leaguePath) {
      toast.error("Please select League of Legends directory first");
      return;
    }

    if (selectedSkins.size === 0) {
      toast.error("Please select at least one skin to inject");
      return;
    }

    try {
      setInjecting(true);

      const toastId = toast.loading("Preparing skin injection...");

      // Get app data directory for the fantome files
      const appDir = await appDataDir();

      // Format path based on platform - backslashes on Windows
      // Tauri should handle this automatically, but it's good to be safe
      const championsPath = `${appDir}/champions`;

      const skins = Array.from(selectedSkins.values()).map((skin) => ({
        champion_id: skin.championId,
        skin_id: skin.skinId,
        chroma_id: skin.chromaId,
        fantome: skin.fantome, // Add the fantome path from the skin data
      }));

      // Log what we're sending for debugging
      console.log("Injecting skins:", JSON.stringify(skins, null, 2));
      console.log("League path:", leaguePath);
      console.log("Champions path:", championsPath);

      // Use new inject_game_skins command that supports fantome paths
      const result = await invoke("inject_game_skins", {
        gamePath: leaguePath,
        skins: skins,
        fantomeFilesDir: championsPath,
      });

      console.log("Injection result:", result);

      toast.dismiss(toastId);
      toast.success("Skins injected successfully");
    } catch (error) {
      console.error("Failed to inject skins:", error);
      toast.error(`Failed to inject skins: ${error as any}`);
    } finally {
      setInjecting(false);
    }
  };

  return (
    <Button
      onClick={() => {
        void handleInject();
      }}
      disabled={isInjecting || !leaguePath || selectedSkins.size === 0}
      variant="default"
      className="flex items-center gap-2"
    >
      {isInjecting ? (
        <>
          <Square className="h-4 w-4" />
          Injecting...
        </>
      ) : (
        <>
          <Play className="h-4 w-4" />
          Inject Skins
        </>
      )}
    </Button>
  );
}
