import type { Page } from '@playwright/test';

// Calls a spec makes from inside the page, so the request carries the page's cookies and `Origin` as a real client's would.
// Client code reaches the network only through `$lib/hub` (CQ-045); this file is test code and runs in the page.

export interface GraphqlAnswer {
  readonly data: Record<string, unknown> | null;
  readonly errors?: readonly { readonly message: string; readonly extensions?: { readonly code?: string } }[];
}

/** POSTs one GraphQL document to the hub from the page. `headers` adds request headers, such as `authorization`. */
export function callGraphql(page: Page, query: string, variables: object = {}, headers: Record<string, string> = {}): Promise<GraphqlAnswer> {
  return page.evaluate(
    async ([document, input, extra]) => {
      // eslint-disable-next-line no-restricted-globals -- test code in the page: it plays the client's request on purpose
      const response = await fetch('/graphql', {
        method: 'POST',
        headers: { 'content-type': 'application/json', ...(extra as Record<string, string>) },
        body: JSON.stringify({ query: document, variables: input }),
      });
      return (await response.json()) as GraphqlAnswer;
    },
    [query, variables, headers] as const,
  );
}

export interface SocketOutcome {
  /** Text frames as received, binary frames as `binary:<bytes>`. */
  readonly frames: readonly string[];
  /** The close code when the hub closed the socket; `null` when the page closed it after the frames it waited for. */
  readonly closeCode: number | null;
}

interface Conversation {
  /** Text messages to send once the socket is open. */
  readonly send: readonly string[];
  /** How many frames to wait for before the page closes the socket. */
  readonly expect: number;
}

/** Opens a WebSocket from the page, plays `conversation`, and returns what arrived. */
export function talkOnSocket(page: Page, url: string, protocol: string, conversation: Conversation): Promise<SocketOutcome> {
  return page.evaluate(
    ([target, subprotocol, { send, expect }]) =>
      new Promise<SocketOutcome>((resolve) => {
        const frames: string[] = [];
        // eslint-disable-next-line no-restricted-globals -- test code in the page: it plays the client's socket on purpose
        const socket = new WebSocket(target, subprotocol);
        socket.binaryType = 'arraybuffer';
        socket.onopen = () => {
          for (const message of send) socket.send(message);
        };
        socket.onmessage = (event: MessageEvent) => {
          const body: unknown = event.data;
          frames.push(typeof body === 'string' ? body : `binary:${String((body as ArrayBuffer).byteLength)}`);
          if (frames.length >= expect) socket.close(1000);
        };
        socket.onclose = (event) => {
          resolve({ frames, closeCode: frames.length >= expect ? null : event.code });
        };
      }),
    [url, protocol, conversation] as const,
  );
}
