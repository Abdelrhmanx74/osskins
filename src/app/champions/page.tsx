"use client";

import { useState, useEffect } from "react";
import { ChampionCard } from "@/components/ChampionCard";
import { SkinCard } from "@/components/SkinCard";
import { useChampions } from "@/lib/hooks/use-champions";
import type { Champion, Skin } from "@/lib/hooks/use-champions";
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { Toaster } from "sonner";
import { SkinInjectionButton } from "@/components/skin-injection/SkinInjectionButton";
import { Button } from "@/components/ui/button";
import { RefreshCw } from "lucide-react";

export default function ChampionsPage() {
  const { champions, loading, error, hasData } = useChampions();
  const { updateData } = useDataUpdate();
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);

  useEffect(() => {
    if (!hasData) {
      void updateData();
    }
  }, [hasData, updateData]);

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <div className="text-destructive">Error: {error}</div>
        <Toaster />
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">Loading champions...</div>
        <Toaster />
      </div>
    );
  }

  if (!hasData) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">Updating champion data...</div>
        <Toaster />
      </div>
    );
  }

  if (champions.length === 0) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">No champions found</div>
        <Toaster />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Top bar with injection button */}
      <div className="flex items-center justify-end p-4 border-b">
        <div className="flex items-center gap-4">
          <Button
            onClick={() => void updateData()}
            variant="outline"
            className="flex items-center gap-2"
          >
            <RefreshCw className="h-4 w-4" />
            Update Data
          </Button>
          <SkinInjectionButton />
        </div>
      </div>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left side - Champions grid */}
        <div className="w-1/3 p-4 overflow-y-auto border-r">
          <div className="grid grid-cols-3 gap-4">
            {champions.map((champion: Champion) => (
              <ChampionCard
                key={champion.id}
                id={champion.id}
                name={champion.name}
                iconSrc={champion.iconSrc}
                onClick={() => {
                  setSelectedChampion(champion.id);
                }}
                isSelected={selectedChampion === champion.id}
              />
            ))}
          </div>
        </div>

        {/* Right side - Skins grid */}
        <div className="w-2/3 p-4 overflow-y-auto">
          {selectedChampion ? (
            <div className="grid grid-cols-3 gap-4">
              {champions
                .find((c: Champion) => c.id === selectedChampion)
                ?.skins.map((skin: Skin) => (
                  <SkinCard
                    key={skin.id}
                    championId={selectedChampion}
                    skin={skin}
                  />
                ))}
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-muted-foreground">
              Select a champion to view their skins
            </div>
          )}
        </div>
      </div>
      <Toaster />
    </div>
  );
}
