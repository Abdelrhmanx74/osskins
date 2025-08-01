import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "@/lib/store";

/**
 * Hook for loading initial configuration from backend
 */
export function useConfigLoader() {
  const { setSelectedMiscItems } = useGameStore();

  // Load initial config on app start
  useEffect(() => {
    const loadConfig = async () => {
      try {
        const config = await invoke("load_config");

        // Load misc items selections if they exist
        if (
          config &&
          typeof config === "object" &&
          "selected_misc_items" in config
        ) {
          const selections = config.selected_misc_items as Record<
            string,
            string[]
          >;
          setSelectedMiscItems(selections);
        }
      } catch (err) {
        console.error("Failed to load initial config:", err);
        // Not a critical error, app can continue with empty selections
      }
    };

    void loadConfig();
  }, [setSelectedMiscItems]);
}
