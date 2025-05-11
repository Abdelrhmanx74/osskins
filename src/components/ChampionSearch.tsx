import { useState, useEffect, useRef } from "react";
import { Search } from "lucide-react";
import { Input } from "./ui/input";
import { Champion } from "@/lib/types";

interface ChampionSearchProps {
  champions: Champion[];
  onSelect: (championId: number) => void;
  selectedChampionId: number | null;
  searchQuery: string;
  onSearchChange: (query: string) => void;
}

function getMatchScore(championName: string, query: string): number {
  const normalizedName = championName.toLowerCase();
  const normalizedQuery = query.toLowerCase();

  // Exact match gets highest score
  if (normalizedName === normalizedQuery) return 100;

  // Starts with query gets high score
  if (normalizedName.startsWith(normalizedQuery)) return 80;

  // Contains query as a word gets medium score
  if (normalizedName.includes(` ${normalizedQuery}`)) return 60;

  // Contains query gets low score
  if (normalizedName.includes(normalizedQuery)) return 40;

  // No match
  return 0;
}

export function ChampionSearch({
  champions,
  onSelect,
  selectedChampionId,
  searchQuery,
  onSearchChange,
}: ChampionSearchProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [hasFocus, setHasFocus] = useState(false);

  // Handle keyboard input when not focused on input
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      // Skip if input or any other text input has focus
      if (
        hasFocus ||
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      // Handle direct text input when not focused on input elements
      if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault(); // Prevent default to avoid double input
        onSearchChange(e.key);
        inputRef.current?.focus();
      }

      // Handle backspace
      if (e.key === "Backspace") {
        e.preventDefault();
        onSearchChange(searchQuery.slice(0, -1));
        inputRef.current?.focus();
      }

      // Handle escape to clear search
      if (e.key === "Escape") {
        e.preventDefault();
        onSearchChange("");
        inputRef.current?.blur();
      }
    };

    document.addEventListener("keydown", down);
    return () => {
      document.removeEventListener("keydown", down);
    };
  }, [searchQuery, onSearchChange, hasFocus]);

  return (
    <Input
      ref={inputRef}
      type="search"
      icon={<Search size={16} />}
      placeholder="Search champions..."
      value={searchQuery}
      onFocus={() => {
        setHasFocus(true);
      }}
      onBlur={() => {
        setHasFocus(false);
      }}
      onChange={(e) => {
        onSearchChange(e.target.value);
      }}
    />
  );
}
