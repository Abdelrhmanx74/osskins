import { Champion } from "@/lib/types";
import { filterAndSortChampionsWithSearch } from "./smart-search";

/**
 * Filter and sort champions based on search query and favorites
 * Now uses smart search that searches both champions and skins
 */
export function filterAndSortChampions(
  champions: Champion[],
  searchQuery: string,
  favorites: Set<number>
): Champion[] {
  return filterAndSortChampionsWithSearch(champions, searchQuery, favorites);
}

/**
 * Get champion match score for search relevance
 * @deprecated Use smart search instead
 */
export function getMatchScore(championName: string, query: string): number {
  const normalizedName = championName.toLowerCase();
  const normalizedQuery = query.toLowerCase();

  // Exact match gets highest score
  if (normalizedName === normalizedQuery) return 100;

  // Starts with query gets high score
  if (normalizedName.startsWith(normalizedQuery)) return 80;

  // Contains query as a word gets medium score
  if (normalizedName.includes(` ${normalizedQuery}`)) return 60;

  // Contains query gets low score
  if (normalizedName.includes(normalizedQuery)) return 40;

  // No match
  return 0;
}
