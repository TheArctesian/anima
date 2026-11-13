import { describe, expect, it } from 'vitest';
import type { CommandLineResult, CommandLineStatus } from '$lib/desktop/command-line/shell';
import { canRemove, describeCommandLine, installTarget, profileAdvice, refusalText } from '../command-line';
import { CommandLineTool, type CommandLineShell } from '../command-line.svelte';

const NOT_INSTALLED: CommandLineStatus = {
  state: 'not-installed',
  folder: '/Users/ann/.local/bin',
  path: '/Users/ann/.local/bin/anima',
  onPath: true,
  create: false,
  profileLine: null,
};
const INSTALLED: CommandLineStatus = { state: 'installed', path: '/Users/ann/.local/bin/anima' };
const ELSEWHERE: CommandLineStatus = {
  state: 'elsewhere',
  folder: '/Users/ann/bin',
  path: '/Users/ann/bin/anima',
  pointsAt: '/Users/ann/Downloads/Anima.app/Contents/MacOS/anima',
};

describe('the row words (INS-006)', () => {
  it('INS-006: each state is said in words with its path', () => {
    expect(describeCommandLine(NOT_INSTALLED)).toBe('Not installed. Install puts anima at /Users/ann/.local/bin/anima.');
    expect(describeCommandLine({ ...NOT_INSTALLED, create: true })).toContain('makes the folder /Users/ann/.local/bin');
    expect(describeCommandLine(INSTALLED)).toBe('Installed at /Users/ann/.local/bin/anima.');
    expect(describeCommandLine(ELSEWHERE)).toBe(
      '/Users/ann/bin/anima points at /Users/ann/Downloads/Anima.app/Contents/MacOS/anima, not this app. Install points it at this app.',
    );
    expect(describeCommandLine({ state: 'unavailable', reason: 'disk-image' })).toContain('Move Anima to Applications');
  });

  it('INS-009: the profile line is shown only when the folder is not on the PATH', () => {
    expect(profileAdvice(NOT_INSTALLED)).toBeNull();
    const line = 'export PATH="$HOME/.local/bin:$PATH"';
    expect(profileAdvice({ ...NOT_INSTALLED, onPath: false, profileLine: line })).toEqual({ folder: '/Users/ann/.local/bin', line });
  });

  it('INS-006: Install writes to the proposed path or re-points another app\'s link; Remove only where Anima made one', () => {
    expect(installTarget(NOT_INSTALLED)).toEqual({ folder: '/Users/ann/.local/bin', path: '/Users/ann/.local/bin/anima' });
    expect(installTarget(ELSEWHERE)).toEqual({ folder: '/Users/ann/bin', path: '/Users/ann/bin/anima' });
    expect(installTarget(INSTALLED)).toBeNull();
    expect(installTarget(null)).toBeNull();
    expect([NOT_INSTALLED, INSTALLED, ELSEWHERE, null].map(canRemove)).toEqual([false, true, true, false]);
  });

  it('INS-006: a refusal says what is in the way and where', () => {
    const link = refusalText('occupied', { kind: 'link', linkTarget: '/opt/x/anima' }, '/a/anima');
    expect(link).toContain('There is a link at /a/anima to /opt/x/anima.');
    expect(refusalText('occupied', { kind: 'file', linkTarget: null }, '/a/anima')).toContain('There is a file at /a/anima.');
    expect(refusalText('unwritable', null, '/a/anima')).toBe('Anima could not write to the folder.');
  });
});

function shellOf(answers: Partial<Record<keyof CommandLineShell, CommandLineResult>>): CommandLineShell & { installs: string[] } {
  const installs: string[] = [];
  const ok = (status: CommandLineStatus): CommandLineResult => ({ ok: true, status });
  return {
    installs,
    read: () => Promise.resolve(answers.read ?? ok(NOT_INSTALLED)),
    install: (folder) => {
      installs.push(folder);
      return Promise.resolve(answers.install ?? ok(INSTALLED));
    },
    remove: () => Promise.resolve(answers.remove ?? ok(NOT_INSTALLED)),
  };
}

describe('CommandLineTool (INS-004 to INS-006)', () => {
  it('INS-004: Install asks first, then sends the proposed folder unchanged', async () => {
    const shell = shellOf({});
    const tool = new CommandLineTool(shell);
    await tool.load();
    tool.askInstall();
    expect(tool.confirming).toEqual({ folder: '/Users/ann/.local/bin', path: '/Users/ann/.local/bin/anima' });
    expect(shell.installs).toEqual([]);
    await tool.confirm();
    expect(shell.installs).toEqual(['/Users/ann/.local/bin']);
    expect(tool.status).toEqual(INSTALLED);
    expect(tool.confirming).toBeNull();
  });

  it('INS-004: Cancel installs nothing', async () => {
    const shell = shellOf({});
    const tool = new CommandLineTool(shell);
    await tool.load();
    tool.askInstall();
    tool.cancel();
    await tool.confirm();
    expect(shell.installs).toEqual([]);
  });

  it('INS-006: a refused install shows why and reads the state again', async () => {
    const refused: CommandLineResult = { ok: false, problem: 'occupied', occupant: { kind: 'file', linkTarget: null } };
    const tool = new CommandLineTool(shellOf({ install: refused, read: { ok: true, status: ELSEWHERE } }));
    tool.status = NOT_INSTALLED;
    tool.askInstall();
    await tool.confirm();
    expect(tool.problem).toContain('There is a file at /Users/ann/.local/bin/anima.');
    expect(tool.status).toEqual(ELSEWHERE);
  });
});
