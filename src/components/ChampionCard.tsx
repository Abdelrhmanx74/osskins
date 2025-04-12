import Image from "next/image";
import { Button } from "./ui/button";

interface ChampionCardProps {
  id: number;
  name: string;
  iconSrc: string;
  onClick: () => void;
  isSelected: boolean;
}

export function ChampionCard({ id, name, iconSrc, onClick, isSelected }: ChampionCardProps) {
  return (
    <Button
      variant={isSelected ? "outline" : "ghost"}
      onClick={onClick}
    >
          <Image
            src={iconSrc}
            alt={`${name} icon`}
            width={64}
            height={64}
          />
    </Button>
  );
}