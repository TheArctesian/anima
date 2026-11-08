import { graphql } from '$lib/hub/generated';
import type { ApiKeyInput, ApiKeysQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// Own documents (WP-50 F-3): the K1 fields are @pending, so a hub without them fails these documents only, and only
// the API keys section shows an error.
const API_KEYS = graphql(`
  query ApiKeys {
    apiKeys {
      id
      name
      scope
      prefix
      expiresAt
      createdAt
      lastUsedAt
    }
  }
`);

const CREATE_API_KEY = graphql(`
  mutation CreateApiKey($input: ApiKeyInput!) {
    createApiKey(input: $input) {
      key {
        id
        name
        scope
        prefix
        expiresAt
        createdAt
        lastUsedAt
      }
      secret
    }
  }
`);

const REVOKE_API_KEY = graphql(`
  mutation RevokeApiKey($id: ID!) {
    revokeApiKey(id: $id)
  }
`);

/** One API key as the hub lists it; the secret is never part of it (INT-003). */
export type ApiKey = ApiKeysQuery['apiKeys'][number];

export type { ApiKeyInput };

/**
 * A key the hub has just made. `secret` is the whole key, shown once (INT-003): the caller keeps it only while the
 * dialog that shows it is open and never logs, stores or sends it anywhere (LOG-007).
 */
export interface CreatedApiKey {
  readonly key: ApiKey;
  readonly secret: string;
}

/** The API key calls of the hub (INT-003, D-23) as plain functions of a `Hub`. */
export interface ApiKeysApi {
  list(): Promise<HubResult<readonly ApiKey[]>>;
  create(input: ApiKeyInput): Promise<HubResult<CreatedApiKey>>;
  /** The id of the revoked key; the key stops working at once. */
  revoke(id: string): Promise<HubResult<string>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function apiKeysApi(hub: Hub): ApiKeysApi {
  return {
    list: async () => mapValue(await hub.request(API_KEYS, {}), (value) => value.apiKeys),
    create: async (input) => mapValue(await hub.request(CREATE_API_KEY, { input }), (value) => value.createApiKey),
    revoke: async (id) => mapValue(await hub.request(REVOKE_API_KEY, { id }), (value) => value.revokeApiKey),
  };
}
