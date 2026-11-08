// The channel editor as plain values: what the operator typed and chose, for every kind at once, so that switching
// the kind of a new channel loses nothing. Secret fields start empty, always: the hub never returns a secret, and the
// form never shows a stored one (D-17, F-8). What is stored is known only as a fact (`Stored`).
import type { AppriseMode, SmtpSecurity, WebhookMethod } from '$lib/hub/generated/graphql';
import type { ChannelKind, NotificationChannel, NotificationTemplateInput } from '../api';

/** A secret the operator may type. `clear` asks the hub to remove a stored optional secret (`""` in the input). */
export interface SecretDraft {
  value: string;
  clear: boolean;
}

export interface HeaderDraft {
  name: string;
  /** A header value is a secret (D-17). */
  value: string;
}

export interface ChannelDraft {
  name: string;
  kind: ChannelKind;
  onByDefault: boolean;
  /** False: the installation's default template (`template: null`, NOT-007). */
  ownTemplate: boolean;
  template: NotificationTemplateInput;
  webhook: { url: string; method: WebhookMethod; replaceHeaders: boolean; headers: HeaderDraft[] };
  ntfy: { server: string; topic: string; token: SecretDraft; priority: number };
  email: { host: string; port: string; security: SmtpSecurity; username: string; password: SecretDraft; from: string; to: string };
  apprise: { server: string; mode: AppriseMode; urls: string; key: string; tags: string };
}

/** What the saved channel holds, as facts: never a secret's value. */
export interface Stored {
  readonly kind: ChannelKind;
  readonly headerNames: readonly string[];
  readonly ntfyToken: boolean;
  readonly emailPassword: boolean;
  /** The Apprise mode whose URLs or key the hub holds. */
  readonly appriseMode: AppriseMode | null;
}

export const KINDS: readonly { readonly id: ChannelKind; readonly label: string }[] = [
  { id: 'WEBHOOK', label: 'Webhook' },
  { id: 'NTFY', label: 'ntfy' },
  { id: 'EMAIL', label: 'Email' },
  { id: 'APPRISE', label: 'Apprise' },
];

/** The K1 input defaults. */
export const NTFY_SERVER = 'https://ntfy.sh';
export const NTFY_PRIORITY = 3;
export const SMTP_PORT: Record<SmtpSecurity, number> = { STARTTLS: 587, TLS: 465 };

function emptySecret(): SecretDraft {
  return { value: '', clear: false };
}

/** A new channel: the K1 defaults, and the default template ready to be edited if the operator wants their own. */
export function newDraft(defaultTemplate: NotificationTemplateInput): ChannelDraft {
  return {
    name: '',
    kind: 'WEBHOOK',
    onByDefault: false,
    ownTemplate: false,
    template: { title: defaultTemplate.title, body: defaultTemplate.body },
    webhook: { url: '', method: 'POST', replaceHeaders: true, headers: [] },
    ntfy: { server: NTFY_SERVER, topic: '', token: emptySecret(), priority: NTFY_PRIORITY },
    email: { host: '', port: String(SMTP_PORT.STARTTLS), security: 'STARTTLS', username: '', password: emptySecret(), from: '', to: '' },
    apprise: { server: '', mode: 'STATELESS_URLS', urls: '', key: '', tags: '' },
  };
}

/** A saved channel's values, every secret field empty. */
export function savedDraft(channel: NotificationChannel, defaultTemplate: NotificationTemplateInput): ChannelDraft {
  const draft = newDraft(channel.template ?? defaultTemplate);
  draft.name = channel.name;
  draft.kind = channel.kind;
  draft.onByDefault = channel.onByDefault;
  draft.ownTemplate = channel.template !== null;
  const { settings } = channel;
  switch (settings.__typename) {
    case 'WebhookSettings':
      draft.webhook.method = settings.method;
      draft.webhook.replaceHeaders = false;
      break;
    case 'NtfySettings':
      Object.assign(draft.ntfy, { server: settings.server, priority: settings.priority });
      break;
    case 'EmailSettings':
      Object.assign(draft.email, {
        host: settings.host,
        port: String(settings.port),
        security: settings.security,
        username: settings.username ?? '',
        from: settings.from,
        to: settings.to.join(', '),
      });
      break;
    case 'AppriseSettings':
      Object.assign(draft.apprise, { server: settings.server, mode: settings.mode, tags: settings.tags.join(', ') });
      break;
  }
  return draft;
}

export function storedOf(channel: NotificationChannel): Stored {
  const { settings } = channel;
  return {
    kind: channel.kind,
    headerNames: settings.__typename === 'WebhookSettings' ? settings.headerNames : [],
    ntfyToken: settings.__typename === 'NtfySettings' && settings.tokenStored,
    emailPassword: settings.__typename === 'EmailSettings' && settings.passwordStored,
    appriseMode: settings.__typename === 'AppriseSettings' ? settings.mode : null,
  };
}

/**
 * Empties every secret the draft holds, in place, so the caller can drop them from the DOM in the same task (F-8).
 * Header names stay: they are not secret, and K1 returns them.
 */
export function forgetSecrets(draft: ChannelDraft): void {
  draft.webhook.url = '';
  for (const header of draft.webhook.headers) header.value = '';
  draft.ntfy.topic = '';
  draft.ntfy.token.value = '';
  draft.email.password.value = '';
  draft.apprise.urls = '';
  draft.apprise.key = '';
}
