import { describe, expect, it } from 'vitest';
import { DEFAULT_HUB_ADDRESS, parseHubAddress } from './address';

describe('parseHubAddress', () => {
  it('NET-005: the default address is hub.anima.ing over https and wss', () => {
    expect(parseHubAddress('https://hub.anima.ing')).toEqual({ ok: true, value: DEFAULT_HUB_ADDRESS });
  });

  it.each([
    ['https://hub.anima.ing/', 'https://hub.anima.ing', 'https://hub.anima.ing/graphql', 'wss://hub.anima.ing/graphql'],
    ['  hub.anima.ing  ', 'https://hub.anima.ing', 'https://hub.anima.ing/graphql', 'wss://hub.anima.ing/graphql'],
    ['https://10.10.0.2:8443', 'https://10.10.0.2:8443', 'https://10.10.0.2:8443/graphql', 'wss://10.10.0.2:8443/graphql'],
    ['HTTPS://Hub.Example.org', 'https://hub.example.org', 'https://hub.example.org/graphql', 'wss://hub.example.org/graphql'],
  ])('NET-005: %s answers at %s', (text, origin, http, socket) => {
    expect(parseHubAddress(text)).toEqual({ ok: true, value: { origin, http, socket } });
  });

  it.each([
    ['', 'not-a-url'],
    ['https://', 'not-a-url'],
    ['http://hub.anima.ing', 'not-https'],
    ['http://localhost.example.com', 'not-https'],
    ['http://10.0.0.5:8443', 'not-https'],
    ['ws://127.0.0.1:8443', 'not-https'],
    ['http://[::1]:8443', 'not-https'],
    // F-1, security review of T6: `localhost` may resolve to ::1 first, where another user can bind the same port.
    ['http://localhost:9000', 'not-https'],
    ['http://user:pw@127.0.0.1:8443', 'has-credentials'],
    ['http://127.0.0.1:8443/graphql', 'has-path'],
    ['wss://hub.anima.ing', 'not-https'],
    ['https://operator:hunter2@hub.anima.ing', 'has-credentials'],
    ['https://hub.anima.ing/graphql', 'has-path'],
    ['https://hub.anima.ing/?x=1', 'has-path'],
  ])('refuses %j as %s', (text, problem) => {
    const result = parseHubAddress(text);
    expect(result.ok ? undefined : result.error.problem).toBe(problem);
  });

  it.each([
    ['http://127.0.0.1:8443', 'http://127.0.0.1:8443', 'ws://127.0.0.1:8443/graphql'],
    ['127.0.0.1:8443', 'https://127.0.0.1:8443', 'wss://127.0.0.1:8443/graphql'],
  ])('CLI-002: %s is a loopback hub; plain http only there, a bare host stays https', (text, origin, socket) => {
    expect(parseHubAddress(text)).toEqual({ ok: true, value: { origin, http: `${origin}/graphql`, socket } });
  });

  it('LOG-007: a refused address does not repeat a password from the input', () => {
    const result = parseHubAddress('https://operator:hunter2@hub.anima.ing');
    expect(JSON.stringify(result)).not.toContain('hunter2');
  });
});

describe('a hub address is frozen', () => {
  it('CLI-002: the default and a parsed address cannot be changed to point a token elsewhere', () => {
    const parsed = parseHubAddress('evil.example');
    expect(Object.isFrozen(DEFAULT_HUB_ADDRESS)).toBe(true);
    expect(parsed.ok && Object.isFrozen(parsed.value)).toBe(true);
    expect(() => Object.assign(DEFAULT_HUB_ADDRESS, { http: 'https://evil.example/graphql' })).toThrow(TypeError);
  });
});
