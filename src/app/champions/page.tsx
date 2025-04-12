'use client';

import { useState, useEffect } from 'react';
import { ChampionCard } from '@/components/ChampionCard';
import { SkinCard } from '@/components/SkinCard';
import { useChampions } from '@/lib/hooks/use-champions';
import type { Champion, Skin } from '@/lib/hooks/use-champions';
import { Button } from '@/components/ui/button';
import { useDataUpdate } from '@/lib/hooks/use-data-update';
import { Toaster } from 'sonner';

export default function ChampionsPage() {
  const { champions, loading, error, hasData } = useChampions();
  const { updateData } = useDataUpdate();
  const [selectedChampion, setSelectedChampion] = useState<number | null>(null);

  useEffect(() => {
    console.log('Champions data:', champions);
    console.log('Loading state:', loading);
    console.log('Has data:', hasData);
    console.log('Error:', error);
  }, [champions, loading, hasData, error]);

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <div className="text-destructive">Error: {error}</div>
        {!hasData && (
          <Button onClick={() => void updateData()}>
            Update Data
          </Button>
        )}
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
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <div className="text-muted-foreground">No champion data found</div>
        <Button onClick={() => void updateData()}>
          Update Data
        </Button>
        <Toaster />
      </div>
    );
  }

  if (!champions || champions.length === 0) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">No champions found</div>
        <Toaster />
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-background">
      {/* Left side - Champions grid */}
      <div className="w-1/3 p-4 overflow-y-auto border-r">
        <h2 className="text-lg font-semibold mb-4">Champions</h2>
        <div className="grid grid-cols-3 gap-4">
          {champions.map((champion: Champion) => (
            <ChampionCard
              key={champion.id}
              id={champion.id}
              name={champion.name}
              iconSrc={champion.iconSrc}
              onClick={() => setSelectedChampion(champion.id)}
              isSelected={selectedChampion === champion.id}
            />
          ))}
        </div>
      </div>

      {/* Right side - Skins grid */}
      <div className="w-2/3 p-4 overflow-y-auto">
        <h2 className="text-lg font-semibold mb-4">Skins</h2>
        {selectedChampion ? (
          <div className="grid grid-cols-3 gap-4">
            {champions
              .find((c: Champion) => c.id === selectedChampion)
              ?.skins.map((skin: Skin) => (
                <SkinCard
                  key={skin.id}
                  id={skin.id}
                  name={skin.name}
                  loadScreenSrc={skin.skinSrc}
                  isBase={skin.isBase}
                  isLegacy={skin.isLegacy}
                  chromas={skin.chromas}
                />
              ))}
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Select a champion to view their skins
          </div>
        )}
      </div>
      <Toaster />
    </div>
  );
} 