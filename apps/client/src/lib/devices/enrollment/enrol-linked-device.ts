import type { DeviceLinkStatus, LinkResult } from '$lib/desktop/link/shell';
import { LINK_SITE } from '$lib/devices/form/site';
import type { OperatingSystem } from '$lib/hub/generated/graphql';
import type { HubResult } from '$lib/hub/hub';
import { deviceLinkAddress } from '$lib/link/devices';
import { enrolAtAddress, type DeviceEnrolment, type ThisComputerParts } from '$lib/hosted/this-computer';

export interface LinkedDeviceParts extends Omit<ThisComputerParts, 'device' | 'linkDevices'> {
  readonly link: () => Promise<LinkResult<DeviceLinkStatus>>;
  readonly linkDevices: () => Promise<HubResult<readonly { readonly key: string; readonly address: string; readonly name?: string }[]>>;
}

interface DeviceIdentity {
  readonly name: string;
  readonly kind: string;
  readonly os?: OperatingSystem;
  readonly invitedKey?: string;
}

function activeLinkKey(origin: string, { joined, state, port, publicKey }: DeviceLinkStatus): string | undefined {
  if (!joined || state !== 'up' || port === undefined || deviceLinkAddress(port)?.origin !== origin) return undefined;
  return publicKey;
}

async function add(origin: string, parts: LinkedDeviceParts, device: DeviceIdentity): Promise<DeviceEnrolment> {
  const local = await parts.link();
  if (!local.ok) return 'failed';
  const publicKey = activeLinkKey(origin, local.value);
  if (publicKey === undefined) return 'not-linked';
  if (device.invitedKey !== undefined && device.invitedKey !== publicKey) return 'not-linked';
  const linked = await parts.linkDevices();
  if (!linked.ok) return 'failed';
  const linkedDevice = linked.value.find((candidate) => candidate.key === publicKey);
  if (!linkedDevice?.address) return 'not-linked';
  const name = (device.invitedKey === undefined ? undefined : linkedDevice.name) || device.name;
  return enrolAtAddress(parts, LINK_SITE, { name, kind: device.kind, host: linkedDevice.address, os: device.os ?? 'OTHER', tags: [], metricsSource: 'NONE', addDefaultCheck: false });
}

const adding = new Map<string, Promise<DeviceEnrolment>>();

/** One explicit inventory enrollment per app and hub at a time, using the current link's public identity. */
export function enrolLinkedDevice(origin: string, parts: LinkedDeviceParts, device: DeviceIdentity): Promise<DeviceEnrolment> {
  const pending = adding.get(origin);
  if (pending !== undefined) return pending;
  const task = add(origin, parts, device).catch((): DeviceEnrolment => 'failed').finally(() => adding.delete(origin));
  adding.set(origin, task);
  return task;
}
