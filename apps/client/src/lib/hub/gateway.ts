/** One answer of the gateway of the pool: its status and its body, or `null` when no answer arrived. */
export interface GatewayAnswer {
  readonly status: number;
  readonly body: unknown;
}

/**
 * A GET of `path` on the origin of this page, with its cookies: a path that the gateway of the pool answers itself for
 * the account of its session, never the hub (HMD-073 in form B, `contracts/account-usage.md`). The answer is checked by
 * the caller; a failure gives no text (LOG-007).
 */
export async function readGateway(path: string, waitMs: number, fetcher: typeof fetch = fetch): Promise<GatewayAnswer | null> {
  try {
    const response = await fetcher(path, { credentials: 'same-origin', signal: AbortSignal.timeout(waitMs) });
    const body: unknown = response.ok ? await response.json() : null;
    return { status: response.status, body };
  } catch {
    return null;
  }
}
