// The editor's values turned into a `NotificationChannelInput`, or refused with fixed messages before anything is
// sent. The secret rule of K1 lives here: a secret the operator did not touch is `null` (the hub keeps it), a stored
// optional secret the operator removed is `""` (the hub clears it), and anything typed is sent as typed. No message
// here ever repeats what was typed into a secret field (LOG-007, F-8).
import type { NotificationChannelInput } from '../api';
import { templateFine, templateProblems } from '../template/template';
import type { ChannelDraft, HeaderDraft, Stored } from './draft';

export type DraftField =
  | 'name'
  | 'webhook.url'
  | 'webhook.headers'
  | 'ntfy.server'
  | 'ntfy.topic'
  | 'email.host'
  | 'email.port'
  | 'email.from'
  | 'email.to'
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

function isHttpUrl(text: string): boolean {
  try {
    const { protocol } = new URL(text);
    return protocol === 'https:' || protocol === 'http:';
  } catch {
    // Not a URL; the caller says so in a fixed sentence.
    return false;
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

/** The headers to send: `null` keeps the stored ones, `[]` removes them all, a list replaces them (K1). */
function headersValue(webhook: ChannelDraft['webhook'], stored: Stored | null, problems: Problems): HeaderDraft[] | null {
  if (!webhook.replaceHeaders) return null;
  const rows = webhook.headers.filter((header) => header.name.trim() !== '' || header.value !== '');
  if (rows.some((header) => !HEADER_NAME.test(header.name.trim()) || header.value === '')) refuse(problems, 'webhook.headers', HEADERS_WRONG);
  if (rows.length === 0) return stored === null ? null : [];
  return rows.map((header) => ({ name: header.name.trim(), value: header.value }));
}

function webhookInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const url = draft.webhook.url.trim();
  if (url === '' && stored === null) refuse(problems, 'webhook.url', URL_MISSING);
  else if (url !== '' && !isHttpUrl(url)) refuse(problems, 'webhook.url', URL_WRONG);
  const headers = headersValue(draft.webhook, stored, problems);
  return { webhook: { url: secretValue(url, false), method: draft.webhook.method, headers } };
}

function ntfyInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const { ntfy } = draft;
  const server = ntfy.server.trim();
  const topic = ntfy.topic.trim();
  if (!isHttpUrl(server)) refuse(problems, 'ntfy.server', URL_WRONG);
  if (topic !== '' || stored === null) {
    if (!TOPIC.test(topic)) refuse(problems, 'ntfy.topic', TOPIC_WRONG);
  }
  const clear = stored?.ntfyToken === true && ntfy.token.clear;
  return { ntfy: { server, topic: secretValue(topic, false), token: secretValue(ntfy.token.value, clear), priority: ntfy.priority } };
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
  const username = email.username.trim();
  const clear = stored?.emailPassword === true && email.password.clear;
  return {
    email: { host, port, security: email.security, username: username === '' ? null : username, password: secretValue(email.password.value, clear), from, to },
  };
}

function appriseInput(draft: ChannelDraft, stored: Stored | null, problems: Problems): NotificationChannelInput['settings'] {
  const { apprise } = draft;
  const server = apprise.server.trim();
  if (!isHttpUrl(server)) refuse(problems, 'apprise.server', URL_WRONG);
  // A mode the hub holds nothing for needs its URLs or key now; the mode it holds keeps them when left empty.
  const required = stored?.appriseMode !== apprise.mode;
  const urls = listOf(apprise.urls);
  const key = apprise.key.trim();
  const tags = listOf(apprise.tags);
  if (apprise.mode === 'STATELESS_URLS') {
    if ((required || urls.length > 0) && (urls.length === 0 || !urls.every((url) => APPRISE_URL.test(url)))) refuse(problems, 'apprise.urls', APPRISE_URLS_WRONG);
    return { apprise: { server, mode: apprise.mode, urls: urls.length === 0 ? null : urls, key: null, tags } };
  }
  if ((required || key !== '') && !APPRISE_KEY.test(key)) refuse(problems, 'apprise.key', APPRISE_KEY_WRONG);
  return { apprise: { server, mode: apprise.mode, urls: null, key: secretValue(key, false), tags } };
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
