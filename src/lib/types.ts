export interface ChampionSummary {
  id: number;
  name: string;
  alias: string;
  squarePortraitPath: string;
}

export interface Chroma {
  id: number;
  name: string;
  skinChromaPath: string;
  colors: string[];
  description?: string;
  rarity?: string;
  fantome?: string;
}

export interface Skin {
  id: number;
  name: string;
  skinSrc: string;
  isBase: boolean;
  skinType?: string;
  rarity: string;
  featuresText: string | null;
  chromas: Chroma[];
  fantome?: string;
}

export interface Champion {
  id: number;
  name: string;
  alias: string;
  iconSrc: string;
  skins: Skin[];
  lastUpdated: number;
}

export interface DataUpdateProgress {
  currentChampion: string;
  totalChampions: number;
  processedChampions: number;
  status: "checking" | "downloading" | "processing";
  progress: number;
}

export interface DataUpdateResult {
  success: boolean;
  error?: string;
  updatedChampions?: string[];
}

// Custom skin type for user uploaded skins
export interface CustomSkin {
  id: string;
  name: string;
  champion_id: number;
  champion_name: string;
  file_path: string;
  created_at: number;
  preview_image?: string;
}
