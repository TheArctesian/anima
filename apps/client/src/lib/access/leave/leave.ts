import { clearAccessToken } from '$lib/desktop/token';
import { signOutOfGateway } from '$lib/hosted/gateway/here';
import type { Hub } from '$lib/hub/hub';
import { isMobile } from '$lib/platform/mobile';
import { capabilities } from '$lib/platform/platform';
import type { AccessApi } from '../api';
import { signOutMobile } from '../mobile/sign-out';
import { goToSignIn, goToWelcome, loadClientAt } from '../navigation';
import { SIGN_IN_PATH } from '../routes';
import { GATEWAY_KEPT, signOut } from '../sign-out';

/**
 * Signs this client out of its hub and goes to where the next sign-in starts. The app of a phone or a tablet also ends
 * its Anima.ing account session and goes to Welcome (MOB-006). A browser behind the gateway of the hosted hubs is also
 * signed out of the gateway (HMD-059), so that the next visit asks for the account again. Returns the message to show
 * when something may stay signed in; `null` when the client moved on.
 */
export async function leaveHub(
  hub: Pick<Hub, 'address' | 'close'>,
  api: Pick<AccessApi, 'logout'>,
  forgetCopy?: () => Promise<void>,
): Promise<string | null> {
  const desktop = capabilities().keyStore;
  if (desktop && isMobile(navigator)) {
    const kept = await signOutMobile(api, hub.address);
    if (kept === null) goToWelcome();
    return kept;
  }
  const kept = await signOut({ api, address: hub.address, desktop, clearToken: clearAccessToken, forgetCopy });
  if (kept !== null) return kept;
  if (desktop) {
    // The desktop socket reads the key store anew, so the router may move within the page.
    goToSignIn();
    return null;
  }
  const gateway = await signOutOfGateway();
  if (gateway === 'none') {
    // A new page, so no socket keeps the old cookie (E-4).
    loadClientAt(SIGN_IN_PATH);
    return null;
  }
  if (gateway === 'kept') return GATEWAY_KEPT;
  // A new page would go to the gateway, which starts a sign-in at once: the page stays, with its socket closed.
  await hub.close().catch(() => undefined);
  goToSignIn();
  return null;
}
