import { useState, useRef, useEffect } from "react";
import { CachedChroma } from "@/utils/api";
import { cn } from "@/lib/utils";
import Image from "next/image";

interface ChromaSelectorProps {
  chromas: CachedChroma[];
  onSelect: (chroma: CachedChroma) => void;
  selectedChromaId?: number;
}

export function ChromaSelector({
  chromas,
  onSelect,
  selectedChromaId,
}: ChromaSelectorProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [hoveredChroma, setHoveredChroma] = useState<CachedChroma | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Calculate the angle for each chroma in the circle
  const totalChromas = chromas.length;
  const angleStep = 360 / totalChromas;

  // Handle mouse enter/leave for the entire component
  const handleMouseEnter = () => {
    setIsExpanded(true);
  };

  const handleMouseLeave = (e: React.MouseEvent) => {
    // Check if we're still within the component
    if (
      containerRef.current &&
      !containerRef.current.contains(e.relatedTarget as Node)
    ) {
      setIsExpanded(false);
    }
  };

  // Add event listener to handle clicks outside the component
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsExpanded(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  return (
    <div
      className="relative"
      ref={containerRef}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* Main circular selector */}
      <div
        className={cn(
          "relative w-6 h-6 rounded-full overflow-hidden transition-all duration-300",
          isExpanded ? "scale-150" : ""
        )}
      >
        {/* Chroma color slices */}
        <div className="absolute inset-0">
          {chromas.map((chroma, index) => {
            const mainColor = chroma.colors[0] ?? "#ffffff";
            const angle = index * angleStep;
            const nextAngle = (index + 1) * angleStep;

            // Calculate points for a perfect circular slice
            const startAngle = (angle * Math.PI) / 180;
            const endAngle = (nextAngle * Math.PI) / 180;

            // Center point
            const cx = 50;
            const cy = 50;

            // Start point (center)
            const x1 = cx;
            const y1 = cy;

            // Point on the circle at start angle
            const x2 = cx + 50 * Math.cos(startAngle);
            const y2 = cy + 50 * Math.sin(startAngle);

            // Point on the circle at end angle
            const x3 = cx + 50 * Math.cos(endAngle);
            const y3 = cy + 50 * Math.sin(endAngle);

            // Create SVG path for a perfect circular slice
            const path = `M ${x1} ${y1} L ${x2} ${y2} A 50 50 0 0 1 ${x3} ${y3} Z`;

            return (
              <svg
                key={chroma.id}
                className="absolute inset-0 w-full h-full"
                viewBox="0 0 100 100"
                style={{
                  transform: `rotate(${angle}deg)`,
                  transformOrigin: "center",
                }}
              >
                <path d={path} fill={mainColor} />
              </svg>
            );
          })}
        </div>
      </div>

      {/* Expanded chroma options */}
      <div
        className={cn(
          "absolute top-0 left-0 origin-top-left transition-all duration-300",
          isExpanded
            ? "opacity-100 scale-100"
            : "opacity-0 scale-95 pointer-events-none"
        )}
      >
        {/* Invisible area to maintain hover state */}
        <div className="absolute -top-20 -left-20 w-40 h-40" />

        <div className="relative w-48 h-48">
          {chromas.map((chroma, index) => {
            const mainColor = chroma.colors[0] ?? "#ffffff";
            const secondaryColor = chroma.colors[1] ?? mainColor;
            const angle = (index * 90) / (totalChromas - 1);
            const radius = 80; // Distance from center

            // Calculate position on the quarter circle (top left)
            const x = -radius * Math.cos((angle * Math.PI) / 180);
            const y = -radius * Math.sin((angle * Math.PI) / 180);

            return (
              <button
                key={chroma.id}
                className={cn(
                  "absolute w-8 h-8 rounded-full transition-all duration-200 hover:scale-110",
                  "ring-2 ring-offset-2 ring-offset-black/50",
                  selectedChromaId === chroma.id
                    ? "ring-white"
                    : "ring-transparent hover:ring-white/50"
                )}
                style={{
                  transform: `translate(${x}px, ${y}px)`,
                  top: "0",
                  left: "0",
                }}
                onClick={() => {
                  onSelect(chroma);
                }}
                onMouseEnter={() => {
                  setHoveredChroma(chroma);
                }}
                onMouseLeave={() => {
                  setHoveredChroma(null);
                }}
              >
                {/* Two-color display with 45-degree tilt and black divider */}
                <div
                  className="size-full rounded-full rotate-45"
                  style={{
                    background: `linear-gradient(90deg, ${mainColor} 0%, ${secondaryColor} 100%)`,
                  }}
                />
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
