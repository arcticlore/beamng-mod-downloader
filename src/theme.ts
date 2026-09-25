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
const DEFAULT_STYLE = "material";

/** Относительная насыщенность тона в Material 3 (baseline tonal saturation). */
const TONE_SAT_FACTOR: Record<number, number> = {
  0: 0.0,
  5: 0.55,
  10: 0.72,
  15: 0.82,
  20: 0.9,
  25: 0.95,
  30: 1.0,
  35: 1.0,
  40: 1.0,
  45: 1.0,
  50: 1.0,
  55: 1.0,
  60: 1.0,
  65: 0.97,
  70: 0.9,
  75: 0.78,
  80: 0.62,
  85: 0.42,
  90: 0.2,
  95: 0.09,
  99: 0.05,
  100: 0.0,
};

/**
 * Применяет стиль (Material 3 / Classic), тему и акцентный цвет к `<html>`:
 * - `data-style` переключает набор CSS-правил (`material.css` vs `styles.css`);
 * - для Material 3 на `<html>` вычисляется акцентная tonal-палитра Material 3 и
 *   раскрывается в роли `--md-sys-color-*` (цветные + нейтральные + secondary/
 *   tertiary/error); классический стиль палитру не использует.
 */
export function applyAppearance(settings: AppSettings | null | undefined): void {
  const theme = settings?.theme ?? "dark";
  const style = settings?.style ?? DEFAULT_STYLE;
  const accent = normalizeHex(settings?.accent) ?? DEFAULT_ACCENT;

  const el = document.documentElement;
  el.dataset.theme = theme;
  el.dataset.style = style;
  el.style.setProperty("--accent", accent);
  el.style.setProperty("--accent-dark", darken(accent, 0.18));

  if (style === "material") {
    applyMaterialTokens(theme, accent);
  }
}

function applyMaterialTokens(theme: string, accent: string): void {
  const dark = theme === "dark";
  const { h, s } = hexToHsl(accent);
  // Нейтральная палитра почти без хроматики, но близка к оттенку акцента.
  const hueShift = Math.round(h);

  const P = toneRamp(h, s);
  const N = toneRamp(hueShift, Math.min(0.14, s * 0.22));
  const S = toneRamp((h + 34) % 360, s * 0.8);
  const T = toneRamp((h + 360 - 34) % 360, s * 0.8);
  const E = toneRamp(25, 0.82);

  const get = (ramp: Record<number, string>, tone: number): string => ramp[tone];

  const tokens: Record<string, string> = dark
    ? {
        "md-sys-color-primary": get(P, 80),
        "md-sys-color-on-primary": get(P, 20),
        "md-sys-color-primary-container": get(P, 30),
        "md-sys-color-on-primary-container": get(P, 90),
        "md-sys-color-secondary": get(S, 80),
        "md-sys-color-on-secondary": get(S, 20),
        "md-sys-color-secondary-container": get(S, 30),
        "md-sys-color-on-secondary-container": get(S, 90),
        "md-sys-color-tertiary": get(T, 80),
        "md-sys-color-on-tertiary": get(T, 20),
        "md-sys-color-tertiary-container": get(T, 30),
        "md-sys-color-on-tertiary-container": get(T, 90),
        "md-sys-color-error": get(E, 80),
        "md-sys-color-on-error": get(E, 20),
        "md-sys-color-error-container": get(E, 30),
        "md-sys-color-on-error-container": get(E, 90),
        "md-sys-color-surface": get(N, 6),
        "md-sys-color-surface-dim": get(N, 6),
        "md-sys-color-surface-bright": get(N, 24),
        "md-sys-color-surface-container-lowest": get(N, 4),
        "md-sys-color-surface-container-low": get(N, 10),
        "md-sys-color-surface-container": get(N, 12),
        "md-sys-color-surface-container-high": get(N, 17),
        "md-sys-color-surface-container-highest": get(N, 22),
        "md-sys-color-on-surface": get(N, 90),
        "md-sys-color-on-surface-variant": get(N, 80),
        "md-sys-color-outline": get(N, 60),
        "md-sys-color-outline-variant": get(N, 30),
        "md-sys-color-inverse-surface": get(N, 90),
        "md-sys-color-inverse-on-surface": get(N, 20),
        "md-sys-color-inverse-primary": get(P, 40),
        "md-sys-color-scrim": "#000000",
        "md-sys-color-shadow": "#000000",
      }
    : {
        "md-sys-color-primary": get(P, 40),
        "md-sys-color-on-primary": get(P, 100),
        "md-sys-color-primary-container": get(P, 90),
        "md-sys-color-on-primary-container": get(P, 10),
        "md-sys-color-secondary": get(S, 40),
        "md-sys-color-on-secondary": get(S, 100),
        "md-sys-color-secondary-container": get(S, 90),
        "md-sys-color-on-secondary-container": get(S, 10),
        "md-sys-color-tertiary": get(T, 40),
        "md-sys-color-on-tertiary": get(T, 100),
        "md-sys-color-tertiary-container": get(T, 90),
        "md-sys-color-on-tertiary-container": get(T, 10),
        "md-sys-color-error": get(E, 40),
        "md-sys-color-on-error": get(E, 100),
        "md-sys-color-error-container": get(E, 90),
        "md-sys-color-on-error-container": get(E, 10),
        "md-sys-color-surface": get(N, 98),
        "md-sys-color-surface-dim": get(N, 87),
        "md-sys-color-surface-bright": get(N, 98),
        "md-sys-color-surface-container-lowest": get(N, 100),
        "md-sys-color-surface-container-low": get(N, 96),
        "md-sys-color-surface-container": get(N, 94),
        "md-sys-color-surface-container-high": get(N, 92),
        "md-sys-color-surface-container-highest": get(N, 90),
        "md-sys-color-on-surface": get(N, 10),
        "md-sys-color-on-surface-variant": get(N, 30),
        "md-sys-color-outline": get(N, 50),
        "md-sys-color-outline-variant": get(N, 80),
        "md-sys-color-inverse-surface": get(N, 20),
        "md-sys-color-inverse-on-surface": get(N, 95),
        "md-sys-color-inverse-primary": get(P, 80),
        "md-sys-color-scrim": "#000000",
        "md-sys-color-shadow": "#000000",
      };

  // State-layer overlays (8%/12% тона color роли) без color-mix — поддерживаются
  // всеми рантаймами (включая старший WebKitGTK).
  const state: Record<string, string> = {
    "md-sys-state-primary-hover": hexWithAlpha(tokens["md-sys-color-on-primary"], 0.08),
    "md-sys-state-primary-pressed": hexWithAlpha(tokens["md-sys-color-on-primary"], 0.12),
    "md-sys-state-error-hover": hexWithAlpha(tokens["md-sys-color-on-error"], 0.08),
    "md-sys-state-error-pressed": hexWithAlpha(tokens["md-sys-color-on-error"], 0.12),
    "md-sys-state-on-surface-hover": hexWithAlpha(tokens["md-sys-color-on-surface"], 0.08),
    "md-sys-state-on-surface-pressed": hexWithAlpha(tokens["md-sys-color-on-surface"], 0.12),
  };

  const style = document.documentElement.style;
  for (const [name, value] of Object.entries({ ...tokens, ...state })) {
    style.setProperty(`--${name}`, value);
  }
}

function hexWithAlpha(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

/** Генерирует tonal-палитру Material 3 (тоны 0..100) по оттенку/насыщенности. */
function toneRamp(hueDeg: number, baseSat: number): Record<number, string> {
  const ramp: Record<number, string> = {};
  const step = Math.max(0, Math.min(1, baseSat));
  for (let tone = 0; tone <= 100; tone += 1) {
    const factor = interpolateFactor(tone);
    ramp[tone] = hslToHex(hueDeg, step * factor, tone / 100);
  }
  return ramp;
}

function interpolateFactor(tone: number): number {
  const keys = Object.keys(TONE_SAT_FACTOR)
    .map(Number)
    .sort((a, b) => a - b);
  if (tone <= keys[0]) return TONE_SAT_FACTOR[keys[0]];
  if (tone >= keys[keys.length - 1]) return TONE_SAT_FACTOR[keys[keys.length - 1]];
  for (let i = 0; i < keys.length - 1; i += 1) {
    const a = keys[i];
    const b = keys[i + 1];
    if (tone >= a && tone <= b) {
      const t = (tone - a) / (b - a);
      return TONE_SAT_FACTOR[a] + (TONE_SAT_FACTOR[b] - TONE_SAT_FACTOR[a]) * t;
    }
  }
  return TONE_SAT_FACTOR[keys[keys.length - 1]];
}

interface Hsl {
  h: number;
  s: number;
  l: number;
}

function hexToHsl(hex: string): Hsl {
  const n = parseInt(hex.slice(1), 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
        break;
      case g:
        h = ((b - r) / d + 2) * 60;
        break;
      default:
        h = ((r - g) / d + 4) * 60;
    }
  }
  return { h, s, l };
}

function hslToHex(h: number, s: number, l: number): string {
  const sat = Number.isFinite(s) ? Math.max(0, Math.min(1, s)) : 0;
  const light = Math.max(0, Math.min(1, l));
  const hh = ((h % 360) + 360) % 360;
  const c = (1 - Math.abs(2 * light - 1)) * sat;
  const x = c * (1 - Math.abs(((hh / 60) % 2) - 1));
  const m = light - c / 2;
  let rgb: [number, number, number];
  if (hh < 60) rgb = [c, x, 0];
  else if (hh < 120) rgb = [x, c, 0];
  else if (hh < 180) rgb = [0, c, x];
  else if (hh < 240) rgb = [0, x, c];
  else if (hh < 300) rgb = [x, 0, c];
  else rgb = [c, 0, x];
  return `#${rgb
    .map((v) => Math.round((v + m) * 255).toString(16).padStart(2, "0"))
    .join("")}`;
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