import { DEFAULT_HUB_ADDRESS, parseHubAddress, type HubAddress } from '$lib/hub/hub';

const STORAGE_KEY = 'anima.hubAddress';

/** The stored text, or `null` when nothing is stored or the storage refuses to be read (privacy modes). */
function readText(storage: Storage): string | null {
  try {
    return storage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** The hub the operator chose, or the default when nothing is stored or the stored text no longer parses (CLI-002). */
export function storedHubAddress(storage: Storage): HubAddress {
  const text = readText(storage);
  if (text === null) return DEFAULT_HUB_ADDRESS;
  const parsed = parseHubAddress(text);
  return parsed.ok ? parsed.value : DEFAULT_HUB_ADDRESS;
}

/** Whether the operator has saved a hub address, even one that no longer parses: a first-run sign (FRN-001, NAV-009). */
export function hasStoredHubAddress(storage: Storage): boolean {
  return readText(storage) !== null;
}

/** Save the address the operator typed if it parses; a refusal saves nothing and keeps the old address (CLI-002). */
export function storeHubAddress(storage: Storage, text: string): ReturnType<typeof parseHubAddress> {
  const parsed = parseHubAddress(text);
  if (parsed.ok) storage.setItem(STORAGE_KEY, parsed.value.origin);
  return parsed;
}
