import { createHash, randomBytes } from 'node:crypto';

// Tunnel tickets (K4 section 1): 32 random bytes, kept as a SHA-256, 30 seconds, single use, bound to one path.

const LIFETIME_MS = 30_000;

export interface Ticket {
  readonly token: string;
  readonly sessionId: string;
  readonly tunnelPath: string;
  readonly expiresAt: Date;
}

interface Issued {
  readonly sessionId: string;
  readonly tunnelPath: string;
  readonly expiresAt: Date;
}

export type Redeemed = { readonly ok: true; readonly sessionId: string } | { readonly ok: false };

export interface TicketBook {
  issue(sessionId: string, tunnelPath: string, now: Date): Ticket;
  /** Removes the ticket whether it is good or not, as the hub does; good only on its own path and before it expires. */
  redeem(token: string | null, tunnelPath: string, now: Date): Redeemed;
}

const digest = (token: string): string => createHash('sha256').update(token).digest('hex');

export function ticketBook(): TicketBook {
  const issued = new Map<string, Issued>();
  return {
    issue(sessionId, tunnelPath, now) {
      const token = randomBytes(32).toString('base64url');
      const expiresAt = new Date(now.getTime() + LIFETIME_MS);
      issued.set(digest(token), { sessionId, tunnelPath, expiresAt });
      return { token, sessionId, tunnelPath, expiresAt };
    },
    redeem(token, tunnelPath, now) {
      if (token === null) return { ok: false };
      const key = digest(token);
      const found = issued.get(key);
      issued.delete(key);
      const good = found !== undefined && found.tunnelPath === tunnelPath && found.expiresAt > now;
      return good ? { ok: true, sessionId: found.sessionId } : { ok: false };
    },
  };
}
