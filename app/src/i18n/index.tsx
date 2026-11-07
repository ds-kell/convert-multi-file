// Minimal internationalisation: typed dictionaries, `{placeholder}`
// interpolation and English plurals. No library needed for two languages.

import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { en, type MessageKey, type Messages } from "./en";
import { vi } from "./vi";

/** Supported UI languages. */
export type Language = "en" | "vi";

/** Languages with their own (untranslated) names, for the language picker. */
export const LANGUAGES: { value: Language; label: string }[] = [
  { value: "en", label: "English" },
  { value: "vi", label: "Tiếng Việt" },
];

const DICTIONARIES: Record<Language, Messages> = { en, vi };

/** localStorage key remembering the chosen language. */
const STORAGE_KEY = "ui.language";

/** Values substituted into `{placeholders}`. */
export type Params = Record<string, string | number>;

/** Base keys that have `.one` / `.other` plural forms. */
type PluralKey = MessageKey extends infer K
  ? K extends `${infer Base}.one`
    ? Base
    : never
  : never;

interface I18n {
  language: Language;
  setLanguage: (language: Language) => void;
  /** Translates `key`, replacing `{name}` placeholders with `params`. */
  t: (key: MessageKey, params?: Params) => string;
  /** Plural-aware translation: picks `<key>.one` or `<key>.other` for `count`
   *  and passes `count` as a parameter. */
  tn: (key: PluralKey, count: number, params?: Params) => string;
}

const I18nContext = createContext<I18n | null>(null);

/** Saved choice, else the system language (Vietnamese → vi, others → en). */
function initialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "vi") return saved;
  } catch {
    // Storage unavailable: fall back to the system language.
  }
  const preferred = navigator.languages?.length ? navigator.languages : [navigator.language];
  return preferred.some((tag) => tag.toLowerCase().startsWith("vi")) ? "vi" : "en";
}

function interpolate(text: string, params?: Params): string {
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

/** Provides the current language and translation functions to the app. */
export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(initialLanguage);

  const setLanguage = useCallback((next: Language) => {
    setLanguageState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Not critical: the choice is just not remembered.
    }
  }, []);

  // Screen readers and spell-checkers use the document language.
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const value = useMemo<I18n>(() => {
    const messages = DICTIONARIES[language];
    const plurals = new Intl.PluralRules(language);
    const t = (key: MessageKey, params?: Params) => interpolate(messages[key], params);
    const tn = (key: PluralKey, count: number, params?: Params) => {
      const form = plurals.select(count) === "one" ? "one" : "other";
      return t(`${key}.${form}` as MessageKey, { count, ...params });
    };
    return { language, setLanguage, t, tn };
  }, [language, setLanguage]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

/** Access translations; must be used inside `I18nProvider`. */
export function useI18n(): I18n {
  const i18n = useContext(I18nContext);
  if (i18n === null) throw new Error("useI18n must be used inside <I18nProvider>");
  return i18n;
}
