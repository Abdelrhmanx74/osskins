import type { Champion, ChampionSummary, Skin, Chroma } from "./types";

const COMMUNITY_DRAGON_BASE_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default";
const FANTOME_BASE_URL =
  "https://raw.githubusercontent.com/darkseal-org/lol-skins-developer/main";
const LOL_SKINS_MANIFEST_URL =
  "https://abdelrhmanx74.github.io/osskins/manifest.json";

export interface LolSkinsManifestItem {
  path: string;
  url: string;
  size: number;
  sha256: string;
  commit: string;
}

export interface LolSkinsManifest {
  schema: number;
  generated_at: string;
  source_repo: string;
  branch: string;
  license: string;
  source: string;
  attribution: string;
  items: LolSkinsManifestItem[];
}

let manifestCache: LolSkinsManifest | null = null;
let manifestFetchPromise: Promise<LolSkinsManifest | null> | null = null;
let manifestGeneration = 0;

export function resetLolSkinsManifestCache(): void {
  manifestGeneration += 1;
  manifestCache = null;
  manifestFetchPromise = null;
}

function stripZipExtension(name: string): string {
  return name.toLowerCase().endsWith(".zip") ? name.slice(0, -4) : name;
}

function normalizeSegment(segment: string): string {
  // Normalize apostrophes and similar characters for fuzzy matching
  return segment
    .toLowerCase()
    .replace(/['’`]/g, "") // Remove all apostrophe-like chars
    .replace(/[^a-z0-9]/g, "");
}

function tokenize(value: string): string[] {
  const cleaned = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
  if (!cleaned) {
    return [];
  }
  return cleaned.split(/\s+/);
}

function tokenSignature(tokens: string[]): string {
  if (tokens.length === 0) {
    return "";
  }
  return [...tokens].sort().join("|");
}

function computeTokenOverlap(target: string[], candidate: string[]): number {
  if (target.length === 0 || candidate.length === 0) {
    return 0;
  }
  const targetCounts = new Map<string, number>();
  for (const token of target) {
    targetCounts.set(token, (targetCounts.get(token) ?? 0) + 1);
  }
  let matches = 0;
  for (const token of candidate) {
    const available = targetCounts.get(token);
    if (available && available > 0) {
      matches += 1;
      targetCounts.set(token, available - 1);
    }
  }
  return matches / Math.max(target.length, candidate.length);
}

function computeSubPathScore(candidate: string[], target: string[]): number {
  if (
    candidate.length === target.length &&
    candidate.every((seg, idx) => seg === target[idx])
  ) {
    return 2;
  }

  if (target.length === 0) {
    return candidate.length === 0 ? 2 : 0;
  }

  if (candidate.length === 0) {
    return 0;
  }

  if (candidate[0] === "chromas" && target[0] === "chromas") {
    const intersection = candidate.filter((seg) => target.includes(seg)).length;
    return 1 + intersection / Math.max(candidate.length, target.length);
  }

  return 0;
}

async function ensureLolSkinsManifest(): Promise<LolSkinsManifest | null> {
  if (manifestCache) {
    return manifestCache;
  }

  if (!manifestFetchPromise) {
    const requestGeneration = manifestGeneration;
    manifestFetchPromise = (async () => {
      try {
        const response = await fetch(LOL_SKINS_MANIFEST_URL, {
          cache: "no-store",
        });
        if (!response.ok) {
          throw new Error(`Manifest request failed: ${response.status}`);
        }
        const data = (await response.json()) as LolSkinsManifest;
        if (manifestGeneration === requestGeneration) {
          manifestCache = data;
        }
        return data;
      } catch (error) {
        console.error(
          `[Manifest] Failed to fetch lol-skins manifest from ${LOL_SKINS_MANIFEST_URL}`,
          error,
        );
        return null;
      } finally {
        if (manifestGeneration === requestGeneration) {
          manifestFetchPromise = null;
        }
      }
    })();
  }

  const fetchPromise = manifestFetchPromise;
  const data = await fetchPromise;
  return data;
}

export async function getLolSkinsManifest(): Promise<LolSkinsManifest | null> {
  return ensureLolSkinsManifest();
}

export function getLolSkinsManifestCommit(
  manifest: LolSkinsManifest,
): string | null {
  const repoParts = manifest.source_repo.split("@");
  if (repoParts.length === 2 && repoParts[1]) {
    return repoParts[1];
  }

  if (manifest.items.length > 0 && manifest.items[0]?.commit) {
    return manifest.items[0].commit;
  }

  return null;
}

function findManifestEntry(
  manifest: LolSkinsManifest,
  championName: string,
  subPath: string[],
  fileName: string,
): LolSkinsManifestItem | null {
  // Normalize for fuzzy matching: remove apostrophes, spaces, lowercase
  const targetChampion = normalizeSegment(championName);
  const targetSubPath = subPath.map(normalizeSegment);
  // Try both normalized and raw for more robust matching
  const targetBaseName = normalizeSegment(fileName);
  const targetBaseNameNoSpace = fileName
    .replace(/['’`]/g, "")
    .replace(/ /g, "")
    .toLowerCase();
  const targetTokens = tokenize(fileName);
  const targetSignature = tokenSignature(targetTokens);

  let bestCandidate: { item: LolSkinsManifestItem; score: number } | null =
    null;

  for (const item of manifest.items) {
    const segments = item.path.split("/");
    if (segments.length < 3) continue;
    if (normalizeSegment(segments[0]) !== "skins") continue;
    if (normalizeSegment(segments[1]) !== targetChampion) continue;

    const middleSegments = segments.slice(2, -1).map(normalizeSegment);
    const subPathScore = computeSubPathScore(middleSegments, targetSubPath);
    if (subPathScore === 0) continue;

    const baseName = stripZipExtension(segments[segments.length - 1]);
    const candidateBase = normalizeSegment(baseName);
    const candidateBaseNoSpace = baseName
      .replace(/['’`]/g, "")
      .replace(/ /g, "")
      .toLowerCase();

    // Direct normalized match
    if (
      candidateBase === targetBaseName ||
      candidateBaseNoSpace === targetBaseNameNoSpace
    ) {
      return item;
    }

    // Fuzzy: try token signature
    const candidateTokens = tokenize(baseName);
    if (tokenSignature(candidateTokens) === targetSignature) {
      return item;
    }

    // Fuzzy: try substring match (for e.g. BeeMawKogMaw vs Bee'Maw)
    if (
      candidateBaseNoSpace.includes(targetBaseNameNoSpace) ||
      targetBaseNameNoSpace.includes(candidateBaseNoSpace)
    ) {
      return item;
    }

    const overlap = computeTokenOverlap(targetTokens, candidateTokens);
    if (overlap === 0) continue;

    const score = subPathScore + overlap;
    if (!bestCandidate || score > bestCandidate.score) {
      bestCandidate = { item, score };
    }
  }

  return bestCandidate?.item ?? null;
}

async function fetchSkinZipViaManifest(
  championName: string,
  subPath: string[],
  fileName: string,
): Promise<Uint8Array | null> {
  try {
    const manifest = await ensureLolSkinsManifest();
    if (!manifest) {
      return null;
    }

    const entry = findManifestEntry(manifest, championName, subPath, fileName);
    if (!entry) {
      return null;
    }

    const response = await fetch(entry.url);
    if (!response.ok) {
      console.warn(
        `[Manifest] Download failed for ${entry.url} (status ${response.status})`,
      );
      return null;
    }

    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
  } catch (error) {
    console.warn(
      `[Manifest] Error fetching skin via manifest for ${championName} / ${fileName}`,
      error,
    );
    return null;
  }
}

async function fetchSkinZipLegacy(
  championName: string,
  fileName: string,
  subPath: string[] = [],
): Promise<Uint8Array> {
  const normalizedChampion = championName.replace(/KDA/g, "K DA");

  const normalizedSubPath = subPath.map((segment) =>
    segment.replace(/KDA/g, "K DA").replace(/\bM\.D(?!\.)(?=\s|$)/g, "M.D."),
  );

  let normalizedFileName = fileName.replace(/KDA/g, "K DA");

  normalizedFileName = normalizedFileName.replace(
    /\bM\.D(?!\.)(?=\s|$)/g,
    "M.D.",
  );

  const encodedSegments = [
    "skins",
    normalizedChampion,
    ...normalizedSubPath,
    `${normalizedFileName}.zip`,
  ]
    .map((segment) => encodeURIComponent(segment))
    .join("/");

  const blobUrl = `https://github.com/darkseal-org/lol-skins/blob/main/${encodedSegments}`;

  const rawUrl = blobUrl
    .replace("github.com", "raw.githubusercontent.com")
    .replace("/blob/", "/");

  try {
    const response = await fetch(rawUrl, { method: "HEAD" });

    if (response.ok) {
      const contentResponse = await fetch(rawUrl);
      if (contentResponse.ok) {
        const buffer = await contentResponse.arrayBuffer();
        return new Uint8Array(buffer);
      }
    }

    if (subPath.includes("chromas") && !response.ok) {
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

  return new Uint8Array();
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
  fileName: string,
  subPath: string[] = [],
): Promise<Uint8Array> {
  const manifestContent = await fetchSkinZipViaManifest(
    championName,
    subPath,
    fileName,
  );
  if (manifestContent) {
    return manifestContent;
  }

  const legacyContent = await fetchSkinZipLegacy(
    championName,
    fileName,
    subPath,
  );
  if (legacyContent.byteLength > 0) {
    return legacyContent;
  }

  console.warn(
    `[Manifest] Unable to locate ${championName} / ${fileName}.zip in manifest or repository`,
  );
  return legacyContent;
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
        skin_file: `${baseDir}/${sanitizeForFileName(skin.name)}_chroma_${chroma.id
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
