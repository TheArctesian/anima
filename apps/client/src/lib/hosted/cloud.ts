import { refusal, type HostedAccount } from './account';

/**
 * STUB. The real Anima.ing cloud is not built: the cloud API that takes a WorkOS token and a device key and gives a
 * device grant and the link address of the hub does not exist, and the hub does not check a grant (hosted-mode.md
 * Section 8, steps 3 and 4). Each call is refused, so the first run says that the cloud refused and offers a retry.
 *
 * When the cloud exists, this file holds the calls: the device authorization flow of WorkOS AuthKit as a public
 * client (HMD-032), the refresh token in the key store of the system (HMD-034), the grant from `cloud.anima.ing`
 * (HMD-004) and the link connection (HMD-005).
 */
export function cloudAccount(): HostedAccount {
  const notBuilt = () => Promise.resolve(refusal<never>('refused'));
  return { start: notBuilt, openPage: notBuilt, poll: notBuilt, hub: notBuilt, connect: notBuilt };
}
