import type { ListenerResult } from '$lib/desktop/hub-listener/shell';
import type { DeviceLinkStatus } from '$lib/desktop/link/shell';
import type { TokenProvider } from '$lib/hub/hub';

/**
 * Whether the desktop app must ask who listens at `origin` before it sends anything there (F-10): a plain `http`
 * origin, which the hub address rules permit only for this computer. Anything that does not parse is checked too, and
 * the shell refuses it.
 */
export function needsListenerCheck(origin: string, desktop: boolean): boolean {
  if (!desktop) return false;
  return !URL.canParse(origin) || new URL(origin).protocol === 'http:';
}

/** The decision for one loopback hub address: send to it only when a program of this user listens (F-10). */
export type ListenerVerdict = { readonly safe: true } | { readonly safe: false; readonly listener: 'other' | 'none' | 'unknown' };

export function verdictOf(result: ListenerResult): ListenerVerdict {
  if (!result.ok) return { safe: false, listener: 'unknown' };
  return result.listener === 'own' ? { safe: true } : { safe: false, listener: result.listener };
}

/** The gate refused: fixed text, so a hub client that reports it says nothing about the address (LOG-007). */
export class ListenerRefused extends Error {
  constructor() {
    super('The hub address on this computer is not served by this app.');
    this.name = 'ListenerRefused';
  }
}

/** The checks of loopback hub addresses (F-10). */
export interface ListenerGate {
  /** The verdict for `origin`; the shell is asked again once the last answer is older than the gate's lifetime. */
  verdict(origin: string): Promise<ListenerVerdict>;
  /** Drops the verdict for `origin`, so that the next call asks again (Check again, Start, Stop, Leave, a lost hub). */
  forget(origin: string): void;
  /**
   * `token`, read only once the verdict for the address is safe; otherwise it rejects, so nothing is sent. A `fresh`
   * reader asks the shell before every read and never takes a kept answer: for the socket, which reconnects by itself
   * after a dead hub, before the page can see the loss (security re-review MF-1).
   */
  token(token: TokenProvider, read?: TokenRead): TokenProvider;
}

/** How a gated reader takes its verdict. */
export interface TokenRead {
  readonly fresh: boolean;
}

/** How long an answer stands: a program that takes the port later is seen within this time (security review M-C). */
export const VERDICT_LIFETIME_MS = 5000;

/** What the gate needs: the shell's check and the clock (CQ-021). */
export interface GateParts {
  readonly check: (origin: string) => Promise<ListenerResult>;
  readonly now: () => number;
}

/** A gate that asks `check`; an answer stands for `VERDICT_LIFETIME_MS`, or until `forget`. */
export function listenerGate({ check, now }: GateParts): ListenerGate {
  const verdicts = new Map<string, { readonly asked: Promise<ListenerVerdict>; readonly at: number }>();
  const verdict = (origin: string): Promise<ListenerVerdict> => {
    const known = verdicts.get(origin);
    if (known !== undefined && now() - known.at < VERDICT_LIFETIME_MS) return known.asked;
    const asked = check(origin).then(verdictOf, () => ({ safe: false, listener: 'unknown' }) as const);
    verdicts.set(origin, { asked, at: now() });
    return asked;
  };
  const forget = (origin: string): void => {
    verdicts.delete(origin);
  };
  return {
    verdict,
    forget,
    token:
      (token, read = { fresh: false }) =>
      async (address) => {
        if (read.fresh) forget(address.origin);
        if (!(await verdict(address.origin)).safe) throw new ListenerRefused();
        return token(address);
      },
  };
}

/** What the sign-in page says when it will not send anything to the hub address (F-10). */
export interface ListenerProblem {
  readonly title: string;
  readonly advice: string;
}

/** The port of `origin`, or `null` when it names none. */
function portOf(origin: string): number | null {
  if (!URL.canParse(origin)) return null;
  const { port } = new URL(origin);
  return port === '' ? null : Number(port);
}

/** The device link's own account of why it does not serve the hub address, or `null` when it is not about the link. */
function linkProblem(origin: string, link: DeviceLinkStatus | null): ListenerProblem | null {
  if (link === null || !link.joined || link.port !== portOf(origin)) return null;
  const title = 'The link to the hub is not up';
  if (link.reason === 'port-taken') return { title, advice: `Another program holds the link's port ${String(link.port)}. Close it, then check again.` };
  if (link.reason === 'not-carried') return { title, advice: 'This build does not carry the link program.' };
  if (link.state === 'starting') return { title: 'The link to the hub is starting', advice: 'Check again in a moment.' };
  if (link.state === 'up') return null;
  return { title, advice: 'The link ended. Check the network of this computer, then check again.' };
}

/** The fixed text for a refused verdict; for a joined device the link's state comes first (F-10). */
export function listenerProblem(verdict: ListenerVerdict, origin: string, link: DeviceLinkStatus | null): ListenerProblem | null {
  if (verdict.safe) return null;
  const fromLink = linkProblem(origin, link);
  if (fromLink !== null) return fromLink;
  switch (verdict.listener) {
    case 'other':
      return { title: "Another program holds the hub's port", advice: 'Anima.ing sends nothing to it: no password and no token. Stop that program, then check again.' };
    case 'none':
      return { title: 'Nothing answers at the hub address', advice: "Start this computer's hub, or the link to your hub, then check again." };
    case 'unknown':
      return { title: 'The hub address could not be checked', advice: 'Anima.ing sends nothing to an address it cannot check. Check again.' };
  }
}
