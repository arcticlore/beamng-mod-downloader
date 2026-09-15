import type { AppSettings } from "./types";

export const ACCENT_PRESETS = [
  "#4f8cff",
  "#35c46a",
  "#ffb648",
  "#ff5c6c",
  "#b06cff",
  "#22c1c8",
  "#e25cff",
];

const DEFAULT_ACCENT = "#4f8cff";

/** Применяет тему и акцентный цвет к CSS-переменным на `<html>`. */
export function applyAppearance(settings: AppSettings | null | undefined): void {
  const theme = settings?.theme ?? "dark";
  document.documentElement.dataset.theme = theme;
  const accent = normalizeHex(settings?.accent) ?? DEFAULT_ACCENT;
  document.documentElement.style.setProperty("--accent", accent);
  document.documentElement.style.setProperty("--accent-dark", darken(accent, 0.18));
}

function normalizeHex(value: string | undefined | null): string | null {
  if (!value) return null;
  const v = value.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(v)) return v.toLowerCase();
  if (/^#[0-9a-fA-F]{3}$/.test(v)) {
    return `#${v
      .slice(1)
      .split("")
      .map((c) => c + c)
      .join("")
      .toLowerCase()}`;
  }
  return null;
}

function darken(hex: string, amount: number): string {
  const n = parseInt(hex.slice(1), 16);
  const darkenChannel = (shift: number) =>
    Math.max(0, Math.min(255, Math.round(((n >> shift) & 255) * (1 - amount))));
  const r = darkenChannel(16);
  const g = darkenChannel(8);
  const b = darkenChannel(0);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}