import { createContext, useContext, useMemo } from "react";
import type { ReactNode } from "react";
import { t as translate, tp as translatePlural } from "./index";
import type { Lang, MessageKey } from "./index";

export interface I18nValue {
  lang: Lang;
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
  tp: (
    key: MessageKey,
    count: number,
    params?: Record<string, string | number>,
  ) => string;
}

const LanguageContext = createContext<I18nValue | null>(null);

export function LanguageProvider({
  lang,
  children,
}: {
  lang: Lang;
  children: ReactNode;
}) {
  const value = useMemo<I18nValue>(
    () => ({
      lang,
      t: (key, params) => translate(lang, key, params),
      tp: (key, count, params) => translatePlural(lang, key, count, params),
    }),
    [lang],
  );
  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useI18n(): I18nValue {
  const ctx = useContext(LanguageContext);
  if (!ctx) {
    throw new Error("useI18n() must be used inside LanguageProvider");
  }
  return ctx;
}