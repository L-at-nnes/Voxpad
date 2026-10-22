/* Deterministic visuals derived from a soundboard/sound name. */
import { convertFileSrc } from '@tauri-apps/api/core';

function hash(str: string): number {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

/** Stable warm→violet gradient for a name (used as a cover/tile fallback). */
export function gradientFromName(name: string): string {
  const h = hash(name || 'voxpad');
  const warm = 15 + (h % 45); // 15..60 (red/orange/amber)
  const violet = 290 + ((h >> 5) % 50); // 290..340 (violet/magenta)
  const angle = 120 + ((h >> 3) % 60);
  return `linear-gradient(${angle}deg, oklch(0.62 0.19 ${warm}), oklch(0.55 0.2 ${violet}))`;
}

/** A subtler version for sound tiles so the grid does not vibrate. */
export function tileGradientFromName(name: string): string {
  const h = hash(name || 'sound');
  const a = 250 + (h % 90); // 250..340 (blue→magenta)
  const b = a - 40 - ((h >> 4) % 30);
  const angle = 135 + ((h >> 2) % 50);
  return `linear-gradient(${angle}deg, oklch(0.5 0.13 ${a}), oklch(0.38 0.1 ${b}))`;
}

export function initials(name: string): string {
  const clean = (name || '').trim();
  if (!clean) return '?';
  const parts = clean.split(/[\s_\-.]+/).filter(Boolean);
  if (parts.length === 0) return clean.slice(0, 2).toUpperCase();
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}

/** Turns a local file path into a webview-loadable url. */
export function fileUrl(path: string): string {
  if (!path) return '';
  if (/^(file|https?|data|asset):/.test(path)) return path;
  // Tauri's webview blocks raw file:// loads by default; local paths must go through the
  // asset protocol (enabled in tauri.conf.json) via convertFileSrc.
  return convertFileSrc(path);
}
