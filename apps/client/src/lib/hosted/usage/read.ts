import { cloudHubUsage } from '$lib/desktop/cloud/shell';
import { readGateway } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { parseUsage, type UsageAnswer } from './usage';

/** The path of the gateway that answers the figures of the account of its session, on the origin of the hub. */
export const GATEWAY_PATH = '/account/v1/usage';
/** How long the page waits for the figures before it says that they are not available. */
export const WAIT_MS = 10_000;

const SIGNED_OUT: UsageAnswer = { state: 'signed-out' };
const UNAVAILABLE: UsageAnswer = { state: 'unavailable' };

function figures(value: unknown): UsageAnswer {
  const usage = parseUsage(value);
  return usage === undefined ? UNAVAILABLE : { state: 'figures', usage };
}

/** In the application: the route of the account site through the shell, with the sign-in the shell holds. */
async function fromShell(): Promise<UsageAnswer> {
  const answered = await cloudHubUsage();
  if (answered.ok) return figures(answered.value);
  return answered.problem === 'signed-out' || answered.problem === 'unconfigured' ? SIGNED_OUT : UNAVAILABLE;
}

/** In a browser at the address of the hub: the path of the gateway, for the account of its session. */
async function fromGateway(fetcher: typeof fetch | undefined): Promise<UsageAnswer> {
  const answered = await readGateway(GATEWAY_PATH, WAIT_MS, fetcher);
  if (answered?.status === 401) return SIGNED_OUT;
  return answered !== null && answered.status === 200 ? figures(answered.body) : UNAVAILABLE;
}

/**
 * The figures of the account for the page of the hub (HMD-073 in form B): from the account, never from the hub. The
 * application asks through its shell; a browser asks the gateway. Each failure is a state of the page, never its text.
 */
export function readAccountUsage(inShell: boolean = capabilities().cloudAccount, fetcher?: typeof fetch): Promise<UsageAnswer> {
  return inShell ? fromShell() : fromGateway(fetcher);
}
