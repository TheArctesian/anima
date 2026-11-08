// Where a channel sends, as the list shows it: an origin or a host and nothing more (D-17, F-8). The hub already
// returns only these, but the list does not rely on it: a path, a query, a fragment or a user name and password in
// what the hub sends never reaches the screen, because the text is rebuilt from the parsed parts it may show.
import type { ChannelKind, ChannelSettings } from '../api';

/** Shown in place of a target the client cannot reduce to an origin or host. */
export const TARGET_HIDDEN = 'Not shown';

const KIND_LABELS: Record<ChannelKind, string> = { WEBHOOK: 'Webhook', NTFY: 'ntfy', EMAIL: 'Email', APPRISE: 'Apprise' };

export function kindLabel(kind: ChannelKind): string {
  return KIND_LABELS[kind];
}

function httpUrl(text: string): URL | null {
  try {
    const url = new URL(text);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url : null;
  } catch {
    // Not a URL: nothing of it is shown.
    return null;
  }
}

/** `https://hooks.example.net` for any http(s) URL; userinfo, path, query and fragment are dropped. */
export function originOf(text: string): string {
  return httpUrl(text)?.origin ?? TARGET_HIDDEN;
}

/** `ntfy.sh`, or `ntfy.example.net:8443` with a port that is not the scheme's own. */
export function hostOf(text: string): string {
  return httpUrl(text)?.host ?? TARGET_HIDDEN;
}

/** A DNS name or an IPv4 address, or an IPv6 address in brackets; anything else is not shown. */
const MAIL_HOST = /^(?:[a-z0-9-]+(?:\.[a-z0-9-]+)*|\[[0-9a-f:.]+\])$/i;

/** The SMTP host as the hub gives it, when it is a plain host name. */
export function mailHostOf(host: string): string {
  return MAIL_HOST.test(host) ? host.toLowerCase() : TARGET_HIDDEN;
}

/**
 * The origin of an https URL that is nothing but an origin, `/` allowed (CR-14.6, CR-14.7): no user name or password,
 * no other path, no query and no fragment, not even an empty one. `null` for anything else.
 */
export function cleanOrigin(text: string): string | null {
  if (/[@?#\s\\]/.test(text)) return null;
  const url = httpUrl(text);
  return url?.protocol === 'https:' && url.pathname === '/' ? url.origin : null;
}

/** The origin of an http(s) URL, or `''`: what the editor shows of a server the hub holds, never its user name. */
export function shownOrigin(text: string): string {
  return httpUrl(text)?.origin ?? '';
}

/** `smtp.example.com:587`: the email target is the host and the port together (CR-14.1). */
export function mailTarget(host: string, port: number): string | null {
  return MAIL_HOST.test(host) ? `${host.toLowerCase()}:${String(port)}` : null;
}

/**
 * What the saved channel sends to, in the form the editor compares a new target with (CR-14.1): the webhook's origin,
 * the ntfy or Apprise server as a clean origin, the email host and port. `null` when it cannot be read: any target
 * then counts as new, so the stored secrets are asked for again.
 */
export function storedTarget(settings: ChannelSettings): string | null {
  switch (settings.__typename) {
    case 'WebhookSettings':
      return httpUrl(settings.origin)?.origin ?? null;
    case 'NtfySettings':
    case 'AppriseSettings':
      return cleanOrigin(settings.server);
    case 'EmailSettings':
      return mailTarget(settings.host, settings.port);
  }
}

/** The one line the list shows for where a channel sends. Email shows the server, never the recipients. */
export function targetOf(settings: ChannelSettings): string {
  switch (settings.__typename) {
    case 'WebhookSettings':
      return originOf(settings.origin);
    case 'NtfySettings':
      return hostOf(settings.server);
    case 'EmailSettings':
      return mailHostOf(settings.host);
    case 'AppriseSettings':
      return originOf(settings.server);
  }
}
