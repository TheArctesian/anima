/**
 * What the cloud gives after the sign-in, so that this computer reaches and uses the hub with no step of the person
 * (hosted-mode.md Section 4.3, R-129): the invite of the link, and the one-time claim that this device trades with the
 * hub for its own token. The cloud never sees that token. It is opaque: a screen, a step or a log sees an object with no
 * fields. Only `openAccess` reads it, for the join.
 */
export interface HubAccess {
  readonly kind: 'hub-access';
}

/** The parts of a `HubAccess`. Secrets: never shown, logged or kept after the join (LOG-007). */
export interface AccessParts {
  /** What `link_join` takes (`anima-invite.1.`), or `null` when this device is already on the hub's link. */
  readonly invite: string | null;
  /** The claim of `claimDeviceToken`: it works once, for the device that joined with `invite`. */
  readonly claim: string;
}

const sealed = new WeakMap<HubAccess, AccessParts>();

export function sealAccess(parts: AccessParts): HubAccess {
  const access: HubAccess = Object.freeze({ kind: 'hub-access' });
  sealed.set(access, parts);
  return access;
}

/** The parts of `access`, or `null` for an access that `sealAccess` did not make. */
export function openAccess(access: HubAccess): AccessParts | null {
  return sealed.get(access) ?? null;
}
