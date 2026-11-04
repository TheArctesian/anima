import { createContext } from 'svelte';
import type { Hub } from './hub';

const [getHub, setHub] = createContext<Hub>();

/** Make `hub` available to every component below; call once in the root layout. */
export function provideHub(hub: Hub): void {
  setHub(hub);
}

/** The hub set by `provideHub` in a parent component; throws when there is none. */
export function useHub(): Hub {
  return getHub();
}
