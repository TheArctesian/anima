import { accessApi } from '$lib/access/api';
import { storeHubAddress } from '$lib/desktop/address';
import { joinLink, readLink } from '$lib/desktop/link/shell';
import { clearAccessToken, writeAccessToken } from '$lib/desktop/token';
import { createRequestHub } from '$lib/hub/hub';
import type { HubAccess } from './access';
import type { Answer, HostedAccount } from './account';
import { cloudAccount } from './cloud';
import { joinHostedHub } from './join';
import { markHosted } from './mark';
import { parseFailure, standInAccount } from './stand-in';

/** The cloud of the first run and the silent join of this computer to the hub. */
export interface HostedParts {
  readonly account: HostedAccount;
  readonly join: (access: HubAccess) => Promise<Answer<void>>;
}

/**
 * The one place that picks the cloud of the first run. `VITE_ANIMA_HOSTED=stand-in` picks the stand-in, which joins
 * nothing; the query `?fail=step:problem` (for example `?fail=poll:expired`) makes it fail once there. Otherwise the
 * cloud, which is a stub until the cloud API exists (`cloud.ts`), with the real join of the link and the key store.
 */
export function hostedParts(mode: string | undefined, query: URLSearchParams): HostedParts {
  if (mode === 'stand-in') {
    const standIn = standInAccount({ fail: parseFailure(query.get('fail')), hub: query.get('hub') === 'waking' ? 'waking' : 'making' });
    return { account: standIn, join: standIn.join };
  }
  return {
    account: cloudAccount(),
    join: (access) =>
      joinHostedHub(access, {
        join: joinLink,
        link: readLink,
        // The claim is the credential of this request: it carries no token.
        claim: (address, claim) => accessApi(createRequestHub({ address, token: () => Promise.resolve(null) })).claimDeviceToken(claim),
        clearToken: clearAccessToken,
        writeToken: writeAccessToken,
        saveAddress: (origin) => {
          storeHubAddress(localStorage, origin);
          markHosted(localStorage, origin);
        },
      }),
  };
}
