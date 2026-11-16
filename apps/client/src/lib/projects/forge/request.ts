import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubResult, Variables } from '$lib/hub/hub';
import type { ForgePage, ForgeRead } from './types';

export type Asker = Pick<Hub, 'request'>;

/** Asks the hub for `document` and gives back its field `name`, typed as codegen wrote it (API-008). */
export async function field<R, V extends Variables, K extends keyof R>(hub: Asker, document: TypedDocumentNode<R, V>, name: K, variables: V): Promise<HubResult<R[K]>> {
  const result = await hub.request(document, variables);
  return result.ok ? { ok: true, value: result.value[name] } : result;
}

interface Connection<T> {
  readonly edges: readonly { readonly node: T }[];
  readonly pageInfo: { readonly hasNextPage: boolean; readonly endCursor: string | null };
  readonly read: ForgeRead;
}

/** One page of a list as the hub sent it: its rows, the cursor of the next page, and when the hub read it. */
export const pageOf = <T>({ edges, pageInfo, read }: Connection<T>): ForgePage<T> => ({
  rows: edges.map((edge) => edge.node),
  next: pageInfo.hasNextPage ? pageInfo.endCursor : null,
  read,
});

/** A list's answer as one page, or the hub's refusal as it came. */
export const paged = <T>(result: HubResult<Connection<T>>): HubResult<ForgePage<T>> => (result.ok ? { ok: true, value: pageOf(result.value) } : result);

/** What a list call is asked: the next page's cursor, and `fresh` to skip the hub's short cache (PRJ-032). */
export interface PageAsk {
  readonly after?: string | null;
  readonly fresh?: boolean;
}

export const asked = (repository: string, ask: PageAsk) => ({ repository, after: ask.after ?? null, fresh: ask.fresh ?? false });
