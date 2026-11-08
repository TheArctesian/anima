/** The themes the operator can choose (UI-005). System follows the OS through CSS alone, so no script listens (F-6). */
export const THEMES = [
  { id: 'dark', label: 'Dark' },
  { id: 'light', label: 'Light' },
  { id: 'system', label: 'System' },
] as const;

export type Theme = (typeof THEMES)[number]['id'];

export const DEFAULT_THEME: Theme = 'dark';

const STORAGE_KEY = 'anima.theme';

function isTheme(text: string | null): text is Theme {
  return THEMES.some((theme) => theme.id === text);
}

/** The theme the operator chose on this device; dark when nothing valid is stored or the storage refuses to be read. */
export function storedTheme(storage: Storage): Theme {
  let text: string | null;
  try {
    text = storage.getItem(STORAGE_KEY);
  } catch {
    return DEFAULT_THEME;
  }
  return isTheme(text) ? text : DEFAULT_THEME;
}

/** Save the theme on this device; throws when the storage refuses (quota, privacy modes). */
export function storeTheme(storage: Storage, theme: Theme): void {
  storage.setItem(STORAGE_KEY, theme);
}

/**
 * Mark the document with the theme; the generated `tokens.css` gives `data-theme="light"`, and `"system"` on a light
 * OS, the light semantic colours, so every component follows without reading the choice (TOK-011).
 */
export function applyTheme(root: HTMLElement, theme: Theme): void {
  root.dataset['theme'] = theme;
}
