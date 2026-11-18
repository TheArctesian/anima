import type { DocumentNode } from 'graphql';
import type { HubAddress } from '../address';
import type { HubResult } from '../error';
import { copiedQuery, copyKey, isShown, type CopyLimits, type CopyStore } from './copy';

/** What the copy needs from the hub that it serves; every part is injected (CQ-021). */
export interface RecentOptions {
  readonly store: CopyStore;
  readonly limits: CopyLimits;
  readonly address: HubAddress;
  readonly now: () => Date;
  /** Hears the time of each copy that is given in place of an answer, so that the status can show its age (NET-004). */
  readonly served: (at: Date) => void;
}

/**
 * The read-only copy of what the client showed recently (FND-007). `answer` runs one request through it: a good
 * answer to a copied query is kept, and when the hub does not answer, the kept answer is given in its place.
 */
export interface RecentCopy {
  answer<TResult>(document: DocumentNode, variables: unknown, send: () => Promise<HubResult<TResult>>): Promise<HubResult<TResult>>;
}

function parsed(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    // A copy that cannot be read is no copy.
    return undefined;
  }
}

async function kept(options: RecentOptions, key: string): Promise<{ at: Date; value: unknown } | undefined> {
  // A store that fails gives no copy: the caller then reports the hub as unreachable, which is the truth.
  const copy = await options.store.read(key).catch(() => undefined);
  if (copy === undefined || !isShown(copy.at, options.now().getTime(), options.limits)) return undefined;
  const value = parsed(copy.text);
  return value === undefined ? undefined : { at: new Date(copy.at), value };
}

function keep(options: RecentOptions, key: string, value: unknown): void {
  const text = JSON.stringify(value);
  const { store, limits } = options;
  const done = text.length > limits.longest ? store.remove(key) : store.write({ key, at: options.now().getTime(), text }, limits.most);
  // The copy is extra: a store that fails must not fail or slow the request that got a good answer.
  done.catch(() => undefined);
}

export function createRecentCopy(options: RecentOptions): RecentCopy {
  return {
    async answer<TResult>(document: DocumentNode, variables: unknown, send: () => Promise<HubResult<TResult>>) {
      const result = await send();
      const query = copiedQuery(document);
      if (query === null) return result;
      const key = copyKey(options.address, query, variables);
      if (result.ok) {
        keep(options, key, result.value);
        return result;
      }
      if (result.error.code !== 'unreachable') return result;
      const copy = await kept(options, key);
      if (copy === undefined) return result;
      options.served(copy.at);
      // The copy is an earlier answer of the hub to the same document, so it has the document's shape (API-008).
      return { ok: true, value: copy.value as TResult };
    },
  };
}
