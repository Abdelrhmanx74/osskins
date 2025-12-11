"use client";

import { Champion } from "@/lib/types";
import { ChampionCard } from "./ChampionCard";
import { MiscCard } from "./MiscCard";
import { Map, Languages, Shapes, Package } from "lucide-react";
import { MiscItemType } from "@/lib/store";
import { memo, useCallback, useMemo } from "react";
import { useImagePreloader } from "@/lib/hooks/use-cached-image";
import { motion } from "framer-motion";

interface ChampionGridProps {
  champions: Champion[];
  selectedChampion: number | null;
  favorites: Set<number>;
  onSelectChampion: (id: number) => void;
  onToggleFavorite: (id: number) => void;
  isCustomMode?: boolean;
  onMiscItemClick?: (type: MiscItemType) => void;
  disableLayout?: boolean;
}

export const ChampionGrid = memo(function ChampionGrid({
  champions,
  selectedChampion,
  favorites,
  onSelectChampion,
  onToggleFavorite,
  isCustomMode = false,
  onMiscItemClick,
  disableLayout = false,
}: ChampionGridProps) {
  // Preload all champion icons
  const iconSources = useMemo(
    () => champions.map((champ) => champ.iconSrc),
    [champions]
  );
  useImagePreloader(iconSources);

  // Misc card handlers
  const handleMapClick = useCallback(() => {
    console.log("Map misc card clicked");
    onMiscItemClick?.("map");
  }, [onMiscItemClick]);

  const handleFontClick = useCallback(() => {
    console.log("Font misc card clicked");
    onMiscItemClick?.("font");
  }, [onMiscItemClick]);

  const handleHudClick = useCallback(() => {
    console.log("HUD misc card clicked");
    onMiscItemClick?.("hud");
  }, [onMiscItemClick]);

  const handleMiscClick = useCallback(() => {
    console.log("Misc misc card clicked");
    onMiscItemClick?.("misc");
  }, [onMiscItemClick]);

  if (champions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
        <p>No champions found</p>
      </div>
    );
  }

  const GridWrapper = disableLayout ? "div" : motion.div;
  const ItemWrapper = disableLayout ? "div" : motion.div;

  return (
    <GridWrapper
      {...(!disableLayout && {
        layout: true,
        transition: { duration: 0.16, ease: "easeOut" },
      })}
      className="w-fit mx-auto grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5 gap-2"
    >
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
        <ItemWrapper
          key={champion.id}
          {...(!disableLayout && {
            layout: true,
            transition: { duration: 0.16, ease: "easeOut" },
          })}
        >
          <ChampionCard
            champion={champion}
            isSelected={selectedChampion === champion.id}
            isFavorite={favorites.has(champion.id)}
            onToggleFavorite={() => onToggleFavorite(champion.id)}
            onClick={() => onSelectChampion(champion.id)}
            className="champion-card"
          />
        </ItemWrapper>
      ))}
    </GridWrapper>
  );
});
