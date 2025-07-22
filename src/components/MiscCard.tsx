import { Card } from "./ui/card";
import { cn } from "@/lib/utils";
import { LucideIcon } from "lucide-react";
import { MiscItemType } from "@/lib/store";

interface MiscCardProps {
  icon: LucideIcon;
  type: MiscItemType;
  onClick: () => void;
  isSelected?: boolean;
  className?: string;
  title?: string;
}

export function MiscCard({
  icon: Icon,
  type,
  onClick,
  isSelected = false,
  className,
  title,
}: MiscCardProps) {
  return (
    <Card
      className={cn(
        "relative cursor-pointer size-fit overflow-hidden border-2 p-0 flex flex-col items-center rounded-none",
        isSelected ? "border-primary" : "border-border",
        "bg-primary/20", // Different background to distinguish from champion cards
        className
      )}
      onClick={onClick}
    >
      <div className="relative w-16 h-16 flex items-center justify-center">
        <Icon size={32} className="text-primary" />
      </div>
    </Card>
  );
}
