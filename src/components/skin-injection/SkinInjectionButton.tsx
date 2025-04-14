import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Play, Square } from "lucide-react";
import { useEffect } from "react";
import { appDataDir } from "@tauri-apps/api/path";

export function SkinInjectionButton() {
  const { isInjecting, setInjecting, selectedSkins, leaguePath } =
    useGameStore();

  // Ensure mod-tools are in place when component mounts
  useEffect(() => {
    async function ensureModTools() {
      try {
        await invoke("ensure_mod_tools");
      } catch (error) {
        console.error("Failed to ensure mod-tools:", error);
      }
    }

    ensureModTools();
  }, []);

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

      const skins = Array.from(selectedSkins.values()).map((skin) => ({
        champion_id: skin.championId,
        skin_id: skin.skinId,
        chroma_id: skin.chromaId,
        fantome: skin.fantome, // Add the fantome path from the skin data
      }));

      // Log what we're sending for debugging
      console.log("Injecting skins:", JSON.stringify(skins, null, 2));

      // Use new inject_game_skins command that supports fantome paths
      await invoke("inject_game_skins", {
        game_path: leaguePath,
        skins: skins,
        fantome_files_dir: `${appDir}/champions`,
      });

      toast.dismiss(toastId);
      toast.success("Skins injected successfully");
    } catch (error) {
      console.error("Failed to inject skins:", error);
      toast.error(`Failed to inject skins: ${error}`);
    } finally {
      setInjecting(false);
    }
  };

  return (
    <Button
      onClick={handleInject}
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
