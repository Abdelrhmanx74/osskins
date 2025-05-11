import React from "react";
import { Champion } from "@/lib/types";
import { cn } from "@/lib/utils";
import { Card } from "./ui/card";
import Image from "next/image";
import { Heart } from "lucide-react";

interface ChampionCardProps {
  champion: Champion;
  isSelected: boolean;
  isFavorite: boolean;
  onToggleFavorite: () => void;
  onClick: () => void;
}

export function ChampionCard({
  champion,
  isSelected,
  isFavorite,
  onToggleFavorite,
  onClick,
}: ChampionCardProps) {
  const handleFavoriteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onToggleFavorite();
  };

  return (
    <Card
      className={cn(
        "relative aspect-square cursor-pointer overflow-hidden transition-all p-0 rounded-none",
        isSelected && "ring ring-primary"
      )}
      onClick={onClick}
    >
      <button
        className="absolute top-1 right-1 p-1 hover:bg-background/80 rounded-full"
        onClick={handleFavoriteClick}
      >
        <Heart
          size={16}
          className={cn(
            "transition-colors",
            isFavorite ? "fill-primary text-primary" : "text-muted-foreground"
          )}
        />
      </button>

      <Image
        src={champion.iconSrc}
        alt={champion.name}
        className="size-full object-cover"
        loading="lazy"
        width={64}
        height={64}
        unoptimized
      />
    </Card>
  );
}
