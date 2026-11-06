import type { DeviceCheck } from '$lib/checks/api';

/** The K1 check shapes the check form reads and writes, from the generated types (T4, API-008). */
export type {
  CheckConfigInput,
  CheckType,
  CreateCheckInput,
  DnsRecordType,
  HttpAuthInput,
  HttpAuthKind,
  HttpConfigInput,
  HttpMethod,
  IpVersion,
  UpdateCheckInput,
} from '$lib/hub/generated/graphql';

/** The part of K1 `CheckConfig` the form reads, discriminated like the GraphQL union. */
export type CheckConfigFields = DeviceCheck['config'];

/** The part of K1 `Check` the form edits. */
export type CheckFields = Pick<
  DeviceCheck,
  'id' | 'type' | 'name' | 'intervalSeconds' | 'retries' | 'retryIntervalSeconds' | 'degradedThresholdMs' | 'invert' | 'config'
>;
