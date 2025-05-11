"use client";

import { useEffect, useState, useRef, useCallback } from "react";
import { useTheme } from "next-themes";
import { cn } from "@/lib/utils";
import { SunIcon, MoonIcon } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useThemeToneContext } from "@/components/providers/ThemeToneProvider";
import { Label } from "./ui/label";

// Define theme tone options with palettes for both light and dark modes
// Now exported so it can be used by ThemeInitializer
export const TONES = [
  {
    name: "Gray",
    value: "gray",
    palette: {
      primary: "oklch(0.55 0 0)",
      background: "oklch(0.98 0 0)",
      backgroundDark: "oklch(0.18 0 0)",
      foreground: "oklch(0.18 0 0)",
      foregroundDark: "oklch(0.98 0 0)",
      border: "oklch(0.85 0 0)",
      borderDark: "oklch(0.25 0 0)",
      accent: "oklch(0.7 0 0)",
      accentDark: "oklch(0.3 0 0)",
      muted: "oklch(0.93 0 0)",
      mutedDark: "oklch(0.22 0 0)",
    },
  },
  {
    name: "Blue",
    value: "blue",
    palette: {
      primary: "oklch(0.65 0.13 250)",
      background: "oklch(0.98 0.02 250)",
      backgroundDark: "oklch(0.22 0.06 250)",
      foreground: "oklch(0.18 0.01 250)",
      foregroundDark: "oklch(0.98 0.01 250)",
      border: "oklch(0.85 0.04 250)",
      borderDark: "oklch(0.3 0.05 250)",
      accent: "oklch(0.7 0.09 250)",
      accentDark: "oklch(0.35 0.09 250)",
      muted: "oklch(0.93 0.02 250)",
      mutedDark: "oklch(0.28 0.03 250)",
    },
  },
  {
    name: "Red",
    value: "red",
    palette: {
      primary: "oklch(0.65 0.18 25)",
      background: "oklch(0.98 0.02 25)",
      backgroundDark: "oklch(0.22 0.09 25)",
      foreground: "oklch(0.18 0.01 25)",
      foregroundDark: "oklch(0.98 0.01 25)",
      border: "oklch(0.85 0.04 25)",
      borderDark: "oklch(0.3 0.06 25)",
      accent: "oklch(0.7 0.09 25)",
      accentDark: "oklch(0.35 0.12 25)",
      muted: "oklch(0.93 0.02 25)",
      mutedDark: "oklch(0.28 0.03 25)",
    },
  },
  {
    name: "Rose",
    value: "rose",
    palette: {
      primary: "oklch(0.7 0.13 20)",
      background: "oklch(0.98 0.02 20)",
      backgroundDark: "oklch(0.22 0.09 20)",
      foreground: "oklch(0.18 0.01 20)",
      foregroundDark: "oklch(0.98 0.01 20)",
      border: "oklch(0.85 0.04 20)",
      borderDark: "oklch(0.3 0.06 20)",
      accent: "oklch(0.7 0.09 20)",
      accentDark: "oklch(0.35 0.12 20)",
      muted: "oklch(0.93 0.02 20)",
      mutedDark: "oklch(0.28 0.03 20)",
    },
  },
  {
    name: "Yellow",
    value: "yellow",
    palette: {
      primary: "oklch(0.9 0.18 100)",
      background: "oklch(0.99 0.03 100)",
      backgroundDark: "oklch(0.32 0.09 100)",
      foreground: "oklch(0.22 0.01 100)",
      foregroundDark: "oklch(0.98 0.01 100)",
      border: "oklch(0.93 0.04 100)",
      borderDark: "oklch(0.38 0.06 100)",
      accent: "oklch(0.8 0.09 100)",
      accentDark: "oklch(0.4 0.12 100)",
      muted: "oklch(0.97 0.02 100)",
      mutedDark: "oklch(0.35 0.03 100)",
    },
  },
  {
    name: "Orange",
    value: "orange",
    palette: {
      primary: "oklch(0.85 0.18 60)",
      background: "oklch(0.99 0.03 60)",
      backgroundDark: "oklch(0.32 0.09 60)",
      foreground: "oklch(0.22 0.01 60)",
      foregroundDark: "oklch(0.98 0.01 60)",
      border: "oklch(0.93 0.04 60)",
      borderDark: "oklch(0.38 0.06 60)",
      accent: "oklch(0.8 0.09 60)",
      accentDark: "oklch(0.4 0.12 60)",
      muted: "oklch(0.97 0.02 60)",
      mutedDark: "oklch(0.35 0.03 60)",
    },
  },
  {
    name: "Green",
    value: "green",
    palette: {
      primary: "oklch(0.7 0.13 140)",
      background: "oklch(0.98 0.02 140)",
      backgroundDark: "oklch(0.22 0.09 140)",
      foreground: "oklch(0.18 0.01 140)",
      foregroundDark: "oklch(0.98 0.01 140)",
      border: "oklch(0.85 0.04 140)",
      borderDark: "oklch(0.3 0.06 140)",
      accent: "oklch(0.7 0.09 140)",
      accentDark: "oklch(0.35 0.12 140)",
      muted: "oklch(0.93 0.02 140)",
      mutedDark: "oklch(0.28 0.03 140)",
    },
  },
  {
    name: "Violet",
    value: "violet",
    palette: {
      primary: "oklch(0.7 0.13 300)",
      background: "oklch(0.98 0.02 300)",
      backgroundDark: "oklch(0.22 0.09 300)",
      foreground: "oklch(0.18 0.01 300)",
      foregroundDark: "oklch(0.98 0.01 300)",
      border: "oklch(0.85 0.04 300)",
      borderDark: "oklch(0.3 0.06 300)",
      accent: "oklch(0.7 0.09 300)",
      accentDark: "oklch(0.35 0.12 300)",
      muted: "oklch(0.93 0.02 300)",
      mutedDark: "oklch(0.28 0.03 300)",
    },
  },
  {
    name: "Cyan",
    value: "cyan",
    palette: {
      primary: "oklch(0.8 0.13 200)",
      background: "oklch(0.98 0.02 200)",
      backgroundDark: "oklch(0.22 0.09 200)",
      foreground: "oklch(0.18 0.01 200)",
      foregroundDark: "oklch(0.98 0.01 200)",
      border: "oklch(0.85 0.04 200)",
      borderDark: "oklch(0.3 0.06 200)",
      accent: "oklch(0.7 0.09 200)",
      accentDark: "oklch(0.35 0.12 200)",
      muted: "oklch(0.93 0.02 200)",
      mutedDark: "oklch(0.28 0.03 200)",
    },
  },
];

// The key used to store tone in localStorage as a backup
export const THEME_TONE_KEY = "theme-tone-preference";

/**
 * Function to apply theme tone variables to the document root
 * Now exported so it can be used by ThemeInitializer
 */
export function setThemeToneVars(
  palette: Record<string, string>,
  isDark: boolean
) {
  if (typeof window === "undefined") return;

  const root = document.documentElement;

  // Set primary color (same for light/dark)
  root.style.setProperty("--primary", palette.primary);

  // Set background based on mode
  root.style.setProperty(
    "--background",
    isDark ? palette.backgroundDark : palette.background
  );

  // Set foreground based on mode
  root.style.setProperty(
    "--foreground",
    isDark ? palette.foregroundDark : palette.foreground
  );

  // Set border based on mode
  root.style.setProperty(
    "--border",
    isDark ? palette.borderDark : palette.border
  );

  // Set accent based on mode
  root.style.setProperty(
    "--accent",
    isDark ? palette.accentDark : palette.accent
  );

  // Set muted based on mode
  root.style.setProperty("--muted", isDark ? palette.mutedDark : palette.muted);
}

/**
 * Save theme preferences both to Tauri config.json and localStorage
 */
async function saveThemePreferences(tone: string, isDark: boolean) {
  try {
    // Save to localStorage as fallback
    if (typeof window !== "undefined") {
      localStorage.setItem(THEME_TONE_KEY, tone);
    }

    // Save to Tauri config
    interface ThemeConfig {
      league_path?: string;
      skins?: unknown[];
      favorites?: unknown[];
      theme?: {
        tone?: string;
        isDark?: boolean;
      };
    }
    const config: ThemeConfig = (await invoke("load_config").catch(
      () => ({})
    )) as ThemeConfig;

    const updatedConfig = {
      ...config,
      league_path: config.league_path ?? "",
      skins: config.skins ?? [],
      favorites: config.favorites ?? [],
      theme: {
        tone,
        isDark,
      },
    };

    await invoke("save_selected_skins", {
      leaguePath: updatedConfig.league_path,
      skins: updatedConfig.skins,
      favorites: updatedConfig.favorites,
      theme: updatedConfig.theme,
    }).catch((err: unknown) => {
      console.error("Failed to save theme to config:", err);
    });

    console.log(`Theme preferences saved: tone=${tone}, isDark=${isDark}`);
  } catch (error) {
    console.error("Failed to save theme preferences:", error);
  }
}

/**
 * Custom hook for theme tone management
 */
export function useThemeTone() {
  // Default to blue tone
  const [tone, setToneState] = useState<string>("gray");
  const [initialized, setInitialized] = useState(false);
  const [isTransitioning, setIsTransitioning] = useState(false);
  const { theme, setTheme, resolvedTheme } = useTheme();

  // Derive isDark from theme or resolvedTheme
  const isDark = theme === "dark" || resolvedTheme === "dark";

  // Load theme tone preference on initial render
  useEffect(() => {
    const loadThemePreferences = async () => {
      try {
        // First, try to load from Tauri config
        interface ThemeConfig {
          league_path?: string;
          skins?: unknown[];
          favorites?: unknown[];
          theme?: {
            tone?: string;
            isDark?: boolean;
          };
        }
        const config = (await invoke("load_config").catch(
          () => null
        )) as ThemeConfig | null;
        let savedTone = config?.theme?.tone;

        // Fallback to localStorage if not found in Tauri config
        if (!savedTone && typeof window !== "undefined") {
          const storedTone = localStorage.getItem(THEME_TONE_KEY);
          savedTone = storedTone ?? undefined;
        }

        // Apply saved tone if found
        if (savedTone) {
          setToneState(savedTone);
        }

        setInitialized(true);
      } catch (error) {
        console.error("Failed to load theme preferences:", error);
        setInitialized(true);
      }
    };

    void loadThemePreferences();
  }, []);

  // Custom setter for tone that also saves the preference
  const setTone = useCallback(
    (newTone: string) => {
      setToneState(newTone);
      void saveThemePreferences(newTone, isDark);
    },
    [isDark]
  );

  // Improved theme toggler with transition class
  const toggleTheme = useCallback(
    (newIsDark: boolean) => {
      if (isTransitioning) return;

      // Add transitioning class to prevent flickering
      if (typeof document !== "undefined") {
        document.documentElement.classList.add("transitioning-theme");
      }

      // Set transitioning state
      setIsTransitioning(true);

      // Change theme
      setTheme(newIsDark ? "dark" : "light");

      // Save preferences
      void saveThemePreferences(tone, newIsDark);

      // Remove transitioning class after the theme change has completed
      const transitionDuration = 250; // slightly longer than CSS transition
      setTimeout(() => {
        if (typeof document !== "undefined") {
          document.documentElement.classList.remove("transitioning-theme");
        }
        setIsTransitioning(false);
      }, transitionDuration);
    },
    [tone, setTheme, isTransitioning]
  );

  // Apply theme variables when tone or dark mode changes
  useEffect(() => {
    if (!initialized) return;

    // Don't make CSS changes during transitions
    if (isTransitioning) return;

    // Get the selected tone palette
    const selected = TONES.find((t) => t.value === tone) ?? TONES[1];

    // Apply CSS variables with a small delay to ensure DOM is ready
    const applyVars = () => {
      setThemeToneVars(selected.palette, isDark);
    };

    // Use requestAnimationFrame for better timing with browser paint cycle
    if (typeof window !== "undefined") {
      window.requestAnimationFrame(applyVars);
    } else {
      applyVars();
    }
  }, [tone, isDark, initialized, isTransitioning]);

  // Provide a clean interface for the component
  return {
    tone,
    setTone,
    isDark,
    toggleTheme,
    initialized,
    isTransitioning,
  };
}

export function ThemeToneSelector() {
  const { tone, setTone, isDark, toggleTheme, isTransitioning } =
    useThemeToneContext();

  return (
    <>
      <div className="flex items-center justify-between">
        <Label>Theme</Label>
        <div className="flex items-center gap-2">
          <SunIcon size={14} className={isDark ? "opacity-40" : ""} />
          <Switch
            checked={isDark}
            onCheckedChange={toggleTheme}
            disabled={isTransitioning}
          />
          <MoonIcon size={14} className={!isDark ? "opacity-40" : ""} />
        </div>
      </div>

      <div className="grid grid-cols-3 gap-2">
        {TONES.map((t) => (
          <button
            key={t.value}
            className={cn(
              "relative h-8 rounded-md transition-all flex items-center justify-center",
              tone === t.value
                ? "ring-2 ring-primary ring-offset-2 ring-offset-background"
                : "ring-1 ring-border hover:ring-2"
            )}
            onClick={() => {
              if (!isTransitioning) {
                setTone(t.value);
                toast.success(`Theme changed to ${t.name}`);
              }
            }}
            disabled={isTransitioning}
            style={{
              background: t.palette.primary,
              opacity: isTransitioning ? 0.7 : 1,
              cursor: isTransitioning ? "not-allowed" : "pointer",
            }}
            title={t.name}
          />
        ))}
      </div>
    </>
  );
}
