import type { AgentRelease, AgentTarget } from './release';

/** Why no command was made: the code or the hub origin is not what the hub issues (MET-019). */
export type CommandRefusal = 'code' | 'origin';

export type InstallCommand =
  /** Download, verify, install, enrol, start. */
  | { readonly kind: 'download'; readonly text: string }
  /** Development: the agent is already on the computer; the operator pastes the code at its prompt. */
  | { readonly kind: 'enrol-only'; readonly text: string; readonly note: string };

export type CommandResult = { readonly ok: true; readonly command: InstallCommand } | { readonly ok: false; readonly refusal: CommandRefusal };

export interface CommandInput {
  readonly release: AgentRelease | null;
  readonly target: AgentTarget | null;
  /** The enrolment code; it is read only to put it between single quotes of a download command. */
  readonly code: string;
  readonly origin: string;
}

/** K8 Section 3: `anima_enrol_` and the unpadded base64url of 16 bytes, which is 22 characters. */
const CODE = /^anima_enrol_[A-Za-z0-9_-]{22}$/;
/**
 * An https origin as the agent takes it (MET-020): a host (letters, digits, dots, hyphens) on port 443, written or not; no
 * other port, path, query, user or quote. The agent refuses any other port, so a command with one could only fail.
 */
const ORIGIN = /^https:\/\/[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?(?::443)?$/;

const DOWNLOAD_ROOT = 'https://releases.anima.ing/agent';
const AGENT = '~/.local/bin/anima-agent';

/** POSIX single quotes; a quote inside the text is closed, escaped and reopened, so no text leaves them. */
function quote(text: string): string {
  return `'${text.replaceAll("'", `'\\''`)}'`;
}

const PASTE = 'Run this where the agent is installed and paste the code at its prompt. The code works once.';
const NO_RELEASE_NOTE = `No download is published for this build. ${PASTE}`;
const NO_TARGET_NOTE = `Choose the computer above to get the full install command. ${PASTE}`;
const NO_BINARY_NOTE = `No download is published for this computer. ${PASTE}`;

/** The command that installs and enrols the agent on the device the code was made for (MET-019, D-10 b). Pure: nothing is read from the page. */
export function installCommand({ release, target, code, origin }: CommandInput): CommandResult {
  if (!ORIGIN.test(origin)) return { ok: false, refusal: 'origin' };
  const sha256 = release !== null && target !== null ? release.sha256[target] : undefined;
  if (release === null || target === null || sha256 === undefined) {
    const note = release === null ? NO_RELEASE_NOTE : target === null ? NO_TARGET_NOTE : NO_BINARY_NOTE;
    return { ok: true, command: { kind: 'enrol-only', text: `anima-agent enrol --hub ${quote(origin)} -`, note } };
  }
  if (!CODE.test(code)) return { ok: false, refusal: 'code' };
  const verify = target.startsWith('macos') ? 'shasum -a 256 -c -' : 'sha256sum -c -';
  const url = `${DOWNLOAD_ROOT}/v${release.version}/anima-agent-${target}`;
  const steps = [
    't=$(mktemp)',
    `trap 'rm -f "$t"' EXIT`,
    `trap 'exit 130' INT HUP TERM`,
    `curl -fsSL --proto '=https' --proto-redir '=https' ${quote(url)} -o "$t"`,
    `printf '%s  %s\\n' ${quote(sha256)} "$t" | ${verify}`,
    'mkdir -p ~/.local/bin',
    `install -m 0755 "$t" ${AGENT}`,
    `printf '%s\\n' ${quote(code)} | ${AGENT} enrol --hub ${quote(origin)} -`,
    `${AGENT} install`,
  ];
  return { ok: true, command: { kind: 'download', text: `(${steps.join(' && ')})` } };
}

/** The sentence that says why there is no command. */
export function refusalMessage(refusal: CommandRefusal): string {
  return refusal === 'origin'
    ? 'The hub address is not a plain https origin, so no install command is shown.'
    : 'The hub made a code in a form the agent does not accept, so no install command is shown.';
}
