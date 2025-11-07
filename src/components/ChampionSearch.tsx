import { useState, useEffect, useRef } from "react";
import { Search } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { Input } from "./ui/input";
import { Champion } from "@/lib/types";

interface ChampionSearchProps {
  champions: Champion[];
  onSelect: (championId: number) => void;
  selectedChampionId: number | null;
  searchQuery: string;
  onSearchChange: (query: string) => void;
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
  const { t } = useI18n();

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
      className="rounded-none"
      icon={<Search size={16} />}
      placeholder={t("search.placeholder")}
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
