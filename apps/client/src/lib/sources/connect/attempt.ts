import type { ConnectAttempt } from './api';

// The words of the three-press GitHub connection, for each phase and each failure (PRJ-039, PRJ-041). Pure: the
// attempt in, the line and the actions out.

/** What the row shows for one attempt: a line, and which actions it offers. */
export interface AttemptView {
  readonly line: string;
  /** "Continue in browser": a new one-time link for the open attempt. */
  readonly continueInBrowser: boolean;
  readonly cancel: boolean;
  /** Where "Install on GitHub" goes; null: no such action. Only a github.com address. */
  readonly installUrl: string | null;
  /** Where "Delete the App on GitHub" goes; null: no such action. Only a github.com address. */
  readonly appSettingsUrl: string | null;
  /** "Connect GitHub": start a new attempt. */
  readonly start: boolean;
}

export const NO_ATTEMPT: AttemptView = { line: '', continueInBrowser: false, cancel: false, installUrl: null, appSettingsUrl: null, start: true };

const AGENT_STARTED = 'An AI agent started this.';
export const HUB_RESTARTED =
  'The hub restarted and dropped the connection. Delete any App you made on GitHub, then start again.';
export const TAB_BLOCKED = 'The tab did not open. Open this page in a browser signed in to Anima.ing and press Continue.';

/** On a hub that no browser can open: the flow is not offered (DEF-003; the gateway of R-239 will give the address). */
export const NO_BROWSER_ADDRESS =
  'Coming soon: Connect GitHub in your browser for this hub. Today a browser cannot open a page of this hub, so nothing is opened. If you made a GitHub App yourself, use its key file.';

const GITHUB = 'https://github.com/';
const TRY_AGAIN = 'Delete any new App on GitHub that starts with "anima-".';
const AT_GITHUB =
  'Waiting for you on GitHub. Press "Create GitHub App". If the name is taken, change it there and press again. GitHub may ask for your password first.';

/** An address the client opens for GitHub, or null: anything not on github.com is never opened. */
export function githubAddress(url: string | null): string | null {
  return url?.startsWith(GITHUB) === true ? url : null;
}

/** A hub page address the client may link to (http or https), or null. */
export function hubPageAddress(url: string): string | null {
  return /^https?:\/\//.test(url) ? url : null;
}

function app(attempt: ConnectAttempt): string {
  return attempt.appSlug === null ? 'The App' : `The App ${attempt.appSlug}`;
}

function failedLine(attempt: ConnectAttempt): string {
  const made = attempt.appSettingsUrl !== null;
  const slug = attempt.appSlug === null ? 'the App' : `the App ${attempt.appSlug}`;
  switch (attempt.failure) {
    case 'EXPIRED':
      return made
        ? `${app(attempt)} was not installed in time. Delete it on GitHub, then start again.`
        : 'The connection timed out. Press Connect GitHub.';
    case 'EXCHANGE_REFUSED':
      return `GitHub did not accept the reply from its page. Start again. ${TRY_AGAIN}`;
    case 'GITHUB_UNREACHABLE':
      return `The hub could not reach GitHub. Start again. ${TRY_AGAIN}`;
    case 'REPLY_UNUSABLE':
      return `GitHub's reply could not be used. Delete ${slug} on GitHub, then start again.`;
    case 'KEY_REFUSED':
      return `GitHub refused the new App's key. Delete ${slug} on GitHub, then start again.`;
    default:
      return attempt.message ?? 'The connection did not finish. Press Connect GitHub.';
  }
}

function cancelledLine(attempt: ConnectAttempt): string {
  if (attempt.appSettingsUrl === null) return 'Cancelled.';
  return `Cancelled. ${app(attempt)} is still on GitHub. Delete it there.`;
}

function connectedLine(attempt: ConnectAttempt): string {
  if (attempt.account?.pausedBy === 'EXTENSION_OFF') {
    return 'GitHub is connected. Set Projects on in Settings > Extensions to read it.';
  }
  return 'GitHub is connected. The first sync has started.';
}

function phaseView(attempt: ConnectAttempt): AttemptView {
  const waiting = { continueInBrowser: true, cancel: true, installUrl: null, appSettingsUrl: null, start: false };
  switch (attempt.phase) {
    case 'WAITING_FOR_BROWSER':
      return { ...waiting, line: 'Opening GitHub in your browser.' };
    case 'AT_GITHUB':
      return { ...waiting, line: AT_GITHUB };
    case 'APP_CREATED': {
      const owner = attempt.ownerLogin === null ? '' : ` for ${attempt.ownerLogin}`;
      return {
        ...waiting,
        continueInBrowser: false,
        installUrl: githubAddress(attempt.installUrl),
        line: `${app(attempt)} is made${owner}. Press Install on GitHub.`,
      };
    }
    case 'CONNECTED':
      return { ...NO_ATTEMPT, start: false, line: connectedLine(attempt) };
    case 'FAILED':
      return { ...NO_ATTEMPT, appSettingsUrl: githubAddress(attempt.appSettingsUrl), line: failedLine(attempt) };
    case 'CANCELLED':
      return { ...NO_ATTEMPT, appSettingsUrl: githubAddress(attempt.appSettingsUrl), line: cancelledLine(attempt) };
  }
}

/** The line and the actions for an attempt; an agent's start puts its words first. */
export function describeAttempt(attempt: ConnectAttempt): AttemptView {
  const view = phaseView(attempt);
  return attempt.startedBy === 'AGENT' ? { ...view, line: `${AGENT_STARTED} ${view.line}` } : view;
}

/** The row when the hub no longer holds an attempt the client had open. */
export const RESTARTED: AttemptView = { ...NO_ATTEMPT, line: HUB_RESTARTED };

/** The attempt is still going: the hub may drop it by restarting. */
export function isOpen(attempt: ConnectAttempt): boolean {
  return attempt.phase === 'WAITING_FOR_BROWSER' || attempt.phase === 'AT_GITHUB' || attempt.phase === 'APP_CREATED';
}
