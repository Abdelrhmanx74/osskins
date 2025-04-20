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

  // Determine if this card is selected and if a chroma is selected
  const selected = selectedSkins.get(championId);
  const isSelected =
    selectedSkins.has(championId) &&
    selectedSkins.get(championId)?.skinId === skin.id &&
    (selectedChroma
      ? selectedSkins.get(championId)?.chromaId === selectedChroma.id
      : true);

  // Show chroma image if selected, otherwise skin image
  const currentImageSrc = selectedChroma?.skinChromaPath ?? skin.skinSrc;

  const handleMouseEnter = () => {
    setIsHovering(true);
  };

  const handleMouseLeave = () => {
    setIsHovering(false);
  };

  // Select skin or chroma in one click
  const handleClick = () => {
    if (isSelected) {
      clearSelection(championId);
    } else {
      const fantomePath = selectedChroma?.fantome ?? skin.fantome;
      selectSkin(championId, skin.id, selectedChroma?.id, fantomePath);
    }
  };

  // When a chroma is selected, immediately update selection and image
  const handleChromaSelect = (chroma: CachedChroma | null) => {
    if (selectedChroma && chroma && selectedChroma.id === chroma.id) {
      // If clicking the already-selected chroma, reset to base skin
      setSelectedChroma(null);
      selectSkin(championId, skin.id, undefined, skin.fantome);
    } else {
      setSelectedChroma(chroma);
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
            width={308}
            height={560}
            className="object-cover"
            priority
          />
        )}

        {isSelected && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/40 z-10">
            <div className="bg-primary/20 p-2 rounded-full">
              <Check className="size-8 text-primary" />
            </div>
          </div>
        )}

        {/* Play button overlay on hover */}
        {!isSelected && isHovering && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/40 z-10">
            <div className="bg-primary/20 p-2 rounded-full">
              <Play className="size-8" />
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
