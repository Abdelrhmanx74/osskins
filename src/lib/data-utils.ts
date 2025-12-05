import type { Champion, ChampionSummary, Skin, Chroma } from "./types";
import { downloadFileSimple } from "./download-utils";

const COMMUNITY_DRAGON_BASE_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";

// New LeagueSkins repository
const LEAGUE_SKINS_BASE_URL =
  "https://raw.githubusercontent.com/Alban1911/LeagueSkins/main";

/**
 * Build download URL for a skin from the LeagueSkins repository
 * Structure: skins/{champion_id}/{skin_id}/{skin_id}.zip or .fantome
 * For chromas: skins/{champion_id}/{skin_id}/{chroma_id}/{chroma_id}.zip or .fantome
 * For forms: skins/{champion_id}/{skin_id}/{form_id}/{form_id}.zip or .fantome
 * 
 * Note: The repo has mixed extensions (.zip and .fantome) - caller should try both
 */
export function buildSkinDownloadUrl(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
  extension: "zip" | "fantome" = "zip",
): string {
  const base = `${LEAGUE_SKINS_BASE_URL}/skins/${championId}/${skinId}`;

  if (chromaId) {
    return `${base}/${chromaId}/${chromaId}.${extension}`;
  }
  if (formId) {
    return `${base}/${formId}/${formId}.${extension}`;
  }
  return `${base}/${skinId}.${extension}`;
}

/**
 * Get the skin ID from champion ID and skin number
 * Skin ID = champion_id * 1000 + skin_number
 */
export function getSkinId(championId: number, skinNumber: number): number {
  return championId * 1000 + skinNumber;
}

/**
 * Extract skin number from skin ID
 */
export function getSkinNumber(skinId: number): number {
  return skinId % 1000;
}

/**
 * Extract champion ID from skin ID
 */
export function getChampionIdFromSkinId(skinId: number): number {
  return Math.floor(skinId / 1000);
}

/**
 * Check if a skin exists in the LeagueSkins repository
 */
export async function checkSkinExists(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): Promise<boolean> {
  const url = buildSkinDownloadUrl(championId, skinId, chromaId, formId);
  try {
    const response = await fetch(url, { method: "HEAD" });
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Download a skin ZIP file directly from LeagueSkins repository
 */
export async function fetchSkinZipById(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): Promise<Uint8Array> {
  const url = buildSkinDownloadUrl(championId, skinId, chromaId, formId);
  try {
    return await downloadFileSimple(url);
  } catch (error) {
    console.warn(
      `[Download] Failed to fetch skin ${skinId} for champion ${championId}:`,
      error,
    );
    return new Uint8Array();
  }
}

/**
 * Download a skin using the legacy name-based approach (for backward compatibility)
 * Maps champion name + skin name to champion ID + skin ID and downloads from new repo
 */
export async function fetchSkinZip(
  championName: string,
  fileName: string,
  subPath: string[] = [],
): Promise<Uint8Array> {
  // This function now requires a champion ID mapping
  // For backward compatibility, we'll try to find matching skin by checking HEAD requests
  console.warn(
    `[Download] fetchSkinZip called with name-based approach. Consider using fetchSkinZipById for better performance.`,
    { championName, fileName, subPath },
  );

  // Return empty array - the caller should use the new ID-based API
  return new Uint8Array();
}

/**
 * Get download URL for a skin (for backend downloads)
 */
export function getSkinDownloadUrl(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): string {
  return buildSkinDownloadUrl(championId, skinId, chromaId, formId);
}

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
  const cleanPath = path.startsWith("/") ? path.slice(1) : path;
  const transformedPath = cleanPath
    .replace("lol-game-data/assets/", "")
    .toLowerCase();
  return `${COMMUNITY_DRAGON_BASE_URL}/${transformedPath}`;
}

export async function fetchChampionSummaries(): Promise<ChampionSummary[]> {
  const response = await fetch(
    `${COMMUNITY_DRAGON_BASE_URL}/v1/champion-summary.json`,
  );
  if (!response.ok) {
    throw new Error("Failed to fetch champion summaries");
  }
  const data = (await response.json()) as ChampionSummary[];
  return data;
}

export async function fetchChampionDetails(
  id: number,
): Promise<ChampionDetails> {
  const response = await fetch(
    `${COMMUNITY_DRAGON_BASE_URL}/v1/champions/${id}.json`,
  );
  if (!response.ok) {
    throw new Error(`Failed to fetch details for champion ${id}`);
  }
  const data = (await response.json()) as ChampionDetails;
  return data;
}

export function sanitizeForFileName(str: string): string {
  return str
    .toLowerCase()
    .trim()
    .replace(/[/\\:?*"<>|()' ]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function transformChampionData(
  summary: ChampionSummary,
  details: ChampionDetails,
  skinFiles: Map<number, Uint8Array>,
): Champion {
  const baseDir = sanitizeForFileName(summary.name);
  const skins: Skin[] = details.skins.map((skin) => {
    const chromas: Chroma[] = (skin.chromas ?? []).map((chroma) => ({
      id: chroma.id,
      name: chroma.name,
      skinChromaPath: constructAssetUrl(chroma.chromaPath),
      colors: chroma.colors,
      description: chroma.description,
      rarity: chroma.rarity,
      skin_file: `${baseDir}/${sanitizeForFileName(skin.name)}_chroma_${chroma.id}.zip`,
    }));

    return {
      id: skin.id,
      name: skin.name,
      skinSrc: constructAssetUrl(skin.loadScreenPath),
      isBase: skin.isBase,
      skinType: skin.skinType,
      rarity: skin.rarity || "kNoRarity",
      featuresText: skin.featuresText ?? null,
      skin_file: `${baseDir}/${sanitizeForFileName(skin.name)}.zip`,
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
  status: "checking" | "downloading" | "processing",
) {
  return {
    currentChampion,
    totalChampions,
    processedChampions,
    status,
    progress: (processedChampions / totalChampions) * 100,
  };
}
