import { CachedChroma } from "@/utils/api";
import { cn } from "@/lib/utils";
import { FloatingButton, FloatingButtonItem } from "./ui/FloatingButton";

interface ChromaSelectorProps {
  chromas: CachedChroma[];
  onSelect: (chroma: CachedChroma | null) => void;
  onHover?: (chroma: CachedChroma | null) => void;
  selectedChromaId?: number;
}

export function ChromaSelector({
  chromas,
  onSelect,
  onHover,
  selectedChromaId,
}: ChromaSelectorProps) {
  // Create a multi-color gradient for the dot (trigger)
  const gradient = `conic-gradient(${chromas
    .map(
      (c, i) =>
        `${c.colors[0] ?? "#fff"} ${(i * 100) / chromas.length}% ${((i + 1) * 100) / chromas.length
        }%`
    )
    .join(", ")})`;

  return (
    <FloatingButton
      className="relative" // keep positioning behavior minimal
      triggerContent={
        <div
          className={cn(
            "size-7 rounded-full border border-primary shadow cursor-pointer"
          )}
          style={{ background: gradient }}
        />
      }
    >
      {chromas.map((chroma) => (
        <FloatingButtonItem key={chroma.id}>
          <button
            type="button"
            className={cn(
              "w-6 h-6 rounded-full border flex items-center justify-center transition-all duration-200 bg-white/10 cursor-pointer relative",
              selectedChromaId === chroma.id && "border-2 border-primary"
            )}
            style={{
              background: `linear-gradient(135deg, ${chroma.colors.join(
                ", "
              )})`,
              boxShadow: "0 1px 4px 0 rgba(0,0,0,0.10)",
            }}
            onClick={(e) => {
              e.stopPropagation();
              onSelect(chroma);
            }}
            onMouseEnter={() => {
              onHover?.(chroma);
            }}
            onMouseLeave={() => onHover?.(null)}
          />
        </FloatingButtonItem>
      ))}
    </FloatingButton>
  );
}
