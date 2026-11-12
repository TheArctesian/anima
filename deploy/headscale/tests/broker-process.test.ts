// The broker process edge (R-055, VP-8): the request line from the socket, the entry points and
// the state and config files.
import { afterEach, describe, expect, it } from 'bun:test';
import { mkdtempSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { readLine } from '../broker/main.ts';
import { MAX_REQUEST_BYTES } from '../broker/protocol.ts';
import { EMPTY_STATE, maxNodesOf, parseState, readState, StateError, writeState } from '../broker/state.ts';

const MAIN = new URL('../broker/main.ts', import.meta.url).pathname;
const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

function stream(chunks: readonly string[], end = true): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) controller.enqueue(new TextEncoder().encode(chunk));
      if (end) controller.close();
    },
  });
}

describe('readLine', () => {
  it('VP-7: reads one line across chunks and ignores what follows', async () => {
    expect(await readLine(stream(['{"verb":', '"nodes"}\n{"verb":"register"}\n']), MAX_REQUEST_BYTES + 1, 1000)).toBe('{"verb":"nodes"}');
  });

  it('VP-7: EOF with nothing read is no request; a last line without a newline still counts', async () => {
    expect(await readLine(stream([]), MAX_REQUEST_BYTES + 1, 1000)).toBeNull();
    expect(await readLine(stream(['{"verb":"nodes"}']), MAX_REQUEST_BYTES + 1, 1000)).toBe('{"verb":"nodes"}');
  });

  it('VP-7: stops reading past the limit, so the parser refuses the line', async () => {
    const line = await readLine(stream(['x'.repeat(600), 'x'.repeat(600), 'x'.repeat(600)], false), MAX_REQUEST_BYTES + 1, 1000);
    expect(line?.length).toBe(1200);
  });

  it('VP-7: a client that sends no newline in time gets no reply', async () => {
    expect(await readLine(stream(['{"verb":'], false), MAX_REQUEST_BYTES + 1, 50)).toBeNull();
  });
});

describe('entry points', () => {
  it('VP-8: EOF with no line exits with no reply', () => {
    const run = Bun.spawnSync(['bun', MAIN], { stdin: new Uint8Array() });
    expect(run.exitCode).toBe(0);
    expect(run.stdout.toString()).toBe('');
    expect(run.stderr.toString()).toBe('anima-broker verb=- outcome=no-request node=- fixes=0\n');
  });

  it('VP-8: an unknown command is a usage error', () => {
    expect(Bun.spawnSync(['bun', MAIN, 'open-device']).exitCode).toBe(64);
    expect(Bun.spawnSync(['bun', MAIN, 'check', 'extra']).exitCode).toBe(64);
  });
});

describe('broker files', () => {
  it('5.1: broker.conf holds max_nodes and nothing else', () => {
    expect(maxNodesOf('# comment\nmax_nodes=12\n')).toBe(12);
    for (const text of ['', 'max_nodes=0', 'max_nodes=12\nper_day=9', 'max_nodes=-1', 'max_nodes = 12']) expect(() => maxNodesOf(text)).toThrow(StateError);
  });

  it('5.1: a state the broker did not write is an error', () => {
    const bad = ['', '[]', '{"keys":[]}', '{"keys":[],"registrations":[],"issued":[{"keyId":"1"}],"pendingRegister":null}', '{"keys":[],"registrations":[],"issued":[],"pendingRegister":6}', '{"keys":[],"registrations":[],"issued":[],"pendingRegister":{"before":["1"],"node":null}}'];
    for (const text of bad) expect(() => parseState(text)).toThrow(StateError);
    expect(parseState(JSON.stringify(EMPTY_STATE))).toEqual(EMPTY_STATE);
  });

  it('5.1: the state is written whole, mode 0600, and read back', async () => {
    const folder = mkdtempSync(join(tmpdir(), 'anima-broker-'));
    folders.push(folder);
    const path = join(folder, 'state.json');
    expect(await readState(path)).toEqual(EMPTY_STATE);
    const state = { ...EMPTY_STATE, keys: ['2026-11-10T00:00:00.000Z'], pendingRegister: { before: [1, 2], node: 6 } };
    await writeState(path, state);
    expect(await readState(path)).toEqual(state);
    expect(statSync(path).mode & 0o777).toBe(0o600);
  });
});
