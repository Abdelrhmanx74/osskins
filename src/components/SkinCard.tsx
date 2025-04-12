import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { CachedChroma } from "@/utils/api";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";

interface SkinCardProps {
  id: number;
  name: string;
  loadScreenSrc: string;
  isBase?: boolean;
  isLegacy?: boolean;
  chromas?: CachedChroma[];
}

export function SkinCard({ id, name, loadScreenSrc, isBase, isLegacy, chromas }: SkinCardProps) {
  const [selectedChroma, setSelectedChroma] = useState<CachedChroma | null>(null);
  const currentImageSrc = selectedChroma?.skinChromaPath ?? loadScreenSrc;

  return (
    <div className="group relative overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm">
      <div className="size-full relative">
        {currentImageSrc && (
          <Image
            src={currentImageSrc}
            alt={name}
            width={308}
            height={560}
            className="transition-transform duration-300 group-hover:scale-105"
          />
        )}
        
        <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent p-4 flex flex-col justify-end">
          <h3 className="text-lg font-semibold text-white mt-2">
            {selectedChroma?.name ?? name}
          </h3>
        </div>
        
        {/* Chroma Selector positioned in bottom right */}
        {chromas && chromas.length > 0 && (
          <div className="absolute bottom-4 right-4 z-10">
            <ChromaSelector
              chromas={chromas}
              onSelect={setSelectedChroma}
              selectedChromaId={selectedChroma?.id}
            />
          </div>
        )}
      </div>
    </div>
  );
} 