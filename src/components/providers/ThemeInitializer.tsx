"use client";

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "next-themes";

// Import the theme tones and the setThemeToneVars function
import { TONES, setThemeToneVars } from "../ThemeToneSelector";

// Local storage fallback key
const THEME_TONE_KEY = "theme-tone-preference";

/**
 * Component that initializes theme preferences on app startup
 * This ensures theme is applied immediately rather than waiting for dropdown to open
 */
export function ThemeInitializer() {
  const { theme, setTheme, resolvedTheme } = useTheme();
  const [isInitializing, setIsInitializing] = useState(true);

  // Initialize theme on component mount
  // Define the expected config type
  interface ThemeConfig {
    theme?: {
      isDark?: boolean;
      tone?: string;
    };
  }

  useEffect(() => {
    // Prevent theme flicker by adding transitioning class immediately
    if (typeof document !== "undefined") {
      document.documentElement.classList.add("transitioning-theme");
    }

    const initializeTheme = async () => {
      try {
        // Try to load from Tauri config first
        const config = await invoke<ThemeConfig>("load_config").catch(
          () => null
        );
        let savedTone = config?.theme?.tone;

        if (!savedTone && typeof window !== "undefined") {
          savedTone = localStorage.getItem(THEME_TONE_KEY) ?? undefined;
        }

        // Apply saved theme mode
        if (config?.theme?.isDark !== undefined) {
          setTheme(config.theme.isDark ? "dark" : "light");
        }

        // If we have a saved tone, apply it immediately
        if (savedTone) {
          const toneObj = TONES.find((t) => t.value === savedTone);
          if (toneObj) {
            // Wait for theme to be resolved before applying variables
            const isDark = theme === "dark" || resolvedTheme === "dark";

            // Apply the tone using requestAnimationFrame for better timing
            if (typeof window !== "undefined") {
              window.requestAnimationFrame(() => {
                setThemeToneVars(toneObj.palette, isDark);
              });
            } else {
              setThemeToneVars(toneObj.palette, isDark);
            }

            // Store tone in localStorage for backup
            if (typeof window !== "undefined") {
              localStorage.setItem(THEME_TONE_KEY, savedTone);
            }
          }
        }
      } catch (error) {
        console.error("Failed to initialize theme preferences:", error);
      } finally {
        // Remove transitioning class after initialization is complete
        setTimeout(() => {
          if (typeof document !== "undefined") {
            document.documentElement.classList.remove("transitioning-theme");
          }
          setIsInitializing(false);
        }, 300);
      }
    };

    void initializeTheme();
  }, [theme, resolvedTheme, setTheme]);

  return null; // This component doesn't render anything
}
