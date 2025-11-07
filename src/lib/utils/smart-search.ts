import Fuse, { FuseResultMatch } from "fuse.js";
import { Champion, Skin } from "@/lib/types";

export interface SearchableItem {
  type: "champion" | "skin";
  championId: number;
  championName: string;
  championAlias: string;
  skinId?: number;
  skinName?: string;
  rarity?: string;
  skinType?: string;
  featuresText?: string | null;
  // Chroma data - colors and descriptions
  chromaColors?: string; // Combined string of all chroma colors
  chromaDescriptions?: string; // Combined string of all chroma descriptions
  // For relevance scoring
  isBase?: boolean;
}

export interface SearchResult {
  item: SearchableItem;
  score: number;
  matches?: readonly FuseResultMatch[];
}

/**
 * Create a searchable index from champions and their skins
 */
export function createSearchIndex(champions: Champion[]): SearchableItem[] {
  const items: SearchableItem[] = [];

  for (const champion of champions) {
    // Add champion itself
    items.push({
      type: "champion",
      championId: champion.id,
      championName: champion.name,
      championAlias: champion.alias,
    });

    // Add all skins for this champion
    for (const skin of champion.skins) {
      // Collect all chroma colors and descriptions
      const allChromaColors: string[] = [];
      const allChromaDescriptions: string[] = [];
      
      for (const chroma of skin.chromas) {
        // Add individual colors
        allChromaColors.push(...chroma.colors);
        // Add chroma name (often contains color info)
        if (chroma.name) {
          allChromaColors.push(chroma.name);
        }
        // Add chroma description
        if (chroma.description) {
          allChromaDescriptions.push(chroma.description);
        }
      }

      items.push({
        type: "skin",
        championId: champion.id,
        championName: champion.name,
        championAlias: champion.alias,
        skinId: skin.id,
        skinName: skin.name,
        rarity: skin.rarity,
        skinType: skin.skinType,
        featuresText: skin.featuresText,
        chromaColors: allChromaColors.length > 0 ? allChromaColors.join(" ") : undefined,
        chromaDescriptions: allChromaDescriptions.length > 0 ? allChromaDescriptions.join(" ") : undefined,
        isBase: skin.isBase,
      });
    }
  }

  return items;
}

/**
 * Configure Fuse.js for smart search
 */
export function createFuseInstance(items: SearchableItem[]): Fuse<SearchableItem> {
  return new Fuse(items, {
    keys: [
      { name: "championName", weight: 0.4 },
      { name: "championAlias", weight: 0.3 },
      { name: "skinName", weight: 0.3 },
      { name: "chromaColors", weight: 0.25 }, // Search chroma colors (e.g., "blue", "red")
      { name: "chromaDescriptions", weight: 0.2 }, // Search chroma descriptions
      { name: "featuresText", weight: 0.15 }, // Search skin features/tags
      { name: "rarity", weight: 0.1 },
      { name: "skinType", weight: 0.1 },
    ],
    threshold: 0.4, // Lower = more strict, higher = more fuzzy (0.0 = exact, 1.0 = match anything)
    includeScore: true,
    includeMatches: true,
    minMatchCharLength: 1,
    ignoreLocation: true, // Don't care where in the string the match is
    findAllMatches: true, // Find all matches, not just the first
    shouldSort: true,
  });
}

/**
 * Smart search that finds both champions and skins
 */
export function smartSearch(
  champions: Champion[],
  query: string,
  favorites: Set<number>
): {
  championIds: Set<number>;
  championMatches: Map<number, number>; // championId -> best score (lower is better)
  skinMatches: Map<number, SearchResult[]>; // championId -> matching skins
} {
  if (!query.trim()) {
    // No query, return all champions
    return {
      championIds: new Set(champions.map((c) => c.id)),
      championMatches: new Map(),
      skinMatches: new Map(),
    };
  }

  const index = createSearchIndex(champions);
  const fuse = createFuseInstance(index);
  const results = fuse.search(query);

  const championIds = new Set<number>();
  const championMatches = new Map<number, number>(); // Track best score for each champion
  const skinMatches = new Map<number, SearchResult[]>();

  // Process results and collect champion IDs and skin matches
  for (const result of results) {
    const item = result.item;
    const score = result.score ?? 1;

    // Always include the champion if it matches or has matching skins
    championIds.add(item.championId);

    // Track champion name matches separately (prioritize these)
    if (item.type === "champion") {
      const currentBest = championMatches.get(item.championId);
      if (currentBest === undefined || score < currentBest) {
        championMatches.set(item.championId, score);
      }
    }

    // If it's a skin match, add it to the skin matches
    if (item.type === "skin" && item.skinId) {
      if (!skinMatches.has(item.championId)) {
        skinMatches.set(item.championId, []);
      }
      skinMatches.get(item.championId)!.push({
        item,
        score,
        matches: result.matches,
      });
    }
  }

  return {
    championIds,
    championMatches,
    skinMatches,
  };
}

/**
 * Filter and sort champions based on smart search results
 */
export function filterAndSortChampionsWithSearch(
  champions: Champion[],
  query: string,
  favorites: Set<number>
): Champion[] {
  const { championIds, championMatches, skinMatches } = smartSearch(champions, query, favorites);

  // Filter champions that match
  const filtered = champions.filter((c) => championIds.has(c.id));

  // Sort by relevance
  return filtered.sort((a, b) => {
    // First sort by favorite status
    const aFav = favorites.has(a.id);
    const bFav = favorites.has(b.id);
    if (aFav && !bFav) return -1;
    if (!aFav && bFav) return 1;

    // Then prioritize champions that matched by name over those that only matched through skins
    const aChampionMatch = championMatches.get(a.id);
    const bChampionMatch = championMatches.get(b.id);
    const aHasChampionMatch = aChampionMatch !== undefined;
    const bHasChampionMatch = bChampionMatch !== undefined;

    if (aHasChampionMatch && !bHasChampionMatch) return -1;
    if (!aHasChampionMatch && bHasChampionMatch) return 1;

    // If both matched by champion name, sort by score (lower is better)
    if (aHasChampionMatch && bHasChampionMatch) {
      const scoreDiff = aChampionMatch! - bChampionMatch!;
      if (scoreDiff !== 0) return scoreDiff;
    }

    // If both only matched through skins, prioritize by whether they have matching skins
    const aHasMatchingSkins = skinMatches.has(a.id);
    const bHasMatchingSkins = skinMatches.has(b.id);
    if (aHasMatchingSkins && !bHasMatchingSkins) return -1;
    if (!aHasMatchingSkins && bHasMatchingSkins) return 1;

    // Finally alphabetically
    return a.name.localeCompare(b.name);
  });
}

/**
 * Filter skins for a specific champion based on search query
 */
export function filterSkinsForChampion(
  champion: Champion | null,
  query: string
): Skin[] {
  if (!champion || !query.trim()) {
    return champion?.skins.filter((s) => !s.isBase) ?? [];
  }

  const index = createSearchIndex([champion]);
  const fuse = createFuseInstance(index);
  const results = fuse.search(query);

  const matchingSkinIds = new Set<number>();
  for (const result of results) {
    if (result.item.type === "skin" && result.item.skinId) {
      matchingSkinIds.add(result.item.skinId);
    }
  }

  // If no matches, return empty (or all if query matches champion name)
  if (matchingSkinIds.size === 0) {
    // Check if query matches champion name
    const championMatches = results.some(
      (r) => r.item.type === "champion" && r.item.championId === champion.id
    );
    if (championMatches) {
      // Query matches champion, show all skins
      return champion.skins.filter((s) => !s.isBase);
    }
    return [];
  }

  // Return matching skins
  return champion.skins.filter(
    (s) => !s.isBase && matchingSkinIds.has(s.id)
  );
}

