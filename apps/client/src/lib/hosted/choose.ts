import type { HostedAccount } from './account';
import { cloudAccount } from './cloud';
import { parseFailure, standInAccount } from './stand-in';

/**
 * The one place that picks the cloud of the first run. `VITE_ANIMA_HOSTED=stand-in` picks the stand-in, and the
 * query `?fail=step:problem` (for example `?fail=poll:expired`) makes it fail once there. Otherwise the cloud, which
 * is a stub until the cloud API exists (`cloud.ts`).
 */
export function hostedAccount(mode: string | undefined, query: URLSearchParams): HostedAccount {
  if (mode !== 'stand-in') return cloudAccount();
  return standInAccount({ fail: parseFailure(query.get('fail')), hub: query.get('hub') === 'waking' ? 'waking' : 'making' });
}
