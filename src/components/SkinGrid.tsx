"use client";

import { Champion } from "@/lib/types";
import { SkinCard } from "./SkinCard";
import { filterSkinsForChampion } from "@/lib/utils/smart-search";
import { memo, useMemo } from "react";
import { useImagePreloader } from "@/lib/hooks/use-cached-image";
import { motion } from "framer-motion";

interface SkinGridProps {
  champion: Champion | null;
  searchQuery?: string;
}

export const SkinGrid = memo(function SkinGrid({ champion, searchQuery = "" }: SkinGridProps) {
  const filteredSkins = useMemo(
    () => (champion ? filterSkinsForChampion(champion, searchQuery) : []),
    [champion, searchQuery]
  );

  // Preload all skin images in the background
  const imagesToPreload = useMemo(
    () => filteredSkins.map((skin) => skin.skinSrc),
    [filteredSkins]
  );
  useImagePreloader(imagesToPreload);

  if (!champion) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Select a champion to view their skins
      </div>
    );
  }

  if (filteredSkins.length === 0 && searchQuery.trim()) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        No skins found matching &quot;{searchQuery}&quot;
      </div>
    );
  }

  return (
    <motion.div
      layout
      className="grid grid-cols-2 gap-2 md:grid-cols-4 xl:grid-cols-5 size-fit pb-2"
      transition={{ duration: 0.14, ease: "easeOut" }}
    >
      {filteredSkins.map((skin) => (
        <motion.div key={skin.id} layout transition={{ duration: 0.14, ease: "easeOut" }}>
          <SkinCard championId={champion.id} skin={skin} />
        </motion.div>
      ))}
    </motion.div>
  );
});
