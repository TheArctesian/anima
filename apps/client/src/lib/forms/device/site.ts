import type { FieldError } from '$lib/hub/error';
import { normalizeHost } from './host';

/** A site Add device creates with the device when the hub has none yet (VPN-001, VPN-002). */
export interface NewSite {
  name: string;
  subnet: string;
}

/** Where a new device goes: a site the hub has, or one created with it. */
export type SitePlacement = { readonly kind: 'existing'; readonly id: string } | { readonly kind: 'new'; readonly site: NewSite };

/** The VPN address range (NET-001): computers reached by their VPN address share one site. */
export const VPN_SITE: NewSite = { name: 'VPN', subnet: '100.64.0.0/10' };

const EMPTY: NewSite = { name: '', subnet: '' };

/** The four octets of a dotted IPv4 address, or undefined. */
function octets(text: string): number[] | undefined {
  const parts = text.split('.');
  if (parts.length !== 4 || !parts.every((part) => /^(0|[1-9]\d{0,2})$/.test(part))) return undefined;
  const numbers = parts.map(Number);
  return numbers.every((number) => number <= 255) ? numbers : undefined;
}

/** The address as a 32-bit number. */
function toNumber(parts: readonly number[]): number {
  return parts.reduce((total, part) => total * 256 + part, 0);
}

/** The bits a prefix of `length` keeps, as a number to compare with. */
function network(address: number, length: number): number {
  const size = 2 ** (32 - length);
  return Math.floor(address / size) * size;
}

/**
 * The site to offer for a device at `host`: the VPN site for a VPN address, the /24 around any other IPv4
 * address, and nothing for a host name or IPv6, which the operator fills in.
 */
export function suggestedSite(host: string): NewSite {
  const parts = octets(normalizeHost(host));
  if (parts === undefined) return { ...EMPTY };
  if (isVpnAddress(host)) return { ...VPN_SITE };
  return { name: 'Local network', subnet: `${parts.slice(0, 3).join('.')}.0/24` };
}

/** Whether `host` is an IPv4 address in the VPN range (100.64.0.0/10). */
export function isVpnAddress(host: string): boolean {
  const parts = octets(normalizeHost(host));
  return parts !== undefined && network(toNumber(parts), 10) === toNumber([100, 64, 0, 0]);
}

/** Why `subnet` is not a CIDR block the hub accepts, or undefined. IPv6 blocks are left to the hub. */
function subnetProblem(subnet: string): string | undefined {
  const [address = '', length, extra] = subnet.split('/');
  if (length === undefined || extra !== undefined || !/^\d{1,3}$/.test(length)) return 'Enter a subnet such as 10.10.0.0/24.';
  if (address.includes(':')) return Number(length) <= 128 ? undefined : 'Enter a prefix length up to 128.';
  const parts = octets(address);
  if (parts === undefined) return 'Enter a subnet such as 10.10.0.0/24.';
  const prefix = Number(length);
  if (prefix > 32) return 'Enter a prefix length up to 32.';
  const value = toNumber(parts);
  return network(value, prefix) === value ? undefined : 'The address must be the first one of the subnet.';
}

/** The site fields' errors, as `site.name` and `site.subnet`. */
export function validateNewSite(site: NewSite): FieldError[] {
  const errors: FieldError[] = [];
  if (site.name.trim() === '') errors.push({ field: 'site.name', message: 'Enter a site name.' });
  const problem = subnetProblem(site.subnet.trim());
  if (problem !== undefined) errors.push({ field: 'site.subnet', message: problem });
  return errors;
}

/** A hub error about the site fields, moved under `site.` so it shows beside them and not on the device's name. */
export function siteFieldErrors(fields: readonly FieldError[]): FieldError[] {
  return fields.map((error) => ({ ...error, field: `site.${error.field}` }));
}
