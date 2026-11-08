import type { DeviceCheck } from '$lib/checks/api';
import type { CheckType } from '$lib/hub/generated/graphql';
import type { DomainExpiryConfigFields } from '../probes/domain-expiry';
import type { ProbeConfig } from '../probes/values';

/** The K1 check shapes the check form reads and writes, from the generated types (T4, API-008). */
export type {
  CheckConfigInput,
  CreateCheckInput,
  DnsRecordType,
  HttpAuthInput,
  HttpAuthKind,
  HttpConfigInput,
  HttpMethod,
  IpVersion,
  CheckType,
  UpdateCheckInput,
} from '$lib/hub/generated/graphql';

/** The part of K1 `CheckConfig` the form reads, discriminated like the GraphQL union; `ProbeConfig` until WP-29 (T5). */
export type CheckConfigFields = DeviceCheck['config'] | ProbeConfig | DomainExpiryConfigFields;

/** The part of K1 `Check` the form edits. */
export type CheckFields = Pick<
  DeviceCheck,
  'id' | 'name' | 'intervalSeconds' | 'retries' | 'retryIntervalSeconds' | 'degradedThresholdMs' | 'invert'
> & {
  type: CheckType;
  config: CheckConfigFields;
  /** DOMAIN_EXPIRY only: whole days left, from a document the route loads once the hub declares the field (F-3). */
  domainDaysLeft?: number | null;
};
