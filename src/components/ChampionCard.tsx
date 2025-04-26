import Image from "next/image";
import { Card } from "./ui/card";
import { cn } from "@/lib/utils";
import { Heart } from "lucide-react";
import { useRef, useState } from "react";
import { Skeleton } from "./ui/skeleton";

interface ChampionCardProps {
  champion: {
    id: number;
    name: string;
    iconSrc: string;
  };
  onClick: () => void;
  isSelected: boolean;
  isFavorite?: boolean;
  onToggleFavorite?: () => void;
  className?: string;
}

export function ChampionCard({
  champion,
  onClick,
  isSelected,
  isFavorite = false,
  onToggleFavorite,
  className,
}: ChampionCardProps) {
  const cardRef = useRef<HTMLDivElement>(null);
  const [imgLoaded, setImgLoaded] = useState(false);

  // Handle favorite click without triggering the main card click
  const handleFavoriteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (onToggleFavorite) {
      onToggleFavorite();
    }
  };

  return (
    <Card
      ref={cardRef}
      className={cn(
        "relative cursor-pointer size-fit overflow-hidden border-2 p-0 flex flex-col items-center rounded-none",
        isSelected ? "border-primary" : "border-border",
        isFavorite ? "bg-primary/5" : "",
        className
      )}
      onClick={onClick}
    >
      <div className="relative w-16 h-16 flex items-center justify-center">
        {!imgLoaded && <Skeleton className="absolute inset-0 w-full h-full" />}
        <Image
          src={champion.iconSrc}
          alt={`${champion.name} icon`}
          width={64}
          height={64}
          className={cn(
            "object-contain transition-opacity duration-200",
            imgLoaded ? "opacity-100" : "opacity-0"
          )}
          onLoad={() => {
            setImgLoaded(true);
          }}
          onLoadingComplete={() => {
            setImgLoaded(true);
          }}
        />
      </div>
      {/* <p className="text-sm text-center truncate max-w-full">{champion.name}</p> */}

      {onToggleFavorite && (
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
      )}
    </Card>
  );
}
