import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { CachedChroma } from "@/utils/api";
import { ChromaSelector } from "./ChromaSelector";
import Image from "next/image";
import { Card, CardContent, CardFooter } from "./ui/card";

interface SkinCardProps {
  id: number;
  name: string;
  loadScreenSrc: string;
  isBase?: boolean;
  isLegacy?: boolean;
  chromas?: CachedChroma[];
}

export function SkinCard({
  id,
  name,
  loadScreenSrc,
  isBase,
  isLegacy,
  chromas,
}: SkinCardProps) {
  const [selectedChroma, setSelectedChroma] = useState<CachedChroma | null>(
    null
  );
  const currentImageSrc = selectedChroma?.skinChromaPath ?? loadScreenSrc;

  return (
    <Card className="size-full p-0 relative overflow-hidden">
      <CardContent className="p-0 size-full relative">
        {currentImageSrc && (
          <Image
            src={currentImageSrc}
            alt={name}
            width={308}
            height={560}
            className="size-full object-contain"
          />
        )}

        <CardFooter className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent p-4 flex flex-col justify-end">
          <div className="w-full h-fit flex items-end justify-between gap-1">
            <h3 className="text-lg font-semibold text-white mt-2">
              {selectedChroma?.name ?? name}
            </h3>

            {/* Chroma Selector positioned in bottom right */}
            {chromas && chromas.length > 0 && (
              <ChromaSelector
                chromas={chromas}
                onSelect={setSelectedChroma}
                selectedChromaId={selectedChroma?.id}
              />
            )}
          </div>
        </CardFooter>
      </CardContent>
    </Card>
  );
}
