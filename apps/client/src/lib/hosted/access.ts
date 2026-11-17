/**
 * What the cloud gives after the sign-in, so that this computer reaches and uses the hub with no step of the person
 * (hosted-mode.md Section 4.3): what the link join takes, and the credential that makes this device a signed-in device
 * of the hub. It is opaque: a screen, a step or a log sees an object with no fields. Only `openAccess` reads it, for
 * the join. Whether the link part is a device grant or an invite used under the hood is the cloud's choice.
 */
export interface HubAccess {
  readonly kind: 'hub-access';
}

/** The parts of a `HubAccess`. Secrets: never shown, logged or kept after the join (LOG-007). */
export interface AccessParts {
  /** What `link_join` takes. Today that is an invite (`anima-invite.1.`). */
  readonly link: string;
  /** The access token of the hub for this device, kept in the key store. */
  readonly token: string;
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
