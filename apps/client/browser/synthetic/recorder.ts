// What the synthetic hub saw, so a spec can assert on it. Only facts about headers are kept, never a header's value
// (LOG-007): a cookie or an authorization value is never stored here.

export type Channel = 'graphql' | 'graphql-socket' | 'terminal' | 'tunnel';

export interface RecordedRequest {
  readonly channel: Channel;
  /** The GraphQL operation name, or its first root field for an unnamed one; `null` for a tunnel. */
  readonly operation: string | null;
  readonly variables: unknown;
  readonly hadAuthorization: boolean;
  readonly hadCookie: boolean;
  readonly origin: string | null;
  /** Whether the hub treated the request as signed in. */
  readonly signedIn: boolean;
}

export interface Recorder {
  readonly requests: readonly RecordedRequest[];
  /** Every request to another origin was aborted; this counts them. Specs assert 0. */
  readonly foreignRequests: number;
  /** The origins (never paths or queries) that were refused. */
  readonly foreignOrigins: readonly string[];
  /** Requests of one channel, or of one GraphQL operation. */
  where(match: { channel?: Channel; operation?: string }): RecordedRequest[];
}

export interface Recording extends Recorder {
  record(request: RecordedRequest): void;
  refuseForeign(url: string): void;
}

export function newRecording(): Recording {
  const requests: RecordedRequest[] = [];
  const foreignOrigins: string[] = [];
  return {
    requests,
    get foreignRequests() {
      return foreignOrigins.length;
    },
    foreignOrigins,
    where: ({ channel, operation }) =>
      requests.filter((found) => (channel === undefined || found.channel === channel) && (operation === undefined || found.operation === operation)),
    record: (request) => requests.push(request),
    refuseForeign: (url) => foreignOrigins.push(URL.canParse(url) ? new URL(url).origin : 'unparsable'),
  };
}
