const STOPS = 'The daemon stops sending metrics. The device keeps the Daemon source, shown as stale, until you change it.';

/**
 * What the operator reads before a daemon is revoked (MET-013, R-105). A hub with a VPN adds that the VPN access stays
 * and how to remove it; a hub that Anima.ing hosts has no VPN and no Headscale, so it names neither (R-211).
 */
export function revokeWarning(hosted: boolean): string {
  return hosted ? STOPS : `${STOPS} VPN access stays: for a device on the VPN only for the daemon, also delete its node in Headscale.`;
}
