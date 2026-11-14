import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult } from '$lib/hub/hub';

const EXTENSIONS = graphql(`
  query Extensions {
    extensions {
      key
      name
      built
      fixed
      on
      needs
      worksWith
      neededBy
    }
  }
`);

const SET_EXTENSION = graphql(`
  mutation SetExtension($key: String!, $on: Boolean!) {
    setExtension(key: $key, on: $on) {
      key
      name
      built
      fixed
      on
      needs
      worksWith
      neededBy
    }
  }
`);

/** One extension as the hub lists it (EXT-001). `needs` and `neededBy` are keys (EXT-011, EXT-012). */
export interface Extension {
  readonly key: string;
  readonly name: string;
  /** False: listed, not built. */
  readonly built: boolean;
  /** Part of the engine: always on, no switch. */
  readonly fixed: boolean;
  readonly on: boolean;
  readonly needs: readonly string[];
  readonly worksWith: readonly string[];
  readonly neededBy: readonly string[];
}

/** The extension calls of the hub. */
export interface ExtensionsApi {
  /** All extensions, in the hub's order. */
  list(): Promise<HubResult<readonly Extension[]>>;
  /** Sets one on or off; answers each extension whose switch changed. The hub refuses with its own words. */
  set(key: string, on: boolean): Promise<HubResult<readonly Extension[]>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function extensionsApi(hub: Pick<Hub, 'request'>): ExtensionsApi {
  return {
    list: async () => mapValue(await hub.request(EXTENSIONS, {}), (value) => value.extensions),
    set: async (key, on) => mapValue(await hub.request(SET_EXTENSION, { key, on }), (value) => value.setExtension),
  };
}
