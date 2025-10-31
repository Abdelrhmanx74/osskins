import React, { useState, useRef } from "react";
import { cn } from "@/lib/utils";
import { CachedChroma } from "@/utils/api";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";
import { Card, CardContent, CardFooter } from "./ui/card";
import { useGameStore } from "@/lib/store";
import { Check, Play } from "lucide-react";
import { Skin } from "@/lib/types";
import { Skeleton } from "./ui/skeleton";

interface ManualSkinCardProps {
  championId: number;
  skin: Skin;
}

export function ManualSkinCard({ championId, skin }: ManualSkinCardProps) {
  const { manualSelectedSkins, selectManualSkin, clearManualSelection } = useGameStore();
  const selected = manualSelectedSkins.get(championId);

  // Initialize selectedChroma from stored selection if it exists
  const [selectedChroma, setSelectedChroma] = useState<CachedChroma | null>(
    () => {
      if (selected?.skinId === skin.id && selected.chromaId) {
        return skin.chromas.find((c) => c.id === selected.chromaId) ?? null;
      }
      return null;
    }
  );

  const [isHovering, setIsHovering] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // Determine if this card is selected and if a chroma is selected
  const isSelected =
    selected?.skinId === skin.id &&
    (selectedChroma
      ? selected.chromaId === selectedChroma.id
      : !selected.chromaId);

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
      clearManualSelection(championId);
    } else {
      selectManualSkin(
        championId,
        skin.id,
        selectedChroma?.id,
        selectedChroma?.fantome ?? skin.fantome
      );
    }
  };

  // When a chroma is selected, immediately update selection and image
  const handleChromaSelect = (chroma: CachedChroma | null) => {
    if (selectedChroma && chroma && selectedChroma.id === chroma.id) {
      // If clicking the already-selected chroma, reset to base skin
      setSelectedChroma(null);
      selectManualSkin(championId, skin.id, undefined, skin.fantome);
    } else {
      setSelectedChroma(chroma);
      selectManualSkin(
        championId,
        skin.id,
        chroma?.id,
        chroma?.fantome ?? skin.fantome
      );
    }
  };

  return (
    <Card
      ref={cardRef}
      className={cn(
        "size-full min-h-[420px] relative cursor-pointer p-0 rounded-none overflow-hidden transition-all duration-300",
        isSelected ? "ring-2 ring-primary" : ""
      )}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <CardContent className="p-0 h-full w-full relative">
        {!imgLoaded && <Skeleton className="absolute inset-0 w-full h-full" />}
        {currentImageSrc && (
          <Image
            src={currentImageSrc}
            alt={selectedChroma?.name ?? skin.name}
            width={310}
            height={420}
            className={cn(
              "object-cover transition-opacity duration-200",
              imgLoaded ? "opacity-100" : "opacity-0"
            )}
            onLoad={() => {
              setImgLoaded(true);
            }}
            onLoadingComplete={() => {
              setImgLoaded(true);
            }}
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
            {skin.chromas.length > 0 && (
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
