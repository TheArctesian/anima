import { describe, expect, it } from 'vitest';
import { tunnelUrl } from './url';

describe('tunnel URL (K4 §2, D-3)', () => {
  it('K4 §2: an https hub gives wss plus the ticket path', () => {
    expect(tunnelUrl('https://hub.anima.ing', '/tunnel')).toBe('wss://hub.anima.ing/tunnel');
  });

  it('K4 §2: the port is kept', () => {
    expect(tunnelUrl('https://hub.anima.ing:8443', '/tunnel')).toBe('wss://hub.anima.ing:8443/tunnel');
  });

  for (const [origin, url] of [
    ['http://127.0.0.1:8080', 'ws://127.0.0.1:8080/tunnel'],
    ['http://127.1.2.3', 'ws://127.1.2.3/tunnel'],
    ['http://localhost:5173', 'ws://localhost:5173/tunnel'],
    ['http://[::1]:8080', 'ws://[::1]:8080/tunnel'],
  ]) {
    it(`SEC-002: http to the loopback host ${origin} gives ws (local development)`, () => {
      expect(tunnelUrl(origin ?? '', '/tunnel')).toBe(url);
    });
  }

  for (const origin of ['http://hub.anima.ing', 'http://192.168.1.10:8080', 'http://localhost.evil.example', 'http://127.0.0.1.evil.example', 'http://[::2]']) {
    it(`SEC-002: http to the non-loopback host ${origin} is refused`, () => {
      expect(tunnelUrl(origin, '/tunnel')).toBeUndefined();
    });
  }

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
