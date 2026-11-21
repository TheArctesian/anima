import type { IconName } from '$lib/icons/icons';

/** The mark of each extension, by its key. An extension that is not built has no page to picture: its mark stands for it. */
const MARKS: Readonly<Record<string, IconName>> = {
  projects: 'carbon:folder',
  integrations: 'carbon:link',
  map: 'carbon:location',
  writing: 'carbon:document',
  secrets: 'carbon:locked',
  people: 'carbon:user',
  money: 'carbon:dashboard',
  history: 'carbon:recently-viewed',
  health: 'carbon:activity',
  documents: 'carbon:certificate',
  reminders: 'carbon:notification',
  assistant_sessions: 'carbon:terminal',
  wardrobe: 'carbon:tag',
  library: 'carbon:catalog',
  model_gateway: 'carbon:connect',
};

/**
 * The extensions that have a picture of their own page in their tile (OD-EXT-NEW-pictures, choice A, made overnight). The
 * gallery tool makes each picture from the sample data of the synthetic hub, so no picture holds the data of an operator.
 */
const PICTURED: ReadonlySet<string> = new Set(['projects', 'integrations']);

/** The two pictures of the page of the built extension `key`, for the dark and the light appearance, or `null`. */
export function pictureOf(key: string, built: boolean): { dark: string; light: string } | null {
  if (!built || !PICTURED.has(key)) return null;
  return { dark: `/extensions/${key}-dark.webp`, light: `/extensions/${key}-light.webp` };
}

/** The mark of the extension `key`; one that this client does not know has the mark of the page. */
export function markOf(key: string): IconName {
  return MARKS[key] ?? 'carbon:grid';
}

/** The key under which this device keeps the view of the Extensions page. */
export const VIEW_KEY = 'anima.extensions.view';

export type ExtensionsView = 'list' | 'grid';

/** The view that was chosen on this device; the list when none was, or when the store cannot be read. */
export function storedView(storage: Pick<Storage, 'getItem'> | undefined): ExtensionsView {
  try {
    return storage?.getItem(VIEW_KEY) === 'grid' ? 'grid' : 'list';
  } catch {
    return 'list';
  }
}

/** Keeps the choice on this device; a store that refuses (a private window) loses only the memory of it. */
export function storeView(storage: Pick<Storage, 'setItem'> | undefined, view: ExtensionsView): void {
  try {
    storage?.setItem(VIEW_KEY, view);
  } catch {
    // The page still shows the view that was chosen.
  }
}
