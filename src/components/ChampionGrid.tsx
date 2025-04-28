"use client";

import { Champion } from "@/lib/types";
import { ChampionCard } from "./ChampionCard";

interface ChampionGridProps {
  champions: Champion[];
  selectedChampion: number | null;
  favorites: Set<number>;
  onSelectChampion: (id: number) => void;
  onToggleFavorite: (id: number) => void;
}

export function ChampionGrid({
  champions,
  selectedChampion,
  favorites,
  onSelectChampion,
  onToggleFavorite,
}: ChampionGridProps) {
  if (champions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
        <p>No champions found</p>
      </div>
    );
  }

  return (
    <div className="w-fit mx-auto grid grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-2">
      {champions.map((champion) => (
        <ChampionCard
          key={champion.id}
          champion={champion}
          isSelected={selectedChampion === champion.id}
          isFavorite={favorites.has(champion.id)}
          onToggleFavorite={() => {
            onToggleFavorite(champion.id);
          }}
          onClick={() => {
            console.log(
              `Selected champion: ${champion.name} (ID: ${champion.id})`
            );
            onSelectChampion(champion.id);
          }}
          className="champion-card"
        />
      ))}
    </div>
  );
}
