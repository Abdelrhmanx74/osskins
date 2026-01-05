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
  skin_file?: string;
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
  skin_file?: string;
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
  currentSkin?: string;
  totalChampions: number;
  processedChampions: number;
  status: "checking" | "downloading" | "processing";
  progress: number;
  /** Dynamic unit label (e.g., "champions", "files", "items") */
  unit?: string;
}

export interface DataUpdateResult {
  success: boolean;
  error?: string;
  updatedChampions?: string[];
}

/** Changed skin file from incremental update comparison */
export interface ChangedSkinFile {
  championId: number;
  skinId: number;
  chromaId?: number;
  filename: string;
  status: "added" | "modified";
  downloadUrl: string;
}

export interface EnsureModToolsResult {
  installed: boolean;
  updated: boolean;
  skipped: boolean;
  version?: string;
  latestVersion?: string;
  path?: string;
}

export interface CslolManagerStatus {
  installed: boolean;
  version?: string;
  latestVersion?: string;
  hasUpdate: boolean;
  path?: string;
  downloadSize?: number;
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
