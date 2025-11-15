"use client";

import { useEffect, useState, useRef, useCallback } from "react";
import { useTheme } from "next-themes";
import { cn } from "@/lib/utils";
import { SunIcon, MoonIcon } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";

// Define theme tone options with palettes for both light and dark modes
// Now exported so it can be used by ThemeInitializer
export const TONES = [
  // Special / dark
  {
    name: "Void",
    value: "void",
    palette: {
      // deep, high-contrast dark theme with neon highlights
      primary: "oklch(0.60 0.28 260)",
      background: "oklch(0.96 0.05 260)",
      backgroundDark: "oklch(0.06 0.02 260)",
      foreground: "oklch(0.16 0.02 260)",
      foregroundDark: "oklch(0.98 0.02 260)",
      border: "oklch(0.88 0.06 260)",
      borderDark: "oklch(0.20 0.03 260)",
      accent: "oklch(0.86 0.32 320)",
      accentDark: "oklch(0.44 0.18 320)",
      muted: "oklch(0.95 0.03 260)",
      mutedDark: "oklch(0.22 0.04 260)",
    },
  },

  // Cool/blue group
  {
    name: "Ice",
    value: "ice",
    palette: {
      primary: "oklch(0.82 0.12 220)",
      background: "oklch(0.99 0.03 220)",
      backgroundDark: "oklch(0.16 0.06 220)",
      foreground: "oklch(0.12 0.02 220)",
      foregroundDark: "oklch(0.98 0.02 220)",
      border: "oklch(0.90 0.04 220)",
      borderDark: "oklch(0.30 0.06 220)",
      accent: "oklch(0.80 0.16 220)",
      accentDark: "oklch(0.38 0.10 220)",
      muted: "oklch(0.98 0.02 220)",
      mutedDark: "oklch(0.28 0.04 220)",
    },
  },
  {
    name: "Sky",
    value: "sky",
    palette: {
      primary: "oklch(0.80 0.14 200)",
      background: "oklch(0.99 0.05 200)",
      backgroundDark: "oklch(0.18 0.07 200)",
      foreground: "oklch(0.14 0.02 200)",
      foregroundDark: "oklch(0.97 0.02 200)",
      border: "oklch(0.90 0.06 200)",
      borderDark: "oklch(0.34 0.06 200)",
      accent: "oklch(0.82 0.18 200)",
      accentDark: "oklch(0.40 0.12 200)",
      muted: "oklch(0.98 0.04 200)",
      mutedDark: "oklch(0.30 0.05 200)",
    },
  },
  {
    name: "Slate",
    value: "slate",
    palette: {
      // cool, desaturated slate-blue with subtle teal accent
      primary: "oklch(0.62 0.10 210)",
      background: "oklch(0.99 0.03 210)",
      backgroundDark: "oklch(0.10 0.08 210)",
      foreground: "oklch(0.12 0.02 210)",
      foregroundDark: "oklch(0.98 0.02 210)",
      border: "oklch(0.90 0.05 210)",
      borderDark: "oklch(0.32 0.06 210)",
      accent: "oklch(0.80 0.18 185)",
      accentDark: "oklch(0.42 0.14 185)",
      muted: "oklch(0.97 0.03 210)",
      mutedDark: "oklch(0.26 0.05 210)",
    },
  },

  // Teal/green transition
  {
    name: "Teal",
    value: "teal",
    palette: {
      primary: "oklch(0.70 0.15 185)",
      background: "oklch(0.98 0.03 185)",
      backgroundDark: "oklch(0.18 0.08 185)",
      foreground: "oklch(0.14 0.02 185)",
      foregroundDark: "oklch(0.96 0.02 185)",
      border: "oklch(0.86 0.06 185)",
      borderDark: "oklch(0.32 0.06 185)",
      accent: "oklch(0.78 0.20 185)",
      accentDark: "oklch(0.36 0.12 185)",
      muted: "oklch(0.95 0.03 185)",
      mutedDark: "oklch(0.28 0.04 185)",
    },
  },
  {
    name: "Mint",
    value: "mint",
    palette: {
      primary: "oklch(0.78 0.18 150)",
      background: "oklch(0.98 0.04 150)",
      backgroundDark: "oklch(0.16 0.08 150)",
      foreground: "oklch(0.12 0.02 150)",
      foregroundDark: "oklch(0.97 0.02 150)",
      border: "oklch(0.88 0.06 150)",
      borderDark: "oklch(0.30 0.06 150)",
      accent: "oklch(0.80 0.20 150)",
      accentDark: "oklch(0.38 0.12 150)",
      muted: "oklch(0.96 0.03 150)",
      mutedDark: "oklch(0.26 0.04 150)",
    },
  },
  {
    name: "Verdure",
    value: "verdure",
    palette: {
      // deep, forest green with warm amber accent
      primary: "oklch(0.56 0.30 140)",
      background: "oklch(0.98 0.04 140)",
      backgroundDark: "oklch(0.08 0.10 140)",
      foreground: "oklch(0.14 0.02 140)",
      foregroundDark: "oklch(0.96 0.02 140)",
      border: "oklch(0.86 0.06 140)",
      borderDark: "oklch(0.30 0.06 140)",
      accent: "oklch(0.88 0.26 48)",
      accentDark: "oklch(0.44 0.18 48)",
      muted: "oklch(0.96 0.04 140)",
      mutedDark: "oklch(0.22 0.05 140)",
    },
  },

  // Purple / Neon group
  {
    name: "Neon",
    value: "neon",
    palette: {
      primary: "oklch(0.78 0.28 320)",
      background: "oklch(0.99 0.08 320)",
      backgroundDark: "oklch(0.12 0.12 320)",
      foreground: "oklch(0.10 0.02 320)",
      foregroundDark: "oklch(0.98 0.02 320)",
      border: "oklch(0.90 0.10 320)",
      borderDark: "oklch(0.36 0.12 320)",
      accent: "oklch(0.88 0.34 320)",
      accentDark: "oklch(0.46 0.18 320)",
      muted: "oklch(0.98 0.06 320)",
      mutedDark: "oklch(0.30 0.06 320)",
    },
  },
  {
    name: "Lavender",
    value: "lavender",
    palette: {
      primary: "oklch(0.74 0.18 270)",
      background: "oklch(0.98 0.04 270)",
      backgroundDark: "oklch(0.18 0.08 270)",
      foreground: "oklch(0.12 0.02 270)",
      foregroundDark: "oklch(0.96 0.02 270)",
      border: "oklch(0.88 0.06 270)",
      borderDark: "oklch(0.34 0.06 270)",
      accent: "oklch(0.82 0.22 270)",
      accentDark: "oklch(0.40 0.14 270)",
      muted: "oklch(0.96 0.04 270)",
      mutedDark: "oklch(0.30 0.05 270)",
    },
  },
  {
    name: "Aurora",
    value: "aurora",
    palette: {
      // vibrant, shifting-chroma look: magenta primary with cool teal accents
      primary: "oklch(0.78 0.30 300)",
      background: "oklch(0.98 0.06 260)",
      backgroundDark: "oklch(0.10 0.14 220)",
      foreground: "oklch(0.10 0.02 300)",
      foregroundDark: "oklch(0.98 0.02 300)",
      border: "oklch(0.90 0.08 280)",
      borderDark: "oklch(0.32 0.08 220)",
      accent: "oklch(0.82 0.34 170)",
      accentDark: "oklch(0.46 0.18 170)",
      muted: "oklch(0.97 0.06 260)",
      mutedDark: "oklch(0.26 0.06 220)",
    },
  },

  // Warm / yellow then warm red
  {
    name: "Lime",
    value: "lime",
    palette: {
      // tuned to a warm, vivid summer yellow
      primary: "oklch(0.92 0.30 95)",
      background: "oklch(0.995 0.06 95)",
      backgroundDark: "oklch(0.34 0.12 95)",
      foreground: "oklch(0.12 0.02 95)",
      foregroundDark: "oklch(0.98 0.02 95)",
      border: "oklch(0.94 0.10 95)",
      borderDark: "oklch(0.40 0.10 95)",
      accent: "oklch(0.92 0.36 95)",
      accentDark: "oklch(0.48 0.20 95)",
      muted: "oklch(0.99 0.05 95)",
      mutedDark: "oklch(0.36 0.06 95)",
    },
  },
  {
    name: "Sunset",
    value: "sunset",
    palette: {
      primary: "oklch(0.74 0.22 28)",
      background: "oklch(0.99 0.06 28)",
      backgroundDark: "oklch(0.18 0.10 28)",
      foreground: "oklch(0.12 0.02 28)",
      foregroundDark: "oklch(0.98 0.02 28)",
      border: "oklch(0.90 0.08 28)",
      borderDark: "oklch(0.36 0.08 28)",
      accent: "oklch(0.86 0.26 28)",
      accentDark: "oklch(0.40 0.14 28)",
      muted: "oklch(0.97 0.05 28)",
      mutedDark: "oklch(0.32 0.06 28)",
    },
  },
  // New themes: Mono, Sepia, Pastel — distinct palettes
  {
    name: "Monochrome",
    value: "mono",
    palette: {
      primary: "oklch(0.54 0.02 0)",
      background: "oklch(0.98 0.02 0)",
      backgroundDark: "oklch(0.06 0.02 0)",
      foreground: "oklch(0.14 0.02 0)",
      foregroundDark: "oklch(0.98 0.02 0)",
      border: "oklch(0.88 0.02 0)",
      borderDark: "oklch(0.20 0.02 0)",
      accent: "oklch(0.98 0.02 0)",
      accentDark: "oklch(0.12 0.02 0)",
      muted: "oklch(0.99 0.01 0)",
      mutedDark: "oklch(0.24 0.02 0)",
    },
  },
  {
    name: "Sepia",
    value: "sepia",
    palette: {
      primary: "oklch(0.62 0.18 40)",
      background: "oklch(0.98 0.04 40)",
      backgroundDark: "oklch(0.08 0.10 40)",
      foreground: "oklch(0.14 0.02 40)",
      foregroundDark: "oklch(0.96 0.02 40)",
      border: "oklch(0.86 0.06 40)",
      borderDark: "oklch(0.30 0.06 40)",
      accent: "oklch(0.78 0.28 30)",
      accentDark: "oklch(0.44 0.18 30)",
      muted: "oklch(0.95 0.04 40)",
      mutedDark: "oklch(0.26 0.05 40)",
    },
  },
  {
    name: "Pastel",
    value: "pastel",
    palette: {
      primary: "oklch(0.94 0.10 300)",
      background: "oklch(0.995 0.06 300)",
      backgroundDark: "oklch(0.30 0.05 300)",
      foreground: "oklch(0.14 0.02 300)",
      foregroundDark: "oklch(0.98 0.02 300)",
      border: "oklch(0.94 0.08 300)",
      borderDark: "oklch(0.36 0.08 300)",
      accent: "oklch(0.86 0.18 160)",
      accentDark: "oklch(0.46 0.18 160)",
      muted: "oklch(0.99 0.05 300)",
      mutedDark: "oklch(0.34 0.06 300)",
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
  // Determine a matching accent. Some palettes may provide an accent
  // that visually clashes with their primary (different hue). Parse
  // OKLCH strings and fall back to primary when the hue delta is large.
  function parseOklch(s: string | undefined) {
    if (!s) return null;
    const m = /oklch\(\s*([0-9.]+)\s+([0-9.]+)\s+([0-9.]+)\s*\)/i.exec(s);
    if (!m) return null;
    return { L: parseFloat(m[1]), C: parseFloat(m[2]), H: parseFloat(m[3]) };
  }

  const primaryParsed = parseOklch(palette.primary);
  const accentParsed = parseOklch(isDark ? palette.accentDark : palette.accent);

  let accentToUse = isDark ? palette.accentDark : palette.accent;

  if (primaryParsed && accentParsed) {
    const h1 = primaryParsed.H % 360;
    const h2 = accentParsed.H % 360;
    let diff = Math.abs(h1 - h2);
    if (diff > 180) diff = 360 - diff;

    // If hue difference is large, use primary as the accent so bg-accent
    // matches the tone (avoids unexpected green highlights on purples).
    if (diff > 45) {
      accentToUse = palette.primary;
    }
  }

  root.style.setProperty("--accent", accentToUse);

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
      selected_misc_items?: Record<string, string[]>;
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
      selectedMiscItems: config.selected_misc_items ?? {},
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
  // Default to 'void' special dark tone instead of shadcn default
  const [tone, setToneState] = useState<string>("slate");
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

    // Get the selected tone palette (prefer 'void' when missing)
    const selected =
      TONES.find((t) => t.value === tone) ??
      TONES.find((t) => t.value === "void") ??
      TONES[0];

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

/**
 * Theme tone selector component
 */
export function ThemeToneSelector() {
  const { tone, setTone, isDark, toggleTheme, isTransitioning } =
    useThemeTone();
  const { t } = useI18n();

  return (
    <>
      <div className="px-2 py-2">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium">{t("theme.label")}</span>
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

        <div className="grid grid-cols-3 gap-2 mt-3">
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
      </div>
    </>
  );
}
