import { useState, useRef, useEffect, useCallback } from "react";
import { ChromaInSummary, Skin } from "@/lib/types";
import { useGameStore } from "@/lib/store";

export function useSkinCardLogic(championId: number, skin: Skin) {
  const { selectedSkins, selectSkin, clearSelection } = useGameStore();
  const selected = selectedSkins.get(championId);

  // Initialize selectedChroma from stored selection if it exists
  const [selectedChroma, setSelectedChroma] = useState<ChromaInSummary | null>(
    () => {
      if (selected?.skinId === skin.id && selected.chromaId) {
        return skin.chromas.find((c) => c.id === selected.chromaId) ?? null;
      }
      return null;
    }
  );

  const [isHovering, setIsHovering] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // Determine if this card is selected and if a chroma is selected
  const isSelected =
    selected?.skinId === skin.id &&
    (selectedChroma
      ? selected.chromaId === selectedChroma.id
      : !selected.chromaId);

  // Show chroma image if selected, otherwise skin image
  const currentImageSrc = selectedChroma?.skinChromaPath ?? skin.skinSrc;

  // Preload all chroma images on hover
  const handleMouseEnter = useCallback(() => {
    setIsHovering(true);
    skin.chromas.forEach((chroma) => {
      if (chroma.skinChromaPath) {
        const img = new window.Image();
        img.src = chroma.skinChromaPath;
      }
    });
  }, [skin.chromas]);

  const handleMouseLeave = () => {
    setIsHovering(false);
  };

  // Select skin or chroma in one click
  const handleClick = () => {
    if (isSelected) {
      clearSelection(championId);
    } else {
      selectSkin(
        championId,
        skin.id,
        selectedChroma?.id,
        selectedChroma?.fantome ?? skin.fantome
      );
    }
  };

  // When a chroma is selected, immediately update selection and image
  const handleChromaSelect = (chroma: ChromaInSummary | null) => {
    if (selectedChroma && chroma && selectedChroma.id === chroma.id) {
      // If clicking the already-selected chroma, reset to base skin
      setSelectedChroma(null);
      selectSkin(championId, skin.id, undefined, skin.fantome);
    } else {
      setSelectedChroma(chroma);
      selectSkin(
        championId,
        skin.id,
        chroma?.id,
        chroma?.fantome ?? skin.fantome
      );
    }
  };

  // Reset image loaded state when switching chroma/skin
  useEffect(() => {
    setImgLoaded(false);
  }, [currentImageSrc]);

  return {
    cardRef,
    selectedChroma,
    setSelectedChroma,
    isHovering,
    setIsHovering,
    imgLoaded,
    setImgLoaded,
    isSelected,
    currentImageSrc,
    handleMouseEnter,
    handleMouseLeave,
    handleClick,
    handleChromaSelect,
  };
}
