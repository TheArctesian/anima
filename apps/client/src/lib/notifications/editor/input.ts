// The editor's values turned into a `NotificationChannelInput`, or refused with fixed messages before anything is
// sent. The secret rule of K1 lives here: a secret the operator did not touch is `null` (the hub keeps it), a stored
// optional secret the operator removed is `""` (the hub clears it), and anything typed is sent as typed. A channel that
// moves to a new target needs every stored secret again (CR-14.1), so a secret meant for one host is never sent to
// another. URLs are https only and a server is an origin only (CR-14.6, CR-14.7). No message here ever repeats what
// was typed into a secret field (LOG-007, F-8).
import type { NotificationChannelInput } from '../api';
import { cleanOrigin, mailTarget } from '../channels/target';
import { templateFine, templateProblems } from '../template/template';
import type { ChannelDraft, HeaderDraft, SecretDraft, Stored } from './draft';

export type DraftField =
  | 'name'
  | 'webhook.url'
  | 'webhook.headers'
  | 'ntfy.server'
  | 'ntfy.topic'
  | 'ntfy.token'
  | 'email.host'
  | 'email.port'
  | 'email.from'
  | 'email.to'
  | 'email.password'
  | 'apprise.server'
  | 'apprise.urls'
  | 'apprise.key'
  | 'template.title'
  | 'template.body';

export interface DraftProblem {
  readonly field: DraftField;
  readonly message: string;
}

export type DraftCheck =
  | { readonly ok: true; readonly input: NotificationChannelInput }
  | { readonly ok: false; readonly problems: readonly DraftProblem[] };

export const NAME_MISSING = 'Enter a name.';
export const URL_WRONG = 'Enter the whole address, starting with https://.';
export const URL_MISSING = 'Enter the webhook URL.';
export const HEADERS_WRONG = 'Each header needs a name (letters, digits, -) and a value.';
export const TOPIC_WRONG = 'Use 1 to 64 letters, digits, - or _.';
export const HOST_WRONG = 'Enter a host name, for example smtp.example.com.';
export const PORT_WRONG = 'Use a port from 1 to 65535.';
export const ADDRESS_WRONG = 'Enter an email address.';
export const RECIPIENTS_WRONG = 'Enter one or more email addresses, separated by commas.';
export const APPRISE_URLS_WRONG = 'Enter one Apprise URL per line, for example discord://….';
export const APPRISE_KEY_WRONG = 'Use 1 to 128 letters, digits, - or _.';
export const SERVER_WRONG = 'Enter https:// and the host only, with no user name, path or query.';
export const AGAIN = 'The target changed: enter this again.';
export const AGAIN_OR_REMOVE = 'The target changed: enter this again, or remove it.';
export const HEADERS_AGAIN = 'The target changed: replace the headers with each value typed again, or remove them all.';

const HEADER_NAME = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
const TOPIC = /^[-_A-Za-z0-9]{1,64}$/;
const HOST = /^(?:[a-z0-9-]+(?:\.[a-z0-9-]+)*|\[[0-9a-f:.]+\])$/i;
const ADDRESS = /^[^\s@,]+@[^\s@,]+$/;
const APPRISE_URL = /^[a-z][a-z0-9+.-]*:\/\/\S+$/i;
const APPRISE_KEY = /^[A-Za-z0-9_-]{1,128}$/;

/** What was typed, else `""` to clear a stored optional secret, else `null` to keep what is stored (K1). */
export function secretValue(typed: string, clear: boolean): string | null {
  if (typed !== '') return typed;
  return clear ? '' : null;
}

/** The origin of an https URL, or `null`: webhook URLs are https only (CR-14.6). */
function httpsOrigin(text: string): string | null {
  try {
    const url = new URL(text);
    return url.protocol === 'https:' ? url.origin : null;
  } catch {
    // Not a URL; the caller says so in a fixed sentence.
    return null;
  }
}

/** Values separated by commas, spaces or lines. */
function listOf(text: string): string[] {
  return text.split(/[\s,]+/).filter((part) => part !== '');
}

type Problems = DraftProblem[];

function refuse(problems: Problems, field: DraftField, message: string): void {
  problems.push({ field, message });
}

/** Whether a saved channel would send somewhere new, so its stored secrets must come again (CR-14.1). */
function moved(stored: Stored | null, target: string | null): boolean {
  return stored !== null && (stored.target === null || target !== stored.target);
}

/** A stored optional secret on a new target: typed again, or removed (`""`); never kept (CR-14.1). */
function askAgain(problems: Problems, field: DraftField, held: boolean, secret: SecretDraft): void {
  if (held && secret.value === '' && !secret.clear) refuse(problems, field, AGAIN_OR_REMOVE);
}

/** The headers to send: `null` keeps the stored ones, `[]` removes them all, a list replaces them (CR-14.3). */
function headersValue(webhook: ChannelDraft['webhook'], stored: Stored | null, problems: Problems): HeaderDraft[] | null {
  if (!webhook.replaceHeaders) return null;
  const rows = webhook.headers.filter((header) => header.name.trim() !== '' || header.value !== '');
  if (rows.some((header) => !HEADER_NAME.test(header.name.trim()) || header.value === '')) refuse(problems, 'webhook.headers', HEADERS_WRONG);
  if (rows.length === 0) return stored === null ? null : [];
  return rows.map((header) => ({ name: header.name.trim(), value: header.value }));
}

function webhookInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const url = draft.webhook.url.trim();
  const origin = httpsOrigin(url);
  if (url === '' && stored === null) refuse(problems, 'webhook.url', URL_MISSING);
  else if (url !== '' && origin === null) refuse(problems, 'webhook.url', URL_WRONG);
  // An untouched URL keeps the stored one, and with it the target.
  const kept = (stored?.headerNames.length ?? 0) > 0 && !draft.webhook.replaceHeaders;
  if (origin !== null && moved(stored, origin) && kept) refuse(problems, 'webhook.headers', HEADERS_AGAIN);
  const headers = headersValue(draft.webhook, stored, problems);
  return { webhook: { url: secretValue(url, false), method: draft.webhook.method, headers } };
}

function ntfyInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const { ntfy } = draft;
  const server = cleanOrigin(ntfy.server.trim());
  const topic = ntfy.topic.trim();
  const away = moved(stored, server);
  if (server === null) refuse(problems, 'ntfy.server', SERVER_WRONG);
  if (topic === '' && away) refuse(problems, 'ntfy.topic', AGAIN);
  else if ((topic !== '' || stored === null) && !TOPIC.test(topic)) refuse(problems, 'ntfy.topic', TOPIC_WRONG);
  if (away) askAgain(problems, 'ntfy.token', stored?.ntfyToken === true, ntfy.token);
  const clear = stored?.ntfyToken === true && ntfy.token.clear;
  return {
    ntfy: { server: server ?? ntfy.server.trim(), topic: secretValue(topic, false), token: secretValue(ntfy.token.value, clear), priority: ntfy.priority },
  };
}

function emailInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const { email } = draft;
  const host = email.host.trim();
  const port = Number(email.port.trim());
  const from = email.from.trim();
  const to = listOf(email.to);
  if (!HOST.test(host)) refuse(problems, 'email.host', HOST_WRONG);
  if (!/^\d{1,5}$/.test(email.port.trim()) || port < 1 || port > 65_535) refuse(problems, 'email.port', PORT_WRONG);
  if (!ADDRESS.test(from)) refuse(problems, 'email.from', ADDRESS_WRONG);
  if (to.length === 0 || !to.every((address) => ADDRESS.test(address))) refuse(problems, 'email.to', RECIPIENTS_WRONG);
  if (moved(stored, mailTarget(host, port))) askAgain(problems, 'email.password', stored?.emailPassword === true, email.password);
  const username = email.username.trim();
  const clear = stored?.emailPassword === true && email.password.clear;
  return {
    email: { host, port, security: email.security, username: username === '' ? null : username, password: secretValue(email.password.value, clear), from, to },
  };
}

/** The URLs or key of the mode sent: typed, or kept when the hub holds them for this mode at this server (CR-14.1, CR-14.2). */
function appriseInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const { apprise } = draft;
  const server = cleanOrigin(apprise.server.trim());
  if (server === null) refuse(problems, 'apprise.server', SERVER_WRONG);
  const required = stored?.appriseMode !== apprise.mode || moved(stored, server);
  const urls = listOf(apprise.urls);
  const key = apprise.key.trim();
  const tags = listOf(apprise.tags);
  const base = { server: server ?? apprise.server.trim(), mode: apprise.mode, tags };
  // The other mode's secret is null: the hub deletes it when the mode changes (CR-14.2).
  if (apprise.mode === 'STATELESS_URLS') {
    if ((required || urls.length > 0) && (urls.length === 0 || !urls.every((url) => APPRISE_URL.test(url)))) refuse(problems, 'apprise.urls', APPRISE_URLS_WRONG);
    return { apprise: { ...base, urls: urls.length === 0 ? null : urls, key: null } };
  }
  if ((required || key !== '') && !APPRISE_KEY.test(key)) refuse(problems, 'apprise.key', APPRISE_KEY_WRONG);
  return { apprise: { ...base, urls: null, key: secretValue(key, false) } };
}

const SETTINGS_INPUT = { WEBHOOK: webhookInput, NTFY: ntfyInput, EMAIL: emailInput, APPRISE: appriseInput };

/**
 * The input to send, or every problem to show; nothing is sent while there is one. `stored` is `null` for a new
 * channel. `placeholders` are the hub's (D-21).
 */
export function checkDraft(draft: ChannelDraft, stored: Stored | null, placeholders: readonly string[]): DraftCheck {
  const problems: Problems = [];
  const name = draft.name.trim();
  if (name === '') refuse(problems, 'name', NAME_MISSING);
  const settings = SETTINGS_INPUT[draft.kind](draft, stored, problems);
  let template: NotificationChannelInput['template'] = null;
  if (draft.ownTemplate) {
    const found = templateProblems(draft.template, placeholders);
    if (found.title !== '') refuse(problems, 'template.title', found.title);
    if (found.body !== '') refuse(problems, 'template.body', found.body);
    if (templateFine(found)) template = { title: draft.template.title, body: draft.template.body };
  }
  if (problems.length > 0) return { ok: false, problems };
  return { ok: true, input: { name, onByDefault: draft.onByDefault, settings, template } };
}
