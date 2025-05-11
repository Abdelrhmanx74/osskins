import { useEffect } from "react";

export function useSearchKeyboardEvents({
  hasFocus,
  searchQuery,
  onSearchChange,
  inputRef,
}: {
  hasFocus: boolean;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  inputRef: React.RefObject<HTMLInputElement>;
}) {
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (
        hasFocus ||
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }
      if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        onSearchChange(e.key);
        inputRef.current?.focus();
      }
      if (e.key === "Backspace") {
        e.preventDefault();
        onSearchChange(searchQuery.slice(0, -1));
        inputRef.current?.focus();
      }
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
  }, [searchQuery, onSearchChange, hasFocus, inputRef]);
}
