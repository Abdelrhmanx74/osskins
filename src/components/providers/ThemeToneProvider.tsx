"use client";

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
} from "react";
import { useTheme } from "next-themes";
import { invoke } from "@tauri-apps/api/core";
import { TONES, setThemeToneVars, THEME_TONE_KEY } from "../ThemeToneSelector";

interface ThemeToneContextProps {
  tone: string;
  setTone: (tone: string) => void;
  isDark: boolean;
  toggleTheme: (isDark: boolean) => void;
  initialized: boolean;
  isTransitioning: boolean;
}

const ThemeToneContext = createContext<ThemeToneContextProps | undefined>(
  undefined
);

export const ThemeToneProvider = ({
  children,
}: {
  children: React.ReactNode;
}) => {
  const [tone, setToneState] = useState<string>("gray");
  const [initialized, setInitialized] = useState(false);
  const [isTransitioning, setIsTransitioning] = useState(false);
  const { theme, setTheme, resolvedTheme } = useTheme();

  const isDark = theme === "dark" || resolvedTheme === "dark";

  useEffect(() => {
    const loadThemePreferences = async () => {
      try {
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
        if (!savedTone && typeof window !== "undefined") {
          const storedTone = localStorage.getItem(THEME_TONE_KEY);
          savedTone = storedTone ?? undefined;
        }
        if (savedTone) {
          setToneState(savedTone);
        }
        setInitialized(true);
      } catch (error) {
        setInitialized(true);
      }
    };
    void loadThemePreferences();
  }, []);

  const setTone = useCallback(
    (newTone: string) => {
      setToneState(newTone);
      void saveThemePreferences(newTone, isDark);
    },
    [isDark]
  );

  const saveThemePreferences = async (tone: string, isDark: boolean) => {
    try {
      if (typeof window !== "undefined") {
        localStorage.setItem(THEME_TONE_KEY, tone);
      }
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
      }).catch(() => {});
    } catch {}
  };

  const toggleTheme = useCallback(
    (newIsDark: boolean) => {
      if (isTransitioning) return;
      if (typeof document !== "undefined") {
        document.documentElement.classList.add("transitioning-theme");
      }
      setIsTransitioning(true);
      setTheme(newIsDark ? "dark" : "light");
      void saveThemePreferences(tone, newIsDark);
      const transitionDuration = 250;
      setTimeout(() => {
        if (typeof document !== "undefined") {
          document.documentElement.classList.remove("transitioning-theme");
        }
        setIsTransitioning(false);
      }, transitionDuration);
    },
    [tone, setTheme, isTransitioning]
  );

  useEffect(() => {
    if (!initialized) return;
    if (isTransitioning) return;
    const selected = TONES.find((t) => t.value === tone) ?? TONES[1];
    const applyVars = () => {
      setThemeToneVars(selected.palette, isDark);
    };
    if (typeof window !== "undefined") {
      window.requestAnimationFrame(applyVars);
    } else {
      applyVars();
    }
  }, [tone, isDark, initialized, isTransitioning]);

  return (
    <ThemeToneContext.Provider
      value={{
        tone,
        setTone,
        isDark,
        toggleTheme,
        initialized,
        isTransitioning,
      }}
    >
      {children}
    </ThemeToneContext.Provider>
  );
};

export function useThemeToneContext() {
  const ctx = useContext(ThemeToneContext);
  if (!ctx)
    throw new Error(
      "useThemeToneContext must be used within ThemeToneProvider"
    );
  return ctx;
}
