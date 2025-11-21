"use client";

import React, { createContext, useContext, useEffect, useState } from "react";

export type Locale =
  | "en"
  | "zh"
  | "ko"
  | "pt-BR"
  | "es"
  | "ru"
  | "tr"
  | "de"
  | "fr"
  | "vi"
  | "ar";

const LOCALE_KEY = "app-locale";

const defaultLocale: Locale = "en";

// Static map of locale data to avoid dynamic import expressions which break Turbopack
import en from "../locales/en.json";
import zh from "../locales/zh.json";
import ko from "../locales/ko.json";
import ptBR from "../locales/pt-BR.json";
import es from "../locales/es.json";
import ru from "../locales/ru.json";
import tr from "../locales/tr.json";
import de from "../locales/de.json";
import fr from "../locales/fr.json";
import vi from "../locales/vi.json";
import ar from "../locales/ar.json";

const LOCALE_MAP: Record<Locale, Record<string, string>> = {
  en,
  zh,
  ko,
  "pt-BR": ptBR,
  es,
  ru,
  tr,
  de,
  fr,
  vi,
  ar,
};

async function loadLocale(locale: Locale): Promise<Record<string, string>> {
  return LOCALE_MAP[locale];
}

interface I18nContextValue {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(defaultLocale);
  const [dict, setDict] = useState<Record<string, string>>({});

  useEffect(() => {
    const saved =
      typeof window !== "undefined"
        ? localStorage.getItem(LOCALE_KEY)
        : undefined;
    const startLocale = saved ? (saved as unknown as Locale) : defaultLocale;
    setLocaleState(startLocale);
    loadLocale(startLocale)
      .then((d) => {
        setDict(d);
      })
      .catch(() => {
        setDict({});
      });
  }, []);

  const setLocale = (l: Locale) => {
    setLocaleState(l);
    if (typeof window !== "undefined") {
      localStorage.setItem(LOCALE_KEY, l);
    }
    loadLocale(l)
      .then((d) => {
        setDict(d);
      })
      .catch(() => {
        setDict({});
      });
  };

  const t = (key: string, vars?: Record<string, string | number>) => {
    // Prefer current locale dictionary, then fall back to English, then the key itself
    const current = (dict as Record<string, string | undefined>)[key];
    const enFallback = (LOCALE_MAP.en as Record<string, string | undefined>)[
      key
    ];
    const raw = current ?? enFallback ?? key;
    if (!vars) return raw;
    let result = raw;
    for (const k of Object.keys(vars)) {
      const re = new RegExp(`\\{${k}\\}`, "g");
      result = result.replace(re, String(vars[k]));
    }
    return result;
  };

  return (
    <I18nContext.Provider value={{ locale, setLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside I18nProvider");
  return ctx;
}
