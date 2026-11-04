import { describe, expect, it } from 'vitest';
import { tunnelUrl } from './url';

describe('tunnel URL (K4 §2, D-3)', () => {
  it('K4 §2: an https hub gives wss plus the ticket path', () => {
    expect(tunnelUrl('https://hub.anima.ing', '/tunnel')).toBe('wss://hub.anima.ing/tunnel');
  });

  it('K4 §2: the port is kept', () => {
    expect(tunnelUrl('https://hub.anima.ing:8443', '/tunnel')).toBe('wss://hub.anima.ing:8443/tunnel');
  });

  it('D-3: an http hub gives ws', () => {
    expect(tunnelUrl('http://127.0.0.1:8080', '/tunnel')).toBe('ws://127.0.0.1:8080/tunnel');
  });

  const refusedPaths = ['', 'tunnel', '//evil.example/tunnel', '/tunnel?token=x', '/tunnel#x', '/../tunnel', '/a/./b', '/tun nel', '/'];
  for (const path of refusedPaths) {
    it(`SEC-002: refuses the tunnel path ${JSON.stringify(path)}`, () => {
      expect(tunnelUrl('https://hub.anima.ing', path)).toBeUndefined();
    });
  }

  for (const origin of ['ftp://hub.anima.ing', 'not a url', 'https://user:pw@hub.anima.ing', 'javascript:alert(1)']) {
    it(`SEC-002: refuses the hub origin ${JSON.stringify(origin)}`, () => {
      expect(tunnelUrl(origin, '/tunnel')).toBeUndefined();
    });
  }
});
