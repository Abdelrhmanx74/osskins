"use client";

import { Champion } from "@/lib/types";
import { SkinCard } from "./SkinCard";

interface SkinGridProps {
  champion: Champion | null;
}

export function SkinGrid({ champion }: SkinGridProps) {
  if (!champion) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Select a champion to view their skins
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-2 md:grid-cols-4 xl:grid-cols-5 size-fit">
      {champion.skins
        .filter((skin) => !skin.isBase)
        .map((skin) => (
          <SkinCard key={skin.id} championId={champion.id} skin={skin} />
        ))}
    </div>
  );
}
