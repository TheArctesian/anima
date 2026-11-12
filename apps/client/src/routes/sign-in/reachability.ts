import type { HubStatus } from '$lib/hub/hub';
import { unreachableBanner } from '$lib/nav/nav';

/** The host of a hub address, which is the name the operator knows the hub by. */
export function hubHost(origin: string): string {
  return new URL(origin).host;
}

/** What the sign-in page says when the hub cannot be reached: the cause, the next step and the technical detail (UI-030). */
export interface HubUnreachable {
  readonly title: string;
  readonly advice: string;
  readonly origin: string;
  readonly age: string;
}

/** The notice for `status`, or `null` when the hub is not unreachable; `now` dates the data age (NET-004). */
export function hubUnreachable(status: HubStatus, origin: string, now: Date): HubUnreachable | null {
  const banner = unreachableBanner(status, origin, now);
  if (banner === null) return null;
  return { title: `Can't reach ${hubHost(origin)}`, advice: 'Check the VPN client, then try again.', origin, age: banner.age };
}
