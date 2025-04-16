import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { CachedChroma } from "@/utils/api";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";
import { Card, CardContent, CardFooter } from "./ui/card";
import { useGameStore } from "@/lib/store";
import { Check } from "lucide-react";
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
  const isSelected =
    selectedSkins.has(championId) &&
    selectedSkins.get(championId)?.skinId === skin.id;
  const currentImageSrc = selectedChroma?.skinChromaPath ?? skin.skinSrc;

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
      className={`relative cursor-pointer size-full p-0 relative overflow-hidden ${
        isSelected ? "ring-2 ring-primary" : ""
      }`}
      onClick={handleClick}
    >
      <CardContent className="p-0 size-full relative">
        {currentImageSrc && (
          <Image
            src={currentImageSrc}
            alt={selectedChroma?.name ?? skin.name}
            width={308}
            height={560}
            className="size-full object-contain"
          />
        )}

        {isSelected && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/50">
            <Check className="h-8 w-8 text-primary" />
          </div>
        )}

        <CardFooter className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent p-4 flex flex-col justify-end">
          <div className="w-full h-fit flex items-end justify-between gap-1">
            <h3 className="text-lg font-semibold text-white mt-2">
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
