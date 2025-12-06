import { CachedChroma } from "@/utils/api";
import { cn } from "@/lib/utils";
import { FloatingButton, FloatingButtonItem } from "./ui/FloatingButton";
import { useMemo } from "react";

interface ChromaSelectorProps {
  chromas: CachedChroma[];
  onSelect: (chroma: CachedChroma | null) => void;
  onHover?: (chroma: CachedChroma | null) => void;
  selectedChromaId?: number;
}

interface SpiralColorWheelProps {
  colors: string[];
  size?: number;
}

/**
 * Creates a spiral color wheel SVG with curved segments
 * For <3 colors it renders straight splits (no spiral) to keep crisp visuals.
 */
function SpiralColorWheel({ colors, size = 28 }: SpiralColorWheelProps) {
  const svgContent = useMemo(() => {
    if (colors.length === 0) return null;

    const centerX = size / 2;
    const centerY = size / 2;
    const outerRadius = size / 2 - 1;
    const innerRadius = 0; // Colors meet at center

    // Special cases: 1 or 2 colors -> straight fill / split
    if (colors.length === 1) {
      // Single solid color circle
      return (
        <circle cx={centerX} cy={centerY} r={outerRadius} fill={colors[0]} />
      );
    }

    if (colors.length === 2) {
      // Two straight halves (semicircles). We'll draw each as a path from center to arc to center.
      const startAngle1 = -Math.PI / 2;
      const endAngle1 = Math.PI / 2;
      const startX1 = centerX + outerRadius * Math.cos(startAngle1);
      const startY1 = centerY + outerRadius * Math.sin(startAngle1);
      const endX1 = centerX + outerRadius * Math.cos(endAngle1);
      const endY1 = centerY + outerRadius * Math.sin(endAngle1);

      const path1 = [
        `M ${centerX} ${centerY}`,
        `L ${startX1} ${startY1}`,
        `A ${outerRadius} ${outerRadius} 0 0 1 ${endX1} ${endY1}`,
        "Z",
      ].join(" ");

      // Second semicircle (opposite)
      const startAngle2 = endAngle1;
      const endAngle2 = startAngle1 + 2 * Math.PI;
      const startX2 = centerX + outerRadius * Math.cos(startAngle2);
      const startY2 = centerY + outerRadius * Math.sin(startAngle2);
      const endX2 = centerX + outerRadius * Math.cos(endAngle2);
      const endY2 = centerY + outerRadius * Math.sin(endAngle2);

      const path2 = [
        `M ${centerX} ${centerY}`,
        `L ${startX2} ${startY2}`,
        `A ${outerRadius} ${outerRadius} 0 0 1 ${endX2} ${endY2}`,
        "Z",
      ].join(" ");

      return (
        <>
          <path d={path1} fill={colors[0]} />
          <path d={path2} fill={colors[1]} />
        </>
      );
    }

    // Fallback: 3+ colors -> spiral rendering
    const segmentCount = colors.length;
    const anglePerSegment = (2 * Math.PI) / segmentCount;

    // Spiral parameters - controls how much the dividing lines curve
    const spiralTwist = 2.0; // radians of twist from inner to outer

    const segments = colors.map((color, i) => {
      const startAngle = i * anglePerSegment - Math.PI / 2;
      const endAngle = (i + 1) * anglePerSegment - Math.PI / 2;

      const innerStartX =
        centerX + innerRadius * Math.cos(startAngle - spiralTwist);
      const innerStartY =
        centerY + innerRadius * Math.sin(startAngle - spiralTwist);
      const innerEndX =
        centerX + innerRadius * Math.cos(endAngle - spiralTwist);
      const innerEndY =
        centerY + innerRadius * Math.sin(endAngle - spiralTwist);

      const outerStartX = centerX + outerRadius * Math.cos(startAngle);
      const outerStartY = centerY + outerRadius * Math.sin(startAngle);
      const outerEndX = centerX + outerRadius * Math.cos(endAngle);
      const outerEndY = centerY + outerRadius * Math.sin(endAngle);

      const midRadius = (innerRadius + outerRadius) / 2;
      const midStartX =
        centerX + midRadius * Math.cos(startAngle - spiralTwist * 0.5);
      const midStartY =
        centerY + midRadius * Math.sin(startAngle - spiralTwist * 0.5);
      const midEndX =
        centerX + midRadius * Math.cos(endAngle - spiralTwist * 0.5);
      const midEndY =
        centerY + midRadius * Math.sin(endAngle - spiralTwist * 0.5);

      const largeArcFlag = anglePerSegment > Math.PI ? 1 : 0;

      const path = [
        `M ${innerStartX} ${innerStartY}`,
        `Q ${midStartX} ${midStartY} ${outerStartX} ${outerStartY}`,
        `A ${outerRadius} ${outerRadius} 0 ${largeArcFlag} 1 ${outerEndX} ${outerEndY}`,
        `Q ${midEndX} ${midEndY} ${innerEndX} ${innerEndY}`,
        `A ${innerRadius} ${innerRadius} 0 ${largeArcFlag} 0 ${innerStartX} ${innerStartY}`,
        "Z",
      ].join(" ");

      return <path key={i} d={path} fill={color} />;
    });

    return segments;
  }, [colors, size]);

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className="rounded-full"
      style={{
        overflow: "visible",
      }}
    >
      <defs>
        {/* Clip to circle */}
        <clipPath id="circleClip">
          <circle cx={size / 2} cy={size / 2} r={size / 2 - 1} />
        </clipPath>
      </defs>
      <g clipPath="url(#circleClip)">{svgContent}</g>
    </svg>
  );
}

export function ChromaSelector({
  chromas,
  onSelect,
  onHover,
  selectedChromaId,
}: ChromaSelectorProps) {
  // Extract primary colors from each chroma for the spiral
  const spiralColors = chromas.map((c) => c.colors[0] ?? "#fff");

  return (
    <FloatingButton
      className="relative"
      triggerContent={
        <div
          className={cn(
            "size-7 rounded-full border border-primary shadow cursor-pointer overflow-hidden flex items-center justify-center bg-background",
          )}
        >
          <SpiralColorWheel colors={spiralColors} size={28} />
        </div>
      }
    >
      {chromas.map((chroma) => (
        <FloatingButtonItem key={chroma.id}>
          <button
            type="button"
            className={cn(
              "w-6 h-6 rounded-full border flex items-center justify-center transition-all duration-200 bg-white/10 cursor-pointer relative",
              selectedChromaId === chroma.id && "border-2 border-primary",
            )}
            style={{
              background: `linear-gradient(135deg, ${chroma.colors.join(
                ", ",
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
