import { describe, expect, it } from 'vitest';
import { installCommand, type CommandInput, type InstallCommand } from './command';
import type { AgentRelease, AgentTarget } from './release';

const SHA = {
  'linux-x86_64': 'a'.repeat(64),
  'linux-aarch64': 'b'.repeat(64),
  'macos-aarch64': 'c'.repeat(64),
} as const;
const RELEASE: AgentRelease = { version: '1.2.3', sha256: SHA };
const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';
const ORIGIN = 'https://hub.anima.ing';
const TARGETS: readonly AgentTarget[] = ['linux-x86_64', 'linux-aarch64', 'macos-aarch64'];

const input = (change: Partial<CommandInput> = {}): CommandInput => ({ release: RELEASE, target: 'linux-x86_64', code: CODE, origin: ORIGIN, ...change });

function made(change: Partial<CommandInput> = {}): InstallCommand {
  const result = installCommand(input(change));
  if (!result.ok) throw new Error(`refused: ${result.refusal}`);
  return result.command;
}

/** The command with every single-quoted span removed: what the shell reads as syntax. */
function outsideQuotes(text: string): string {
  return text.replace(/'(?:[^']|'\\'')*'/g, '');
}

describe('installCommand with a release (MET-019, D-10 b)', () => {
  it.each(TARGETS)('MET-019: %s downloads that target, checks the pinned sha256 and stops when the check fails', (target) => {
    const { text } = made({ target });
    const verify = target.startsWith('macos') ? 'shasum -a 256 -c -' : 'sha256sum -c -';
    expect(text).toContain(`'https://releases.anima.ing/agent/v1.2.3/anima-agent-${target}'`);
    expect(text).toContain(`'${SHA[target]}'`);
    expect(text).toContain(`| ${verify} && `);
    // The check sits between the download and the install, joined by && so a failure ends the chain.
    expect(text.indexOf('curl')).toBeLessThan(text.indexOf(verify));
    expect(text.indexOf(verify)).toBeLessThan(text.indexOf('install -m 0755'));
    expect(text.indexOf('install -m 0755')).toBeLessThan(text.indexOf(' enrol '));
    expect(text.indexOf(' enrol ')).toBeLessThan(text.indexOf('anima-agent install'));
    expect(outsideQuotes(text)).not.toMatch(/;|\|\|/);
  });

  it('MET-019: the code goes to enrol on stdin, never on the command line of the agent', () => {
    const { text } = made();
    expect(text).toContain(`printf '%s\\n' '${CODE}' | ~/.local/bin/anima-agent enrol --hub '${ORIGIN}' -`);
    expect(text.split(CODE)).toHaveLength(2);
    expect(text).toContain('~/.local/bin/anima-agent install)');
  });

  it('MET-019: an interrupt or hangup ends the chain and still removes the temp file', () => {
    const { text } = made();
    expect(text).toContain(`trap 'rm -f "$t"' EXIT && trap 'exit 130' INT HUP TERM && `);
  });

  it('MET-019: curl accepts only https, also after a redirect', () => {
    expect(made().text).toContain("curl -fsSL --proto '=https' --proto-redir '=https' 'https://releases.anima.ing/");
  });

  it('MET-019: Linux verifies with sha256sum and macOS with shasum, not the other way', () => {
    expect(made({ target: 'linux-aarch64' }).text).not.toContain('shasum');
    expect(made({ target: 'macos-aarch64' }).text).not.toContain('sha256sum');
  });

  it('MET-019: a release without the chosen target gives the enrol-only command', () => {
    const partial: AgentRelease = { version: '1.2.3', sha256: { 'linux-x86_64': SHA['linux-x86_64'] } };
    expect(made({ release: partial, target: 'macos-aarch64' }).kind).toBe('enrol-only');
  });
});

describe('installCommand without a release (D-10 c)', () => {
  it('MET-019: development shows only the enrol command, with a note about pasting the code', () => {
    const command = made({ release: null, target: null });
    expect(command.kind).toBe('enrol-only');
    expect(command.text).toBe(`anima-agent enrol --hub '${ORIGIN}' -`);
    expect(command.kind === 'enrol-only' && command.note).toContain('paste the code at its prompt');
  });

  it('MET-019: a release without the computer chosen yet says to choose it, not that no download exists', () => {
    const command = made({ target: null });
    expect(command.kind === 'enrol-only' && command.note).toContain('Choose the computer');
  });

  it('MET-019: the code is not in the development command', () => {
    expect(made({ release: null, target: null }).text).not.toContain(CODE);
  });
});

describe('installCommand refuses hostile input (MET-019, LOG-007)', () => {
  const HOSTILE_CODES = [
    `${CODE}'; rm -rf ~ #`,
    `anima_enrol_$(touch x)${'a'.repeat(10)}`,
    `${CODE}\n`,
    `${CODE} `,
    `${CODE}\`id\``,
    'anima_enrol_short',
    `${CODE}A`,
    CODE.replace('anima_enrol_', 'anima_agent_'),
    `anima_enrol_${'+'.repeat(22)}`,
    `anima_enrol_${'='.repeat(22)}`,
    '',
  ];
  const HOSTILE_ORIGINS = [
    "https://hub.test'; id; '",
    'https://hub.test/path',
    'https://hub.test/',
    'https://hub.test?x=1',
    'https://user@hub.test',
    'https://hub.test:99999x',
    'https://hub.test:8443',
    'https://hub.test:0443',
    'https://hub.test:',
    'https://hub test',
    'https://hub.test\n',
    'https://$(id).test',
    'https://hub.test`id`',
    'http://hub.test',
    'hub.test',
    'https://',
    '',
  ];

  it.each(HOSTILE_CODES)('MET-019: refuses the code %j', (code) => {
    expect(installCommand(input({ code }))).toEqual({ ok: false, refusal: 'code' });
  });

  it.each(HOSTILE_ORIGINS)('MET-019: refuses the origin %j', (origin) => {
    expect(installCommand(input({ origin }))).toEqual({ ok: false, refusal: 'origin' });
    expect(installCommand(input({ origin, release: null, target: null }))).toEqual({ ok: false, refusal: 'origin' });
  });

  it('MET-020: the origins the agent takes give a command: no port, or port 443 written out', () => {
    for (const origin of ['https://hub.test', 'https://hub.test:443']) {
      expect(made({ origin }).text).toContain(`--hub '${origin}' -`);
    }
  });

  it('MET-019: the code and the origin stand only inside single quotes', () => {
    const { text } = made({ origin: 'https://hub.home.arpa:443' });
    expect(text).toContain("'https://hub.home.arpa:443'");
    const bare = outsideQuotes(text);
    expect(bare).not.toContain(CODE);
    expect(bare).not.toContain('hub.home.arpa');
    expect(bare).not.toContain('anima_enrol_');
  });

  it('MET-019: a quote in a release value cannot leave its single quotes', () => {
    const release: AgentRelease = { version: "1.0.0'; id; '", sha256: SHA };
    const { text } = made({ release });
    expect(outsideQuotes(text)).not.toContain('id;');
    expect(outsideQuotes(text)).not.toContain('1.0.0');
  });
});
