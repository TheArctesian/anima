/** The colour palettes the operator can choose (UI-032). A third palette is one more entry here and its token block. */
export const PALETTES = [
  { id: 'graphite', label: 'Graphite' },
  { id: 'nord', label: 'Nord' },
] as const;

export type Palette = (typeof PALETTES)[number]['id'];

export const DEFAULT_PALETTE: Palette = 'graphite';

const STORAGE_KEY = 'anima.palette';

function isPalette(text: string | null): text is Palette {
  return PALETTES.some((palette) => palette.id === text);
}

/** The palette the operator chose on this device; Graphite when nothing valid is stored or the storage refuses to be read. */
export function storedPalette(storage: Storage): Palette {
  let text: string | null;
  try {
    text = storage.getItem(STORAGE_KEY);
  } catch {
    return DEFAULT_PALETTE;
  }
  return isPalette(text) ? text : DEFAULT_PALETTE;
}

/** Save the palette on this device; throws when the storage refuses (quota, privacy modes). */
export function storePalette(storage: Storage, palette: Palette): void {
  storage.setItem(STORAGE_KEY, palette);
}

/**
 * Mark the document with the palette; the generated `tokens.css` gives each `data-palette` its colours, so every
 * component follows without reading the choice (UI-032).
 */
export function applyPalette(root: HTMLElement, palette: Palette): void {
  root.dataset['palette'] = palette;
}
