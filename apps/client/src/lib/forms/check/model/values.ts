import type {
  CheckConfigFields,
  CheckFields,
  CheckType,
  DnsRecordType,
  HttpAuthKind,
  HttpMethod,
  IpVersion,
} from './hub-types';

export interface HeaderRow {
  name: string;
  value: string;
  /** The hub holds a value it never returns (Authorization, Cookie); an empty value keeps it. */
  stored: boolean;
}

export interface StatusRow {
  from: string;
  to: string;
}

/** What the operator typed. Number fields stay text so that "abc" in the port box is reportable. */
export interface CheckFormValues {
  type: CheckType;
  name: string;
  intervalSeconds: string;
  retries: string;
  retryIntervalSeconds: string;
  degradedThresholdMs: string;
  invert: boolean;
  icmp: { timeoutMs: string; ipVersion: IpVersion };
  tcp: { port: string; timeoutMs: string };
  tls: { port: string; serverName: string; timeoutMs: string };
  http: {
    url: string;
    method: HttpMethod;
    headers: HeaderRow[];
    body: string;
    maxRedirects: string;
    acceptedStatus: StatusRow[];
    keyword: string;
    jsonQuery: string;
    jsonExpected: string;
    authKind: HttpAuthKind;
    /** The kind the hub holds; the secrets themselves are never returned. */
    storedAuth: HttpAuthKind;
    username: string;
    /** Never filled from the hub; empty means "keep the stored one". */
    password: string;
    /** Never filled from the hub; empty means "keep the stored one". */
    bearerToken: string;
    verifyTls: boolean;
    timeoutMs: string;
  };
  dns: { name: string; recordType: DnsRecordType; resolver: string; expected: string };
  /** Shown once the push check is saved; the token is part of the path. */
  push: { path: string };
  group: { childIds: string[] };
}

/** CHK-008 and K1 `CreateCheckInput` defaults. */
const DEFAULTS = { intervalSeconds: '60', retries: '3', retryIntervalSeconds: '20', degradedThresholdMs: '1000' } as const;

function blank(type: CheckType): CheckFormValues {
  return {
    type,
    name: '',
    ...DEFAULTS,
    invert: false,
    icmp: { timeoutMs: '2000', ipVersion: 'V4' },
    tcp: { port: '', timeoutMs: '5000' },
    tls: { port: '443', serverName: '', timeoutMs: '5000' },
    http: {
      url: '',
      method: 'GET',
      headers: [],
      body: '',
      maxRedirects: '10',
      acceptedStatus: [{ from: '200', to: '299' }],
      keyword: '',
      jsonQuery: '',
      jsonExpected: '',
      authKind: 'NONE',
      storedAuth: 'NONE',
      username: '',
      password: '',
      bearerToken: '',
      verifyTls: true,
      timeoutMs: '10000',
    },
    dns: { name: '', recordType: 'A', resolver: '', expected: '' },
    push: { path: '' },
    group: { childIds: [] },
  };
}

function withConfig(values: CheckFormValues, config: CheckConfigFields): CheckFormValues {
  switch (config.__typename) {
    case 'IcmpConfig':
      return { ...values, icmp: { timeoutMs: String(config.timeoutMs), ipVersion: config.ipVersion } };
    case 'TcpConfig':
      return { ...values, tcp: { port: String(config.port), timeoutMs: String(config.timeoutMs) } };
    case 'TlsConfig':
      return {
        ...values,
        tls: { port: String(config.port), serverName: config.serverName ?? '', timeoutMs: String(config.timeoutMs) },
      };
    case 'HttpConfig':
      return {
        ...values,
        http: {
          ...values.http,
          url: config.url,
          method: config.method,
          headers: config.headers.map((header) => ({
            name: header.name,
            value: header.value ?? '',
            stored: header.value === null,
          })),
          body: config.body ?? '',
          maxRedirects: String(config.maxRedirects),
          acceptedStatus: config.acceptedStatus.map((range) => ({ from: String(range.from), to: String(range.to) })),
          keyword: config.keyword ?? '',
          jsonQuery: config.jsonQuery ?? '',
          jsonExpected: config.jsonExpected ?? '',
          authKind: config.authKind,
          storedAuth: config.authKind,
          verifyTls: config.verifyTls,
          timeoutMs: String(config.timeoutMs),
        },
      };
    case 'DnsConfig':
      return {
        ...values,
        dns: {
          name: config.name,
          recordType: config.recordType,
          resolver: config.resolver ?? '',
          expected: config.expected ?? '',
        },
      };
    case 'PushConfig':
      return { ...values, push: { path: config.pushPath } };
    case 'GroupConfig':
      return { ...values, group: { childIds: config.children.map((child) => child.id) } };
  }
}

/**
 * CHK-009: the form values for a new check of `type` with the K1 defaults, or for editing `check`.
 * An existing check keeps its own type; `type` is then ignored. No secret is ever filled in.
 */
export function checkValues(check: CheckFields | null, type: CheckType): CheckFormValues {
  if (check === null) return blank(type);
  return withConfig(
    {
      ...blank(check.type),
      name: check.name,
      intervalSeconds: String(check.intervalSeconds),
      retries: String(check.retries),
      retryIntervalSeconds: String(check.retryIntervalSeconds),
      degradedThresholdMs: String(check.degradedThresholdMs),
      invert: check.invert,
    },
    check.config,
  );
}
