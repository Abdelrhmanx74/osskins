import { invoke } from "@tauri-apps/api/core";

// Types for unified skin management
export interface SkinData {
  champion_id: number;
  skin_id: number;
  chroma_id?: number;
  fantome?: string;
}

export interface CustomSkinData {
  id: string;
  name: string;
  champion_id: number;
  champion_name: string;
  file_path: string;
  created_at: number;
  preview_image?: string;
}

export interface SavedConfig {
  league_path?: string;
  skins: SkinData[];
  custom_skins: CustomSkinData[];
  favorites: number[];
  theme?: {
    tone?: string;
    is_dark?: boolean;
  };
  party_mode: {
    paired_friends: any[];
    notifications: boolean;
  };
  selected_misc_items: Record<string, string[]>;
}

// Unified skin management API
export const skinManagementApi = {
  // Select a skin for a champion (official or custom, with mutual exclusion)
  async selectSkinForChampion(
    championId: number,
    skinData?: SkinData,
    customSkinData?: CustomSkinData
  ): Promise<void> {
    return invoke("select_skin_for_champion", {
      championId,
      skinData: skinData ?? null,
      customSkinData: customSkinData ?? null,
    });
  },

  // Remove any skin selection for a champion
  async removeSkinForChampion(championId: number): Promise<void> {
    return invoke("remove_skin_for_champion", { championId });
  },

  // Add or update a custom skin
  async saveCustomSkin(customSkin: CustomSkinData): Promise<void> {
    return invoke("save_custom_skin", { customSkin });
  },

  // Delete a custom skin
  async deleteCustomSkin(skinId: string): Promise<void> {
    return invoke("delete_custom_skin", { skinId });
  },

  // Get all custom skins
  async getAllCustomSkins(): Promise<CustomSkinData[]> {
    return invoke("get_all_custom_skins");
  },

  // Load the complete config
  async loadConfig(): Promise<SavedConfig> {
    return invoke("load_config");
  },

  // Legacy function to save selected skins (updated to work with new system)
  async saveSelectedSkins(
    leaguePath: string,
    skins: SkinData[],
    favorites: number[],
    theme?: { tone?: string; is_dark?: boolean },
    selectedMiscItems?: Record<string, string[]>
  ): Promise<void> {
    return invoke("save_selected_skins", {
      leaguePath,
      skins,
      favorites,
      theme: theme ?? null,
      selectedMiscItems: selectedMiscItems ?? null,
    });
  },
};

// Helper functions for the frontend
export const skinHelpers = {
  // Check if a champion has any skin selected (official or custom)
  hasAnySkinSelected(championId: number, config: SavedConfig): boolean {
    const hasOfficialSkin = config.skins.some(
      (s) => s.champion_id === championId
    );
    const hasCustomSkin = config.custom_skins.some(
      (s) => s.champion_id === championId
    );
    return hasOfficialSkin || hasCustomSkin;
  },

  // Get the selected skin for a champion (official or custom)
  getSelectedSkin(
    championId: number,
    config: SavedConfig
  ): { type: "official" | "custom"; data: SkinData | CustomSkinData } | null {
    const officialSkin = config.skins.find((s) => s.champion_id === championId);
    if (officialSkin) {
      return { type: "official", data: officialSkin };
    }

    const customSkin = config.custom_skins.find(
      (s) => s.champion_id === championId
    );
    if (customSkin) {
      return { type: "custom", data: customSkin };
    }

    return null;
  },

  // Get all custom skins for a specific champion
  getCustomSkinsForChampion(
    championId: number,
    customSkins: CustomSkinData[]
  ): CustomSkinData[] {
    return customSkins.filter((s) => s.champion_id === championId);
  },

  // Check if selecting a skin would conflict with existing selections
  wouldConflict(
    championId: number,
    config: SavedConfig
  ): "official" | "custom" | null {
    const officialSkin = config.skins.find((s) => s.champion_id === championId);
    if (officialSkin) return "official";

    const customSkin = config.custom_skins.find(
      (s) => s.champion_id === championId
    );
    if (customSkin) return "custom";

    return null;
  },
};

// Example usage in React components:
/*
// To select an official skin for a champion:
await skinManagementApi.selectSkinForChampion(
  championId,
  { champion_id: championId, skin_id: skinId, chroma_id: chromaId },
  undefined
);

// To select a custom skin for a champion:
await skinManagementApi.selectSkinForChampion(
  championId,
  undefined,
  { id: customSkinId, name: "Custom Skin", champion_id: championId, champion_name: "Champion Name", file_path: "/path/to/skin", created_at: Date.now() }
);

// To remove any skin selection:
await skinManagementApi.removeSkinForChampion(championId);

// To check what's currently selected:
const config = await skinManagementApi.loadConfig();
const selectedSkin = skinHelpers.getSelectedSkin(championId, config);
if (selectedSkin) {
  console.log(`Champion ${championId} has ${selectedSkin.type} skin selected:`, selectedSkin.data);
}
*/
