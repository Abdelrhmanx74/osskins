import { Champion } from "@/lib/types";

/**
 * Filter and sort champions based on search query and favorites
 */
export function filterAndSortChampions(
  champions: Champion[],
  searchQuery: string,
  favorites: Set<number>
): Champion[] {
  return champions
    .filter((champion) =>
      champion.name.toLowerCase().includes(searchQuery.toLowerCase())
    )
    .sort((a, b) => {
      // First sort by favorite status
      const aFav = favorites.has(a.id);
      const bFav = favorites.has(b.id);
      if (aFav && !bFav) return -1;
      if (!aFav && bFav) return 1;

      // Then by search relevance
      if (searchQuery) {
        const aStarts = a.name
          .toLowerCase()
          .startsWith(searchQuery.toLowerCase());
        const bStarts = b.name
          .toLowerCase()
          .startsWith(searchQuery.toLowerCase());
        if (aStarts && !bStarts) return -1;
        if (!aStarts && bStarts) return 1;
      }

      // Finally alphabetically
      return a.name.localeCompare(b.name);
    });
}

/**
 * Get champion match score for search relevance
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
