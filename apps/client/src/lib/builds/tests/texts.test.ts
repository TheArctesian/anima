import { describe, expect, it } from 'vitest';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';
import { builderIdKey, builderIdStore } from '../desktop.svelte';
import { builderText, statusText, thisPlatformText, TOOL_NAMES } from '../row';
import { JOB } from './builds.fixture';

function address(host: string): HubAddress {
  const parsed = parseHubAddress(host);
  if (!parsed.ok) throw new Error('bad test address');
  return parsed.value;
}

class MemoryStorage {
  readonly entries = new Map<string, string>();
  getItem = (key: string) => this.entries.get(key) ?? null;
  setItem = (key: string, value: string) => void this.entries.set(key, value);
  removeItem = (key: string) => void this.entries.delete(key);
}

describe('the builder ID store (BLD-025)', () => {
  it('BLD-025: keeps one builder ID per hub, and never the token', () => {
    const storage = new MemoryStorage();
    const a = builderIdStore(storage as unknown as Storage, address('hub-a.example'));
    const b = builderIdStore(storage as unknown as Storage, address('hub-b.example'));
    a.write('b-1');
    expect(a.read()).toBe('b-1');
    expect(b.read()).toBeNull();
    expect([...storage.entries.keys()]).toEqual([builderIdKey(address('hub-a.example'))]);
    a.clear();
    expect(a.read()).toBeNull();
  });

  it('BLD-025: a storage that refuses to be read holds no ID', () => {
    const refusing = { getItem: () => { throw new Error('denied'); } } as unknown as Storage;
    expect(builderIdStore(refusing, address('hub-a.example')).read()).toBeNull();
  });
});

describe('the Builds row texts (BLD-020)', () => {
  it('BLD-020: says what the builder does', () => {
    expect(statusText({ kind: 'building', job: JOB, step: 'gates' })).toBe('Building 0.1.12: running the gates.');
    expect(statusText({ kind: 'waiting' })).toBe('Waiting for a release to build.');
    expect(statusText({ kind: 'missing', tools: ['git'] })).toBe('A tool is missing, so this computer claims no build.');
  });

  it('I-4: says what turning building on or off is doing while it runs', () => {
    expect(builderText(false, true, { kind: 'stopped' })).toBe('Enrolling…');
    expect(builderText(true, true, { kind: 'waiting' })).toBe('Turning off…');
    expect(builderText(true, false, { kind: 'waiting' })).toBe('Waiting for a release to build.');
    expect(builderText(false, false, { kind: 'waiting' })).toBe('Off.');
  });

  it('BLD-028, R-117: names cargo-nextest among the tools, and no updater key', () => {
    expect(TOOL_NAMES['cargo-nextest']).toBe('cargo-nextest');
    expect(Object.keys(TOOL_NAMES).some((tool) => tool.startsWith('updater-key'))).toBe(false);
  });

  it('BLD-028: a missing darwin Rust target names the target and the rustup command to add it', () => {
    expect(TOOL_NAMES['rust-target-x86_64-apple-darwin']).toContain('rustup target add x86_64-apple-darwin');
    expect(TOOL_NAMES['rust-target-aarch64-apple-darwin']).toContain('rustup target add aarch64-apple-darwin');
  });

  it('BLD-020: names the platform once the shell has answered', () => {
    expect(thisPlatformText(null, false)).toBe('');
    expect(thisPlatformText(null, true)).toBe('This computer could not say which platform it is.');
    expect(thisPlatformText({ os: 'MACOS', arch: 'AARCH64', missing: [] }, false)).toBe('macOS, Apple silicon (arm64)');
  });
});
