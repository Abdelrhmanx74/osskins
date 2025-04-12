import { Champion, ChampionSummary, Skin, Chroma } from "./types";

const COMMUNITY_DRAGON_BASE_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";
const FANTOME_BASE_URL =
  "https://raw.githubusercontent.com/darkseal-org/lol-skins-developer/main";

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
  const cleanPath = path.startsWith("/") ? path.slice(1) : path;

  // Remove the 'lol-game-data/assets/' prefix if present and convert to lowercase
  const transformedPath = cleanPath
    .replace("lol-game-data/assets/", "")
    .toLowerCase();

  return `${COMMUNITY_DRAGON_BASE_URL}/${transformedPath}`;
}

export async function fetchChampionSummaries(): Promise<ChampionSummary[]> {
  const response = await fetch(
    `${COMMUNITY_DRAGON_BASE_URL}/v1/champion-summary.json`
  );
  if (!response.ok) {
    throw new Error("Failed to fetch champion summaries");
  }
  const data = (await response.json()) as ChampionSummary[];
  return data;
}

export async function fetchChampionDetails(
  id: number
): Promise<ChampionDetails> {
  const response = await fetch(
    `${COMMUNITY_DRAGON_BASE_URL}/v1/champions/${id}.json`
  );
  if (!response.ok) {
    throw new Error(`Failed to fetch details for champion ${id}`);
  }
  const data = (await response.json()) as ChampionDetails;
  return data;
}

export async function fetchFantomeFile(
  championId: number,
  skinId: number
): Promise<Uint8Array> {
  const response = await fetch(
    `${FANTOME_BASE_URL}/${championId}/${skinId}.fantome`
  );
  if (!response.ok) {
    throw new Error(
      `Failed to fetch fantome file for champion ${championId}, skin ${skinId}`
    );
  }
  const arrayBuffer = await response.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}

export function transformChampionData(
  summary: ChampionSummary,
  details: ChampionDetails,
  fantomeFiles: Map<number, Uint8Array>
): Champion {
  const skins: Skin[] = details.skins.map((skin) => {
    const baseSkinId = skin.id % 1000;
    const chromas: Chroma[] = (skin.chromas ?? []).map((chroma) => {
      const chromaBaseSkinId = chroma.id % 1000;
      return {
        id: chroma.id,
        name: chroma.name,
        skinChromaPath: constructAssetUrl(chroma.chromaPath),
        colors: chroma.colors,
        description: chroma.description,
        rarity: chroma.rarity,
        fandom: `${summary.name.toLowerCase().replace(/\s+/g, "_")}/${skin.name
          .toLowerCase()
          .replace(/\s+/g, "_")}_chroma_${chroma.id}.fantome`,
      };
    });

    return {
      id: skin.id,
      name: skin.name,
      skinSrc: constructAssetUrl(skin.loadScreenPath),
      isBase: skin.isBase,
      skinType: skin.skinType,
      rarity: skin.rarity || "kNoRarity",
      featuresText: skin.featuresText ?? null,
      fandom: `${summary.name.toLowerCase().replace(/\s+/g, "_")}/${skin.name
        .toLowerCase()
        .replace(/\s+/g, "_")}.fantome`,
      chromas,
    };
  });

  return {
    id: summary.id,
    name: summary.name,
    alias: summary.alias,
    iconSrc: constructAssetUrl(summary.squarePortraitPath),
    skins,
    lastUpdated: Date.now(),
  };
}

export function calculateProgress(
  currentChampion: string,
  totalChampions: number,
  processedChampions: number,
  status: "checking" | "downloading" | "processing"
) {
  return {
    currentChampion,
    totalChampions,
    processedChampions,
    status,
    progress: (processedChampions / totalChampions) * 100,
  };
}
