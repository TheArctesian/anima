import { beforeEach, describe, expect, it, vi } from 'vitest';
import { BuilderShellError, desktopBuilderShell, releaseFilesFrom, toolCheckFrom } from './shell';

const core = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const events = vi.hoisted(() => ({ listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => core);
vi.mock('@tauri-apps/api/event', () => events);

const JOB = { commit: 'a'.repeat(40), target: 'LINUX_X86_64', version: '0.1.12' };
const KEY_TEXT = 'RWRTY0Iysecret-key-text';

beforeEach(() => {
  core.invoke.mockReset();
  core.isTauri.mockReset().mockReturnValue(true);
  events.listen.mockReset();
});

describe('the build steps (BLD-027)', () => {
  it('BLD-027: each step is a fixed command that carries only the job fields, never a program or a path', async () => {
    core.invoke.mockResolvedValue('Anima_0.1.12_amd64.AppImage');
    await desktopBuilderShell.checkout(JOB.commit);
    await desktopBuilderShell.runGates(JOB.commit);
    await desktopBuilderShell.bundle(JOB);
    await desktopBuilderShell.cancel();
    expect(core.invoke.mock.calls).toEqual([
      ['builder_checkout', { commit: JOB.commit }],
      ['builder_gates_run', { commit: JOB.commit }],
      ['builder_bundle', { job: JOB }],
      ['builder_cancel', {}],
    ]);
  });

  it('BLD-027, LOG-007: a refused step keeps only the known code, never the shell text', async () => {
    core.invoke.mockRejectedValue({ code: 'not-on-main', message: `secret ${KEY_TEXT}` });
    const error: unknown = await desktopBuilderShell.checkout(JOB.commit).catch((reason: unknown) => reason);
    expect(error).toBeInstanceOf(BuilderShellError);
    expect((error as BuilderShellError).code).toBe('not-on-main');
    expect(JSON.stringify(error) + String(error)).not.toContain(KEY_TEXT);
    core.invoke.mockRejectedValue({ code: 'run-anything' });
    await expect(desktopBuilderShell.runGates(JOB.commit)).rejects.toMatchObject({ code: 'failed' });
  });

  it('BLD-026: outside the desktop app no step reaches a shell', async () => {
    core.isTauri.mockReturnValue(false);
    await expect(desktopBuilderShell.checkTools()).rejects.toMatchObject({ code: 'not-desktop' });
    await expect(desktopBuilderShell.onLine(() => undefined)).resolves.toBeTypeOf('function');
    expect(core.invoke).not.toHaveBeenCalled();
    expect(events.listen).not.toHaveBeenCalled();
  });

  it('BLD-048: output lines arrive from the builder-line event; other payloads are ignored', async () => {
    let deliver: (event: { payload: unknown }) => void = () => undefined;
    events.listen.mockImplementation((_name: string, handler: typeof deliver) => {
      deliver = handler;
      return Promise.resolve(() => undefined);
    });
    const lines: string[] = [];
    await desktopBuilderShell.onLine((line) => lines.push(line));
    deliver({ payload: { step: 'gates', text: 'cargo fmt --check' } });
    deliver({ payload: { step: 'gates' } });
    deliver({ payload: 'raw' });
    expect(events.listen).toHaveBeenCalledWith('builder-line', expect.any(Function));
    expect(lines).toEqual(['cargo fmt --check']);
  });
});

describe('the upload (BLD-060, BLD-063)', () => {
  const FILE = { name: 'Anima_0.1.12_amd64.AppImage', sha256: 'a'.repeat(64), size: 5 };
  const UPDATER = FILE.name;

  it('BLD-063, R-117: the release files and the unsigned updater file name are read for the job, each field checked', async () => {
    core.invoke.mockResolvedValue({ files: [FILE], updater: UPDATER });
    await expect(desktopBuilderShell.releaseFiles(JOB)).resolves.toEqual({ files: [FILE], updater: UPDATER });
    expect(core.invoke).toHaveBeenCalledWith('builder_release_files', { target: JOB.target, version: JOB.version });
  });

  it('BLD-063: a file without a lower-case SHA-256 or a positive whole size is a bundle failure', () => {
    for (const file of [
      { ...FILE, sha256: 'A'.repeat(64) },
      { ...FILE, sha256: 'a'.repeat(63) },
      { ...FILE, size: 0 },
      { ...FILE, size: 1.5 },
      { ...FILE, name: 3 },
    ]) {
      expect(() => releaseFilesFrom({ files: [file], updater: UPDATER })).toThrow(BuilderShellError);
    }
    expect(() => releaseFilesFrom({ files: [], updater: UPDATER })).toThrow(BuilderShellError);
    for (const updater of ['', { file: FILE.name, signature: 'c2ln' }, undefined]) {
      expect(() => releaseFilesFrom({ files: [FILE], updater })).toThrow(BuilderShellError);
    }
  });

  it('BLD-060: an upload carries the job and the presigned upload, and no bytes or text of the web view', async () => {
    core.invoke.mockResolvedValue(null);
    const upload = { name: 'SHA256SUMS', url: 'https://r2.example/x', headers: [{ name: 'content-length', value: '5' }] };
    await desktopBuilderShell.upload(JOB, { ...upload, extra: 'x' } as typeof upload);
    await desktopBuilderShell.upload(JOB, { ...upload, name: FILE.name });
    expect(core.invoke.mock.calls).toEqual([
      ['builder_upload', { job: { target: JOB.target, version: JOB.version, upload } }],
      ['builder_upload', { job: { target: JOB.target, version: JOB.version, upload: { ...upload, name: FILE.name } } }],
    ]);
  });
});

describe('toolCheckFrom (BLD-028)', () => {
  it('BLD-028: reads the platform and the missing tools', () => {
    expect(toolCheckFrom({ os: 'LINUX', arch: 'X86_64', missing: ['cargo-nextest', 'postgres', 'updater-public-key'] })).toEqual({
      os: 'LINUX',
      arch: 'X86_64',
      missing: ['cargo-nextest', 'postgres', 'updater-public-key'],
    });
  });

  it.each([
    null,
    'LINUX',
    { os: 'WINDOWS', arch: 'X86_64', missing: [] },
    { os: 'LINUX', arch: 'MIPS', missing: [] },
    { os: 'LINUX', arch: 'X86_64', missing: ['compiler'] },
    // R-117: the builder holds no updater key, so a key is never a missing tool.
    { os: 'LINUX', arch: 'X86_64', missing: ['updater-key'] },
    { os: 'LINUX', arch: 'X86_64', missing: ['updater-key-locked'] },
  ])(
    'BLD-028: refuses %j',
    (answer) => {
      expect(() => toolCheckFrom(answer)).toThrow(BuilderShellError);
    },
  );
});
