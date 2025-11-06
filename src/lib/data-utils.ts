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

export async function fetchFantomeFile(
  championId: number,
  skinId: number,
): Promise<Uint8Array> {
  try {
    const response = await fetch(
      `${FANTOME_BASE_URL}/${championId}/${skinId}.skin_file`,
    );
    if (!response.ok) {
      console.warn(`Fantome not found: champion ${championId}, skin ${skinId}`);
      return new Uint8Array();
    }
    const arrayBuffer = await response.arrayBuffer();
    return new Uint8Array(arrayBuffer);
  } catch (err) {
    console.warn(
      `Error fetching skin_file for champion ${championId}, skin ${skinId}:`,
      err,
    );
    return new Uint8Array();
  }
}

// Download skin ZIP from darkseal-org/lol-skins repository
export async function fetchSkinZip(
  championName: string,
  subPath: string[] = [],
  fileName: string,
): Promise<Uint8Array> {
  // Special handling for K/DA skins - replace KDA with K DA for repo URLs
  const normalizedChampion = championName.replace(/KDA/g, "K DA");

  // Handle folder paths with special cases
  const normalizedSubPath = subPath.map((segment) =>
    segment.replace(/KDA/g, "K DA").replace(/\bM\.D(?!\.)(?=\s|$)/g, "M.D."),
  );

  // Handle both KDA and M.D. naming conventions in file names
  let normalizedFileName = fileName.replace(/KDA/g, "K DA");

  // Add period after M.D if missing, but don't double up periods
  normalizedFileName = normalizedFileName.replace(
    /\bM\.D(?!\.)(?=\s|$)/g,
    "M.D.",
  );

  // Construct URL with proper encoding for each path segment
  const encodedSegments = [
    "skins",
    normalizedChampion,
    ...normalizedSubPath,
    `${normalizedFileName}.zip`,
  ]
    .map((segment) => encodeURIComponent(segment))
    .join("/");

  const blobUrl = `https://github.com/darkseal-org/lol-skins/blob/main/${encodedSegments}`;

  // Convert to raw URL
  const rawUrl = blobUrl
    .replace("github.com", "raw.githubusercontent.com")
    .replace("/blob/", "/");

  try {
    // First, try the direct URL
    const response = await fetch(rawUrl, { method: "HEAD" });

    if (response.ok) {
      // If successful, get the actual content
      const contentResponse = await fetch(rawUrl);
      if (contentResponse.ok) {
        const buffer = await contentResponse.arrayBuffer();
        return new Uint8Array(buffer);
      }
    }

    // If it's a chroma and the first attempt failed, try alternative path structures
    if (subPath.includes("chromas") && !response.ok) {
      // Repository may have a different structure for some chromas
      // Try a direct structure without nesting: champion/chromas/skinName chromaId.zip
      const flatStructureSegments = [
        "skins",
        normalizedChampion,
        "chromas",
        `${normalizedFileName}.zip`,
      ]
        .map((segment) => encodeURIComponent(segment))
        .join("/");

      const flatBlobUrl = `https://github.com/darkseal-org/lol-skins/blob/main/${flatStructureSegments}`;
      const flatRawUrl = flatBlobUrl
        .replace("github.com", "raw.githubusercontent.com")
        .replace("/blob/", "/");

      const flatResponse = await fetch(flatRawUrl);
      if (flatResponse.ok) {
        const buffer = await flatResponse.arrayBuffer();
        return new Uint8Array(buffer);
      }
    }
  } catch {
    // ignore fetch errors
  }

  // Silently return empty buffer (will fall back to skin_file)
  return new Uint8Array();
}

// Sanitize a string to be safe for use as a filename or path component
export function sanitizeForFileName(str: string): string {
  return str
    .toLowerCase()
    .trim()
    .replace(/[/\\:?*"<>|()' ]+/g, "_") // replace invalid Windows path chars, spaces, apostrophes, parentheses
    .replace(/_+/g, "_") // collapse multiple underscores
    .replace(/^_+|_+$/g, ""); // trim leading/trailing underscores
}

export function transformChampionData(
  summary: ChampionSummary,
  details: ChampionDetails,
  skinFiles: Map<number, Uint8Array>,
): Champion {
  const baseDir = sanitizeForFileName(summary.name);
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
        skin_file: `${baseDir}/${sanitizeForFileName(skin.name)}_chroma_${
          chroma.id
        }.zip`,
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
