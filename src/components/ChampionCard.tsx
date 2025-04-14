import Image from "next/image";
import { Button } from "./ui/button";

interface ChampionCardProps {
  id: number;
  name: string;
  iconSrc: string;
  onClick: () => void;
  isSelected: boolean;
}

export function ChampionCard({
  id,
  name,
  iconSrc,
  onClick,
  isSelected,
}: ChampionCardProps) {
  return (
    <Button
      className="py-0 px-0 size-fit"
      variant={isSelected ? "outline" : "ghost"}
      onClick={onClick}
      asChild
    >
      <Image
        src={iconSrc}
        alt={`${name} icon`}
        width={64}
        height={64}
        className="object-contain"
      />
    </Button>
  );
}
