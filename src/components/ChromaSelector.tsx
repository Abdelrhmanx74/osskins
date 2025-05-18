import { ChromaInSummary } from "@/lib/types";
import { cn } from "@/lib/utils";

interface ChromaSelectorProps {
  chromas: ChromaInSummary[];
  onSelect: (chroma: ChromaInSummary | null) => void;
  selectedChromaId?: number;
}

export function ChromaSelector({
  chromas,
  onSelect,
  selectedChromaId,
}: ChromaSelectorProps) {
  // Create a multi-color gradient for the dot
  const gradient = `conic-gradient(${chromas
    .map(
      (c, i) =>
        `${c.colors[0] ?? "#fff"} ${(i * 100) / chromas.length}% ${
          ((i + 1) * 100) / chromas.length
        }%`
    )
    .join(", ")})`;

  return (
    <div className="relative flex flex-col items-center group">
      {/* Main dot with gradient */}
      <div
        className={cn(
          "size-7 rounded-full border border-primary shadow cursor-pointer"
        )}
        style={{ background: gradient }}
      />
      {/* Animated vertical chroma popup */}
      <div
        className={cn(
          "absolute left-1/2 -translate-x-1/2 flex flex-col gap-1 z-30 opacity-0 translate-y-4 pointer-events-none group-hover:opacity-100 group-hover:-translate-y-0 pb-8 group-hover:pointer-events-auto transition-all duration-300 origin-bottom"
        )}
        style={{
          bottom: 0,
          filter: "drop-shadow(0 4px 12px rgba(0,0,0,0.18))",
        }}
      >
        {chromas.map((chroma) => (
          <button
            key={chroma.id}
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
          />
        ))}
      </div>
    </div>
  );
}
