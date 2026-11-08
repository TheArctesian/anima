import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult } from '$lib/hub/hub';

const LOGIN = graphql(`
  mutation Login($input: LoginInput!) {
    login(input: $input) {
      token
    }
  }
`);

const SIGN_IN_BROWSER = graphql(`
  mutation SignInBrowser($input: LoginInput!) {
    signInBrowser(input: $input) {
      expiresAt
    }
  }
`);

const LOGOUT = graphql(`
  mutation Logout {
    logout
  }
`);

const ME = graphql(`
  query Me {
    me {
      username
    }
  }
`);

/** What the operator types to sign in. `totpCode` is left out when the account has no second factor (OD-2). */
export interface LoginInput {
  readonly username: string;
  readonly password: string;
  readonly totpCode?: string;
}

/** The access calls of the hub (SEC-002, API-003) as plain functions of a `Hub` (WP-11 H-3). */
export interface AccessApi {
  /** The access token on success. The caller keeps it only until it has written it to the key store (CLI-003). */
  login(input: LoginInput): Promise<HubResult<string>>;
  /**
   * Signs a browser in: the hub sets the `HttpOnly` cookie `__Host-anima` with the token, which script never sees
   * (CLI-003, D-3). The value is only when that token ends.
   */
  signInBrowser(input: LoginInput): Promise<HubResult<string>>;
  /** Ends the token on the hub; for a browser the hub also clears the cookie (K7). */
  logout(): Promise<HubResult<boolean>>;
  /** Who the stored token belongs to; `UNAUTHENTICATED` when there is none or it is no longer valid. */
  me(): Promise<HubResult<{ readonly username: string }>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function accessApi(hub: Hub): AccessApi {
  return {
    login: async (input) => mapValue(await hub.request(LOGIN, { input }), (value) => value.login.token),
    signInBrowser: async (input) => mapValue(await hub.request(SIGN_IN_BROWSER, { input }), (value) => value.signInBrowser.expiresAt),
    logout: async () => mapValue(await hub.request(LOGOUT, {}), (value) => value.logout),
    me: async () => mapValue(await hub.request(ME, {}), (value) => value.me),
  };
}
