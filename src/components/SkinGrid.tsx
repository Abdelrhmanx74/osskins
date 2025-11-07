"use client";

import { Champion } from "@/lib/types";
import { SkinCard } from "./SkinCard";
import { filterSkinsForChampion } from "@/lib/utils/smart-search";

interface SkinGridProps {
  champion: Champion | null;
  searchQuery?: string;
}

export function SkinGrid({ champion, searchQuery = "" }: SkinGridProps) {
  if (!champion) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Select a champion to view their skins
      </div>
    );
  }

  const filteredSkins = filterSkinsForChampion(champion, searchQuery);

  if (filteredSkins.length === 0 && searchQuery.trim()) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        No skins found matching "{searchQuery}"
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-2 md:grid-cols-4 xl:grid-cols-5 size-fit">
      {filteredSkins.map((skin) => (
        <SkinCard key={skin.id} championId={champion.id} skin={skin} />
      ))}
    </div>
  );
}
