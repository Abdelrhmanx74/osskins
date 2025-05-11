"use client";

import React from "react";
import { Champion } from "@/lib/types";
import { Card, CardContent } from "./ui/card";
import { Skeleton } from "./ui/skeleton";
import { ChampionCard } from "@/components/ChampionCard";

interface ChampionGridProps {
  champions: Champion[];
  selectedChampion: number | null;
  favorites: Set<number>;
  onSelectChampion: (id: number) => void;
  onToggleFavorite: (id: number) => void;
}

function ChampionGridLoading() {
  return (
    <div className="w-full h-fit mx-auto grid grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-2">
      {Array.from({ length: 45 }).map((_, i) => (
        <Skeleton key={i} className="aspect-square size-[64px]" />
      ))}
    </div>
  );
}

export default function ChampionGrid({
  champions,
  selectedChampion,
  favorites,
  onSelectChampion,
  onToggleFavorite,
}: ChampionGridProps) {
  if (champions.length === 0) {
    return <ChampionGridLoading />;
  }

  // Sort champions so favorites are on top
  const sortedChampions = [...champions].sort((a, b) => {
    const aFav = favorites.has(a.id) ? 1 : 0;
    const bFav = favorites.has(b.id) ? 1 : 0;
    return bFav - aFav;
  });

  return (
    <div className="w-fit mx-auto grid grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-2">
      {sortedChampions.map((champion: Champion) => (
        <ChampionCard
          key={champion.id}
          champion={champion}
          isSelected={selectedChampion === champion.id}
          isFavorite={favorites.has(champion.id)}
          onToggleFavorite={() => {
            onToggleFavorite(champion.id);
          }}
          onClick={() => {
            onSelectChampion(champion.id);
          }}
        />
      ))}
    </div>
  );
}
