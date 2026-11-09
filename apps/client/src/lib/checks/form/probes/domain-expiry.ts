import type { FieldError } from '$lib/hub/hub';
import { isValidHost, normalizeHost } from '$lib/devices/form/host';
import { dayCount } from '../../days';
import type { CheckConfigInput, CheckType } from '../model/hub-types';
import { integerError, isIntegerIn, lengthError, timeoutError, toNumber } from '../model/range';

/** D-29: RDAP is slow to change, so a domain expiry check runs at most once an hour. */
export const MIN_INTERVAL_SECONDS = 3600;
/** D-29: the interval the form proposes for a new domain expiry check, 12 hours. */
export const PROPOSED_INTERVAL_SECONDS = 43_200;

const DOMAIN_BYTES = 253;
const WARN_DAYS: readonly [number, number] = [1, 365];

/** A domain expiry config with its fields; the generated `Check.config` union names it without them until the hub declares them (F-3). */
export interface DomainExpiryConfigFields {
  readonly __typename: 'DomainExpiryConfig';
  readonly domain: string;
  readonly warnDays: number;
  readonly timeoutMs: number;
}

/** A domain expiry config as the generated union names it today: the type only. */
interface BareDomainExpiryConfig {
  readonly __typename: 'DomainExpiryConfig';
}

/** What the operator typed for a domain expiry check (CHK-009). */
export interface DomainExpiryValues {
  domain: string;
  warnDays: string;
  timeoutMs: string;
  /** Whole days until the registration expires, as the hub last read it; null when unknown. Never sent. */
  daysLeft: number | null;
}

export function blankDomainExpiry(): DomainExpiryValues {
  return { domain: '', warnDays: '30', timeoutMs: '10000', daysLeft: null };
}

/** `values` with the settings of a saved config, or as they were when the config has no fields yet. */
export function withDomainExpiryConfig(
  values: DomainExpiryValues,
  config: DomainExpiryConfigFields | BareDomainExpiryConfig,
  daysLeft: number | null,
): DomainExpiryValues {
  if (!('domain' in config)) return { ...values, daysLeft };
  return { domain: config.domain, warnDays: String(config.warnDays), timeoutMs: String(config.timeoutMs), daysLeft };
}

/**
 * D-29: the interval a new check of `type` shows. A touched interval stays as the operator typed it;
 * otherwise a domain expiry check proposes 12 hours and any other type takes `ordinary`.
 */
export function intervalFor(type: CheckType, current: string, ordinary: string, touched: boolean): string {
  if (touched) return current;
  return type === 'DOMAIN_EXPIRY' ? String(PROPOSED_INTERVAL_SECONDS) : ordinary;
}

/** D-29: refuses an interval under one hour; a malformed interval is left to the general settings check. */
export function intervalErrors(intervalSeconds: string): FieldError[] {
  if (!isIntegerIn(intervalSeconds, 0) || toNumber(intervalSeconds) >= MIN_INTERVAL_SECONDS) return [];
  return [{ field: 'intervalSeconds', message: 'A domain expiry check runs at most once an hour: enter 3600 seconds or more.' }];
}

function domainError(name: string): FieldError[] {
  if (name === '') return [{ field: 'domain', message: 'Enter the domain, such as example.net.' }];
  if (!isValidHost(name) || !name.includes('.') || name.includes(':') || /^[\d.]+$/.test(name)) {
    return [{ field: 'domain', message: 'Enter a domain name such as example.net, not an address.' }];
  }
  return lengthError('domain', name, DOMAIN_BYTES, 'The domain');
}

/** CHK-009, D-29: every reason the domain, warn days or timeout cannot be saved. */
export function domainExpiryErrors({ domain, warnDays, timeoutMs }: DomainExpiryValues): FieldError[] {
  return [
    ...domainError(normalizeHost(domain)),
    ...integerError('warnDays', warnDays, WARN_DAYS, `Enter the warn days from ${String(WARN_DAYS[0])} to ${String(WARN_DAYS[1])}.`),
    ...timeoutError(timeoutMs),
  ];
}

/** The `@oneOf` config of a domain expiry check. */
export function domainExpiryInput({ domain, warnDays, timeoutMs }: DomainExpiryValues): CheckConfigInput {
  return { domainExpiry: { domain: normalizeHost(domain), warnDays: toNumber(warnDays), timeoutMs: toNumber(timeoutMs) } };
}

/** The days-left line for the check form; the same words in every place that shows it. */
export function daysLeftText(daysLeft: number | null): string {
  if (daysLeft === null) return 'Days left: not known yet';
  return `Days left: ${dayCount(daysLeft)}`;
}
