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
