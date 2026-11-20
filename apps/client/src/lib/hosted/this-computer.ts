import type { CloudDevice } from '$lib/desktop/cloud/device';
import { normalizeHost } from '$lib/devices/form/host';
import { LINK_SITE, type NewSite } from '$lib/devices/form/site';
import type { CreateDeviceInput } from '$lib/hub/generated/graphql';
import type { HubResult } from '$lib/hub/hub';

/** What the silent add needs; each part is injected, so that tests need no shell and no hub. */
export interface ThisComputerParts {
  /** This computer as the shell reads it; `null` when it cannot. */
  readonly device: () => Promise<CloudDevice | null>;
  /** The devices on the link of the hub, with the name the hub gave each one. */
  readonly linkDevices: () => Promise<HubResult<readonly { readonly name: string; readonly address: string }[]>>;
  /** The host of every device of the hub. */
  readonly hosts: () => Promise<HubResult<readonly string[]>>;
  readonly sites: () => Promise<HubResult<readonly { readonly id: string; readonly name: string; readonly subnet: string }[]>>;
  readonly createSite: (name: string, subnet: string) => Promise<HubResult<{ readonly id: string }>>;
  readonly createDevice: (input: CreateDeviceInput) => Promise<HubResult<unknown>>;
}

const GROUP = /^[0-9a-f]{1,4}$/;

/**
 * A host in the one form that compares: an IPv6 address as its eight groups with no leading zeros, in lower case and
 * without brackets, so `FD41:4E49:4D41:0:0:0:0:7` is `fd41:4e49:4d41::7`. Any other host is its lower-case text.
 */
export function comparable(host: string): string {
  const text = normalizeHost(host).toLowerCase();
  const [head = '', tail, extra] = text.split('::');
  if (!text.includes(':') || extra !== undefined) return text;
  const before = head === '' ? [] : head.split(':');
  const after = tail === undefined || tail === '' ? [] : tail.split(':');
  const missing = 8 - before.length - after.length;
  if (tail === undefined ? missing !== 0 : missing < 1) return text;
  const groups = [...before, ...Array.from({ length: missing }, () => '0'), ...after];
  if (!groups.every((group) => GROUP.test(group))) return text;
  return groups.map((group) => Number.parseInt(group, 16).toString(16)).join(':');
}

/** An existing site for the managed address range, or a newly created one. */
export async function networkSite(parts: Pick<ThisComputerParts, 'sites' | 'createSite'>, network: NewSite): Promise<string | null> {
  const sites = await parts.sites();
  if (!sites.ok) return null;
  const known = sites.value.find((site) => site.subnet === network.subnet || site.name === network.name);
  if (known !== undefined) return known.id;
  const created = await parts.createSite(network.name, network.subnet);
  return created.ok ? created.value.id : null;
}

/**
 * The link address of this computer when no device of the hub has it as its host; `null` when this computer is not on
 * the link, is a device already, or a read failed (a partial answer could hide its device).
 */
async function addressOnLink(device: CloudDevice, parts: ThisComputerParts): Promise<string | null> {
  const linked = await parts.linkDevices();
  if (!linked.ok) return null;
  const address = linked.value.find((link) => link.name === device.name)?.address;
  return address || null;
}

export type DeviceEnrolment = 'added' | 'already-added' | 'not-linked' | 'failed' | 'metrics-failed';

export async function enrolAtAddress(parts: Pick<ThisComputerParts, 'hosts' | 'sites' | 'createSite' | 'createDevice'>, network: NewSite, input: Omit<CreateDeviceInput, 'siteId'>): Promise<DeviceEnrolment> {
  const hosts = await parts.hosts();
  if (!hosts.ok) return 'failed';
  if (hosts.value.some((host) => comparable(host) === comparable(input.host))) return 'already-added';
  const siteId = await networkSite(parts, network);
  if (siteId === null) return 'failed';
  const created = await parts.createDevice({ ...input, siteId });
  return created.ok ? 'added' : 'failed';
}

async function add(parts: ThisComputerParts): Promise<DeviceEnrolment> {
  const device = await parts.device();
  if (device === null) return 'not-linked';
  const host = await addressOnLink(device, parts);
  if (host === null) return 'not-linked';
  const name = device.computer.trim() === '' ? device.name : device.computer.trim();
  // No default check: the hub cannot reach a link address yet, so an ICMP check would always say Down (HM-D11).
  return enrolAtAddress(parts, LINK_SITE, { name, host, kind: 'workstation', os: device.os, tags: [], addDefaultCheck: false });
}

/** The add that runs now; a second page that opens meanwhile waits for it, so it cannot add the computer twice. */
let running: Promise<DeviceEnrolment> | null = null;

/**
 * Adds the computer the app runs on to the devices of a hosted hub, with no question (HMD-044, R-136): its host is its
 * address on the link of the hub, in the site `Link`. `true` when it added a device. It does nothing when a device has
 * that host already, and every failure is quiet: the device list stays as it is, and the next start tries again.
 */
export function enrolThisComputer(parts: ThisComputerParts): Promise<DeviceEnrolment> {
  running ??= add(parts)
    .catch((): DeviceEnrolment => 'failed')
    .finally(() => {
      running = null;
    });
  return running;
}

export async function addThisComputer(parts: ThisComputerParts): Promise<boolean> {
  return (await enrolThisComputer(parts)) === 'added';
}
