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
