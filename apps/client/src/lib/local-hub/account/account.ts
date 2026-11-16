import { localHubMessage, type LocalHubProblem } from '$lib/desktop/local-hub/shell';

/** The operator name rule of `anima-hub operator` (`docs/setup/local-hub.md` Section 7). */
const NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/;

/** The shortest password the hub's `operator set` rules take. */
export const PASSWORD_MIN = 12;

/** What the account step asks for. The password and its repeat are secrets (LOG-007). */
export interface AccountForm {
  readonly name: string;
  readonly password: string;
  readonly repeat: string;
}

export type AccountField = keyof AccountForm;

export type AccountProblems = Partial<Record<AccountField, string>>;

/** The problems the client sees before anything leaves the page; empty when the form is good (FRN-003). */
export function checkAccount(form: AccountForm): AccountProblems {
  const problems: AccountProblems = {};
  if (!NAME.test(form.name)) {
    problems.name = 'Use 1 to 64 letters, digits, dots, underscores or hyphens, starting with a letter or a digit.';
  }
  if (form.password.length < PASSWORD_MIN) problems.password = `Use at least ${String(PASSWORD_MIN)} characters.`;
  else if (form.repeat !== form.password) problems.repeat = 'The two passwords are not the same.';
  return problems;
}

const FIELDS: readonly AccountField[] = ['name', 'password', 'repeat'];

/** The first field in page order with a problem, for focus. */
export function firstProblem(problems: AccountProblems): AccountField | undefined {
  return FIELDS.find((field) => problems[field] !== undefined);
}

/** A TOTP secret for the eye: groups of four characters, so that it can be typed into an authenticator by hand. */
export function groupedSecret(secret: string): string {
  return secret.match(/.{1,4}/g)?.join(' ') ?? '';
}

/** TOTP changes its code every 30 seconds (RFC 6238). */
export const TOTP_STEP_S = 30;

/** Whole seconds until the authenticator shows its next code, from 1 to 30 (FRN-004). */
export function secondsToNextCode(now: Date): number {
  const seconds = Math.floor(now.getTime() / 1000);
  return TOTP_STEP_S - (seconds % TOTP_STEP_S);
}

const ACCOUNT_TEXT: Partial<Record<LocalHubProblem, string>> = {
  refused: 'The code did not match. Wait for a new code and try again.',
  // For 60 s after the person cancels the replace dialog the shell shows no dialog and answers `busy` (F-9).
  busy: 'Wait a minute before you try again.',
  argument: 'The hub refused the name: use 1 to 64 letters, digits, dots, underscores or hyphens, starting with a letter or a digit.',
};

/** The fixed text for a refused account change (FRN-004, FRN-007). */
export function accountFailure(problem: LocalHubProblem): string {
  return ACCOUNT_TEXT[problem] ?? localHubMessage(problem);
}
