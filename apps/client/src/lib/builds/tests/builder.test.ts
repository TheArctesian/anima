import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { BuildsApi, Enrolment } from '../api';
import { ThisBuilder, type ThisBuilderPorts } from '../builder.svelte';
import { BUILDER_TOKEN, FakeBuildsApi, FakeShell, ok, refused, sleep } from './builds.fixture';

class Stores {
  token: string | null = null;
  id: string | null = null;
  writeFails = false;
  readonly tokens = {
    read: () => Promise.resolve(this.token),
    write: (token: string) => (this.writeFails ? Promise.reject(new Error('denied')) : Promise.resolve(void (this.token = token))),
    clear: () => Promise.resolve(void (this.token = null)),
  };
  readonly ids = {
    read: () => this.id,
    write: (id: string) => void (this.id = id),
    clear: () => void (this.id = null),
  };
}

let stores: Stores;
let shell: FakeShell;
let builderApi: FakeBuildsApi;
let operator: { enrol: Mock<BuildsApi['enrol']>; revoke: Mock<BuildsApi['revoke']> };
let builder: ThisBuilder;

beforeEach(() => {
  vi.useFakeTimers({ now: 0 });
  stores = new Stores();
  shell = new FakeShell(sleep);
  builderApi = new FakeBuildsApi();
  operator = {
    enrol: vi.fn<BuildsApi['enrol']>((): Promise<HubResult<Enrolment>> => Promise.resolve(ok({ builderId: 'b-1', token: BUILDER_TOKEN }))),
    revoke: vi.fn<BuildsApi['revoke']>((): Promise<HubResult<string>> => Promise.resolve(ok('b-1'))),
  };
  const ports: ThisBuilderPorts = {
    operator,
    builder: builderApi,
    shell,
    tokens: stores.tokens,
    ids: stores.ids,
    computerName: () => Promise.resolve('pop-os'),
    sleep,
  };
  builder = new ThisBuilder(ports);
});

afterEach(async () => {
  await builder.stop();
  vi.useRealTimers();
});

describe('turning building on (BLD-021, BLD-023)', () => {
  it('BLD-021, BLD-023: enrols with this computer’s name and platform, keeps the token in the key store and starts claiming', async () => {
    await builder.enable();
    await vi.advanceTimersByTimeAsync(0);
    expect(operator.enrol).toHaveBeenCalledWith('pop-os', { os: 'LINUX', arch: 'X86_64' });
    expect(stores.token).toBe(BUILDER_TOKEN);
    expect(stores.id).toBe('b-1');
    expect(builder.enrolled).toBe(true);
    expect(builderApi.claims).toBe(1);
    expect(builder.status).toEqual({ kind: 'waiting' });
  });

  it('BLD-021: a refused enrolment keeps nothing and starts nothing', async () => {
    operator.enrol.mockResolvedValue(refused('FORBIDDEN'));
    await builder.enable();
    expect(stores.token).toBeNull();
    expect(builder.enrolled).not.toBe(true);
    expect(builder.problem).toBe('The hub did not enrol this computer as a builder.');
    expect(builderApi.claims).toBe(0);
  });

  it('BLD-023: when the key store refuses the token, the new builder is revoked at once', async () => {
    stores.writeFails = true;
    await builder.enable();
    expect(operator.revoke).toHaveBeenCalledWith('b-1');
    expect(stores.id).toBeNull();
    expect(builder.problem).toBe('The builder token could not be kept in the key store.');
  });

  it('BLD-026: a computer that already holds a token starts its loop when the app starts', async () => {
    stores.token = BUILDER_TOKEN;
    await builder.resume();
    await vi.advanceTimersByTimeAsync(0);
    expect(builder.enrolled).toBe(true);
    expect(builderApi.claims).toBe(1);
  });

  it('BLD-020: the toggle is off by default', async () => {
    await builder.resume();
    expect(builder.enrolled).toBe(false);
    expect(builderApi.claims).toBe(0);
  });
});

describe('turning building off (BLD-025)', () => {
  it('BLD-025: stops, revokes the token on the hub and deletes it from the key store', async () => {
    await builder.enable();
    await builder.disable();
    expect(operator.revoke).toHaveBeenCalledWith('b-1');
    expect(stores.token).toBeNull();
    expect(stores.id).toBeNull();
    expect(builder.enrolled).toBe(false);
    expect(builder.status).toEqual({ kind: 'stopped' });
    const claims = builderApi.claims;
    await vi.advanceTimersByTimeAsync(120_000);
    expect(builderApi.claims).toBe(claims);
  });

  it('BLD-025: the token leaves the key store even when the hub cannot be reached, and the row says so', async () => {
    await builder.enable();
    operator.revoke.mockResolvedValue(refused('unreachable'));
    await builder.disable();
    expect(stores.token).toBeNull();
    expect(builder.problem).toBe('The hub could not revoke this builder. Revoke it in the list below.');
  });

  it('BLD-025: revoking this computer in the list forgets its token; another builder’s does not', async () => {
    await builder.enable();
    await builder.revoked('b-2');
    expect(stores.token).toBe(BUILDER_TOKEN);
    await builder.revoked('b-1');
    expect(stores.token).toBeNull();
    expect(builder.enrolled).toBe(false);
  });
});
