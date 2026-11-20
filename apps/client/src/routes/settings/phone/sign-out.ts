import { signOut } from '$lib/access/sign-out';
import type { AccessApi } from '$lib/access/api';
import { clearCloudSession } from '$lib/desktop/cloud/shell';
import { clearAccessToken } from '$lib/desktop/token';
import type { HubAddress } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';

export async function signOutPhone(api: Pick<AccessApi, 'logout'>, address: HubAddress): Promise<string | null> {
  const platform = capabilities();
  const problem = await signOut({ api, address, desktop: platform.keyStore, clearToken: clearAccessToken });
  if (!platform.cloudAccount) return problem;
  try {
    await clearCloudSession();
  } catch {
    return problem ?? 'The key store kept the Anima.ing account session. Try signing out again.';
  }
  return problem;
}
