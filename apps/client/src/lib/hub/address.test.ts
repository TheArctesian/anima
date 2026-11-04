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
    ['wss://hub.anima.ing', 'not-https'],
    ['https://operator:hunter2@hub.anima.ing', 'has-credentials'],
    ['https://hub.anima.ing/graphql', 'has-path'],
    ['https://hub.anima.ing/?x=1', 'has-path'],
  ])('refuses %j as %s', (text, problem) => {
    const result = parseHubAddress(text);
    expect(result.ok ? undefined : result.error.problem).toBe(problem);
  });

  it('LOG-007: a refused address does not repeat a password from the input', () => {
    const result = parseHubAddress('https://operator:hunter2@hub.anima.ing');
    expect(JSON.stringify(result)).not.toContain('hunter2');
  });
});
