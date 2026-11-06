/**
 * The K1 check shapes the check form reads and writes. The codegen emits no input types without
 * documents (E-1 pending), so these match `contracts/schema.graphql` exactly.
 */
export type CheckType = 'ICMP' | 'TCP' | 'TLS' | 'HTTP' | 'DNS' | 'PUSH' | 'GROUP';
export type IpVersion = 'V4' | 'V6';
export type HttpMethod = 'GET' | 'HEAD' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'OPTIONS';
export type HttpAuthKind = 'NONE' | 'BASIC' | 'BEARER';
export type DnsRecordType = 'A' | 'AAAA' | 'CNAME' | 'MX' | 'NS' | 'TXT' | 'SRV' | 'PTR';

export interface StatusRange {
  readonly from: number;
  readonly to: number;
}

/** `value` is null for the headers whose values the hub never returns. */
export interface HttpHeader {
  readonly name: string;
  readonly value: string | null;
}

/** The part of K1 `CheckConfig` the form reads, discriminated like the GraphQL union. */
export type CheckConfigFields =
  | { readonly __typename: 'IcmpConfig'; readonly timeoutMs: number; readonly ipVersion: IpVersion }
  | { readonly __typename: 'TcpConfig'; readonly port: number; readonly timeoutMs: number }
  | { readonly __typename: 'TlsConfig'; readonly port: number; readonly serverName: string | null; readonly timeoutMs: number }
  | {
      readonly __typename: 'HttpConfig';
      readonly url: string;
      readonly method: HttpMethod;
      readonly headers: readonly HttpHeader[];
      readonly body: string | null;
      readonly maxRedirects: number;
      readonly acceptedStatus: readonly StatusRange[];
      readonly keyword: string | null;
      readonly jsonQuery: string | null;
      readonly jsonExpected: string | null;
      readonly authKind: HttpAuthKind;
      readonly verifyTls: boolean;
      readonly timeoutMs: number;
    }
  | {
      readonly __typename: 'DnsConfig';
      readonly name: string;
      readonly recordType: DnsRecordType;
      readonly resolver: string | null;
      readonly expected: string | null;
    }
  | { readonly __typename: 'PushConfig'; readonly pushPath: string }
  | { readonly __typename: 'GroupConfig'; readonly children: readonly { readonly id: string }[] };

/** The part of K1 `Check` the form edits. */
export interface CheckFields {
  readonly id: string;
  readonly type: CheckType;
  readonly name: string;
  readonly intervalSeconds: number;
  readonly retries: number;
  readonly retryIntervalSeconds: number;
  readonly degradedThresholdMs: number;
  readonly invert: boolean;
  readonly config: CheckConfigFields;
}

export interface HttpHeaderInput {
  readonly name: string;
  readonly value: string;
}

/** `@oneOf`: exactly one member is present. An empty password or token keeps the stored one (see `config-input.ts`). */
export type HttpAuthInput =
  | { readonly basic: { readonly username: string; readonly password: string } }
  | { readonly bearerToken: string };

export interface HttpConfigInput {
  readonly url: string;
  readonly method: HttpMethod;
  readonly headers: readonly HttpHeaderInput[];
  readonly body?: string;
  readonly maxRedirects: number;
  readonly acceptedStatus: readonly StatusRange[];
  readonly keyword?: string;
  readonly jsonQuery?: string;
  readonly jsonExpected?: string;
  readonly auth?: HttpAuthInput;
  readonly verifyTls: boolean;
  readonly timeoutMs: number;
}

/** `@oneOf`: exactly one member is present. */
export type CheckConfigInput =
  | { readonly icmp: { readonly timeoutMs: number; readonly ipVersion: IpVersion } }
  | { readonly tcp: { readonly port: number; readonly timeoutMs: number } }
  | { readonly tls: { readonly port: number; readonly serverName?: string; readonly timeoutMs: number } }
  | { readonly http: HttpConfigInput }
  | {
      readonly dns: {
        readonly name: string;
        readonly recordType: DnsRecordType;
        readonly resolver?: string;
        readonly expected?: string;
      };
    }
  | { readonly push: true }
  | { readonly group: { readonly childIds: readonly string[] } };

export interface CreateCheckInput {
  readonly deviceId: string;
  readonly parentId?: string;
  readonly name: string;
  readonly config: CheckConfigInput;
  readonly intervalSeconds: number;
  readonly retries: number;
  readonly retryIntervalSeconds: number;
  readonly degradedThresholdMs: number;
  readonly invert: boolean;
}

export interface UpdateCheckInput {
  readonly parentId?: string;
  readonly name?: string;
  readonly config?: CheckConfigInput;
  readonly intervalSeconds?: number;
  readonly retries?: number;
  readonly retryIntervalSeconds?: number;
  readonly degradedThresholdMs?: number;
  readonly invert?: boolean;
}
