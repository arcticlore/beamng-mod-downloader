import { ru } from "./ru.ts";
import { en } from "./en.ts";

export type Lang = "ru" | "en";

export type MessageDict = {
  [K in keyof typeof ru]: string | readonly string[];
};
export type MessageKey = keyof typeof ru;
export type MessageValue = string | readonly string[];

const DICTS: Record<Lang, MessageDict> = { ru, en };

function dictOf(lang: Lang): Record<MessageKey, MessageValue> {
  return DICTS[lang] as unknown as Record<MessageKey, MessageValue>;
}

/** Индекс формы множественного числа: RU — 0/1/2 (1, 2–4, 5+), EN — 0/1. */
export function pluralIndex(lang: Lang, n: number): number {
  if (lang === "en") return n === 1 ? 0 : 1;
  const mod10 = n % 10;
  const mod100 = n % 100;
  if (mod10 === 1 && mod100 !== 11) return 0;
  if (mod10 >= 2 && mod10 <= 4 && !(mod100 >= 12 && mod100 <= 14)) return 1;
  return 2;
}

function render(
  lang: Lang,
  key: MessageKey,
  opts: { count?: number; params?: Record<string, string | number> } = {},
): string {
  const value = dictOf(lang)[key];
  const array = Array.isArray(value);
  const forms = array ? value : null;
  const template =
    forms != null ? forms[Math.min(pluralIndex(lang, opts.count ?? 2), forms.length - 1)] : value;
  if (template === undefined) return String(key);
  let out = template;
  if (opts.count !== undefined) out = out.replaceAll("{n}", String(opts.count));
  if (opts.params) {
    for (const [k, v] of Object.entries(opts.params)) {
      out = out.replaceAll(`{${k}}`, String(v));
    }
  }
  return out;
}

/** Простая строка (не множественное число). */
export function t(
  lang: Lang,
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  const value = dictOf(lang)[key];
  if (Array.isArray(value)) {
    throw new Error(`i18n: key "${String(key)}" is plural — use tp() with a count`);
  }
  return render(lang, key, { params });
}

/** Множественное число по количеству: формы выбираются языком и `count`. */
export function tp(
  lang: Lang,
  key: MessageKey,
  count: number,
  params?: Record<string, string | number>,
): string {
  return render(lang, key, { count, params });
}

/** Нормализация строки языка из настроек в `Lang`. */
export function normalizeLang(value: string | null | undefined): Lang {
  return String(value ?? "ru").toLowerCase().startsWith("en") ? "en" : "ru";
}