import { refusal, type HostedAccount } from './account';

/**
 * STUB. The real Anima.ing cloud is not built: the cloud API that takes a WorkOS token and a device key and gives the
 * hub access does not exist, and the hub accepts nothing from the cloud (hosted-mode.md Section 8, steps 3 and 4). Each call is refused, so the first run says that the cloud refused and offers a retry.
 *
 * When the cloud exists, this file holds the calls: the device authorization flow of WorkOS AuthKit as a public
 * client (HMD-032), the refresh token in the key store of the system (HMD-034), and the hub access from
 * `cloud.anima.ing` (HMD-004): what the link join takes and this device's credential for the hub, sealed with
 * `sealAccess`. The join itself is not a stub: `join.ts` does it with the link and the key store of the app.
 */
export function cloudAccount(): HostedAccount {
  const notBuilt = () => Promise.resolve(refusal<never>('refused'));
  return { start: notBuilt, openPage: notBuilt, poll: notBuilt, hub: notBuilt, access: notBuilt };
}
