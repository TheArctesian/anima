/** The typefaces the operator can choose for each text role (UI-010, UI-011, UI-020). */
export const TYPEFACES = [
  { id: 'fira-code', label: 'Fira Code' },
  { id: 'opendyslexic', label: 'OpenDyslexic' },
] as const;

export type Typeface = (typeof TYPEFACES)[number]['id'];

/** Interface text, and the monospace text of IP addresses, host names and code. */
export const TEXT_ROLES = [
  { id: 'ui', label: 'Interface font' },
  { id: 'mono', label: 'Monospace font' },
] as const;

export type TextRole = (typeof TEXT_ROLES)[number]['id'];

export type TypefaceChoice = Readonly<Record<TextRole, Typeface>>;

export const DEFAULT_TYPEFACE: Typeface = 'fira-code';

const storageKey = (role: TextRole): string => `anima.typeface.${role}`;

function isTypeface(text: string | null): text is Typeface {
  return TYPEFACES.some((typeface) => typeface.id === text);
}

/** The stored text, or `null` when nothing is stored or the storage refuses to be read (privacy modes). */
function readText(storage: Storage, role: TextRole): string | null {
  try {
    return storage.getItem(storageKey(role));
  } catch {
    return null;
  }
}

/** The typeface the operator chose for each role; the default when nothing valid is stored (UI-020). */
export function storedTypefaces(storage: Storage): TypefaceChoice {
  const choose = (role: TextRole): Typeface => {
    const text = readText(storage, role);
    return isTypeface(text) ? text : DEFAULT_TYPEFACE;
  };
  return { ui: choose('ui'), mono: choose('mono') };
}

/** Save the typeface for one role on this device. */
export function storeTypeface(storage: Storage, role: TextRole, typeface: Typeface): void {
  storage.setItem(storageKey(role), typeface);
}

/**
 * Mark the document with the chosen typefaces; `global.css` points the `font-ui` and `font-mono` tokens at the
 * chosen family, so every component follows without reading the choice (UI-020).
 */
export function applyTypefaces(root: HTMLElement, choice: TypefaceChoice): void {
  root.dataset['fontUi'] = choice.ui;
  root.dataset['fontMono'] = choice.mono;
}
