import React, { useState, useRef } from "react";
import { cn } from "@/lib/utils";
import { CachedChroma } from "@/utils/api";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";
import { Card, CardContent, CardFooter } from "./ui/card";
import { useGameStore } from "@/lib/store";
import { Check, Play } from "lucide-react";
import type { Skin } from "@/lib/hooks/use-champions";

interface SkinCardProps {
  championId: number;
  skin: Skin;
}

export function SkinCard({ championId, skin }: SkinCardProps) {
  const { selectedSkins, selectSkin, clearSelection } = useGameStore();
  const [selectedChroma, setSelectedChroma] = useState<CachedChroma | null>(
    null
  );
  const [isHovering, setIsHovering] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  const isSelected =
    selectedSkins.has(championId) &&
    selectedSkins.get(championId)?.skinId === skin.id;

  const currentImageSrc = selectedChroma?.skinChromaPath ?? skin.skinSrc;

  const handleMouseEnter = () => {
    setIsHovering(true);
  };

  const handleMouseLeave = () => {
    setIsHovering(false);
  };

  const handleClick = () => {
    if (isSelected) {
      clearSelection(championId);
    } else {
      // Pass the fantome path from the skin data
      const fantomePath = selectedChroma?.fantome ?? skin.fantome;
      selectSkin(championId, skin.id, selectedChroma?.id, fantomePath);
    }
  };

  const handleChromaSelect = (chroma: CachedChroma | null) => {
    setSelectedChroma(chroma);
    if (isSelected) {
      // Pass the fantome path from the selected chroma or base skin
      const fantomePath = chroma?.fantome ?? skin.fantome;
      selectSkin(championId, skin.id, chroma?.id, fantomePath);
    }
  };

  return (
    <Card
      ref={cardRef}
      className={cn(
        "relative cursor-pointer w-full h-80 p-0 overflow-hidden transition-all duration-300",
        isSelected ? "ring-2 ring-primary" : ""
      )}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <CardContent className="p-0 h-full w-full relative">
        {currentImageSrc && (
          <Image
            src={currentImageSrc}
            alt={selectedChroma?.name ?? skin.name}
            fill
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
            className="object-cover object-top"
            priority
          />
        )}

        {isSelected && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/60 z-10">
            <div className="bg-primary/20 p-4 rounded-full">
              <Check className="h-10 w-10 text-primary" />
            </div>
          </div>
        )}

        {/* Play button overlay on hover */}
        {!isSelected && isHovering && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/40 z-10">
            <div className="bg-primary/20 p-3 rounded-full">
              <Play className="h-8 w-8 text-white" fill="white" />
            </div>
          </div>
        )}

        <CardFooter className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/90 via-black/60 to-transparent p-4 flex flex-col justify-end z-20">
          <div className="w-full h-fit flex items-end justify-between gap-1">
            <h3 className="text-lg font-semibold text-white drop-shadow-md">
              {selectedChroma?.name ?? skin.name}
            </h3>

            {/* Chroma Selector positioned in bottom right */}
            {skin.chromas && skin.chromas.length > 0 && (
              <ChromaSelector
                chromas={skin.chromas}
                onSelect={handleChromaSelect}
                selectedChromaId={selectedChroma?.id}
              />
            )}
          </div>
        </CardFooter>
      </CardContent>
    </Card>
  );
}
