import { Champion, ChampionSummary, Skin, Chroma } from './types';

const COMMUNITY_DRAGON_BASE_URL = 'https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default';
const FANTOME_BASE_URL = 'https://raw.githubusercontent.com/darkseal-org/lol-skins-developer/tree/main/skins';

interface ChampionDetails {
  skins: Array<{
    id: number;
    name: string;
    loadScreenPath: string;
    isBase: boolean;
    skinType: string;
    rarity: string;
    featuresText: string | null;
    chromas?: Array<{
      id: number;
      name: string;
      chromaPath: string;
      colors: string[];
      description: string;
      rarity: string;
    }>;
  }>;
}

function constructAssetUrl(path: string): string {
  // Remove leading slash if present
  const cleanPath = path.startsWith('/') ? path.slice(1) : path;
  
  // Remove the 'lol-game-data/assets/' prefix if present and convert to lowercase
  const transformedPath = cleanPath
    .replace('lol-game-data/assets/', '')
    .toLowerCase();
  
  return `${COMMUNITY_DRAGON_BASE_URL}/${transformedPath}`;
}

export async function fetchChampionSummaries(): Promise<ChampionSummary[]> {
  const response = await fetch(`${COMMUNITY_DRAGON_BASE_URL}/v1/champion-summary.json`);
  if (!response.ok) {
    throw new Error('Failed to fetch champion summaries');
  }
  const data = await response.json() as ChampionSummary[];
  return data;
}

export async function fetchChampionDetails(id: number): Promise<ChampionDetails> {
  const response = await fetch(`${COMMUNITY_DRAGON_BASE_URL}/v1/champions/${id}.json`);
  if (!response.ok) {
    throw new Error(`Failed to fetch details for champion ${id}`);
  }
  const data = await response.json() as ChampionDetails;
  return data;
}

export async function fetchFantomeFile(championId: number, skinIndex: number): Promise<string> {
  const response = await fetch(`${FANTOME_BASE_URL}/${championId}/${skinIndex}.fantome`);
  if (!response.ok) {
    throw new Error(`Failed to fetch fantome file for champion ${championId}, skin ${skinIndex}`);
  }
  return response.text();
}

export function transformChampionData(
  summary: ChampionSummary,
  details: ChampionDetails,
  fantomeFiles: Map<number, string>
): Champion {
  const skins: Skin[] = details.skins.map((skin) => {
    const chromas: Chroma[] = (skin.chromas ?? []).map((chroma) => ({
      id: chroma.id,
      name: chroma.name,
      skinChromaPath: constructAssetUrl(chroma.chromaPath),
      colors: chroma.colors,
      description: chroma.description,
      rarity: chroma.rarity
    }));

    return {
      id: skin.id,
      name: skin.name,
      skinSrc: constructAssetUrl(skin.loadScreenPath),
      isBase: skin.isBase,
      skinType: skin.skinType,
      rarity: skin.rarity || 'kNoRarity',
      featuresText: skin.featuresText ?? null,
      chromas
    };
  });

  return {
    id: summary.id,
    name: summary.name,
    alias: summary.alias,
    iconSrc: constructAssetUrl(summary.squarePortraitPath),
    skins,
    lastUpdated: Date.now()
  };
}

export function calculateProgress(
  currentChampion: string,
  totalChampions: number,
  processedChampions: number,
  status: 'checking' | 'downloading' | 'processing'
) {
  return {
    currentChampion,
    totalChampions,
    processedChampions,
    status,
    progress: (processedChampions / totalChampions) * 100
  };
} 