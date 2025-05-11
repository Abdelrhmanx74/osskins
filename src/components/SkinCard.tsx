import React, { Suspense } from "react";
import { cn } from "@/lib/utils";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";
import { Card, CardContent, CardFooter } from "./ui/card";
import { Check, Play } from "lucide-react";
import { Skin } from "@/lib/types";
import { Skeleton } from "./ui/skeleton";
import { useSkinCardLogic } from "@/lib/hooks/use-skin-card-logic";

interface SkinCardProps {
  championId: number;
  skin: Skin;
}

export const SkinCard = React.memo(function SkinCard({
  championId,
  skin,
}: SkinCardProps) {
  const {
    cardRef,
    selectedChroma,
    isHovering,
    imgLoaded,
    isSelected,
    currentImageSrc,
    handleMouseEnter,
    handleMouseLeave,
    handleClick,
    handleChromaSelect,
    setImgLoaded,
  } = useSkinCardLogic(championId, skin);

  return (
    <Card
      ref={cardRef}
      className={cn(
        "size-full relative cursor-pointer p-0 rounded-none overflow-hidden transition-all duration-300",
        isSelected ? "ring-2 ring-primary" : ""
      )}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {!imgLoaded && <Skeleton className="absolute inset-0 w-full h-full" />}
      {currentImageSrc && (
        <Image
          src={currentImageSrc}
          alt={selectedChroma?.name ?? skin.name}
          width={308}
          height={560}
          className={cn(
            "object-cover transition-opacity duration-300",
            imgLoaded ? "opacity-100" : "opacity-0"
          )}
          onLoad={() => setImgLoaded(true)}
          onLoadingComplete={() => setImgLoaded(true)}
          placeholder="blur"
          blurDataURL="/osskins-screenshot.png" // fallback placeholder, replace with a real LQIP if available
          priority={false}
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
    </Card>
  );
});
