"use client";

import { Champion } from "@/lib/types";
import { ChampionCard } from "./ChampionCard";
import { MiscCard } from "./MiscCard";
import { Map, Languages, Shapes, Package } from "lucide-react";
import { MiscItemType } from "@/lib/store";

interface ChampionGridProps {
  champions: Champion[];
  selectedChampion: number | null;
  favorites: Set<number>;
  onSelectChampion: (id: number) => void;
  onToggleFavorite: (id: number) => void;
  isCustomMode?: boolean;
  onMiscItemClick?: (type: MiscItemType) => void;
}

export function ChampionGrid({
  champions,
  selectedChampion,
  favorites,
  onSelectChampion,
  onToggleFavorite,
  isCustomMode = false,
  onMiscItemClick,
}: ChampionGridProps) {
  // Misc card handlers
  const handleMapClick = () => {
    console.log("Map misc card clicked");
    onMiscItemClick?.("map");
  };

  const handleFontClick = () => {
    console.log("Font misc card clicked");
    onMiscItemClick?.("font");
  };

  const handleHudClick = () => {
    console.log("HUD misc card clicked");
    onMiscItemClick?.("hud");
  };

  const handleMiscClick = () => {
    console.log("Misc misc card clicked");
    onMiscItemClick?.("misc");
  };

  if (champions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
        <p>No champions found</p>
      </div>
    );
  }

  return (
    <div className="w-fit mx-auto grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5 gap-2">
      {/* Misc cards - available in both official and custom tabs */}
      <>
        <MiscCard icon={Map} type="map" onClick={handleMapClick} title="Map" />
        <MiscCard
          icon={Languages}
          type="font"
          onClick={handleFontClick}
          title="Font"
        />
        <MiscCard
          icon={Shapes}
          type="hud"
          onClick={handleHudClick}
          title="HUD"
        />
        <MiscCard
          icon={Package}
          type="misc"
          onClick={handleMiscClick}
          title="Misc"
        />
      </>

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
