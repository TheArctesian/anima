import type { DeviceCheck } from '$lib/checks/api';
import type {
  CheckConfigInput as HubCheckConfigInput,
  CheckType as HubCheckType,
} from '$lib/hub/generated/graphql';
import type { PendingCheckConfig, PendingCheckConfigInput, PendingCheckType } from '$lib/forms/hub-pending';

/** The K1 check shapes the check form reads and writes, from the generated types (T4, API-008). */
export type {
  CreateCheckInput,
  DnsRecordType,
  HttpAuthInput,
  HttpAuthKind,
  HttpConfigInput,
  HttpMethod,
  IpVersion,
  UpdateCheckInput,
} from '$lib/hub/generated/graphql';

/** The check types the form offers: Phase 1 from the generated types, Phase 2 from the pending ones (D-1). */
export type CheckType = HubCheckType | PendingCheckType;

/** The `@oneOf` config the form builds: the generated input and the K1 additions. */
export type CheckConfigInput = HubCheckConfigInput & PendingCheckConfigInput;

/** The part of K1 `CheckConfig` the form reads, discriminated like the GraphQL union. */
export type CheckConfigFields = DeviceCheck['config'] | PendingCheckConfig;

/** The part of K1 `Check` the form edits. */
export type CheckFields = Pick<
  DeviceCheck,
  'id' | 'name' | 'intervalSeconds' | 'retries' | 'retryIntervalSeconds' | 'degradedThresholdMs' | 'invert'
> & { type: CheckType; config: CheckConfigFields };
