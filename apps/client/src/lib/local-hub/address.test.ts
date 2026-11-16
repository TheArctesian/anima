import { describe, expect, it } from 'vitest';
import { localHubOrigin } from './address';

describe('localHubOrigin', () => {
  it('FRN-008: a ready, installed hub is an http origin at its address', () => {
    expect(localHubOrigin({ installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443' })).toBe('http://127.0.0.1:8443');
  });

  it.each([
    [{ installed: false, running: false }],
    [{ installed: true, running: true, state: 'starting', hub: '127.0.0.1:8443' }],
    [{ installed: true, running: true, state: 'ready' }],
    [{ installed: true, running: true, state: 'ready', hub: '' }],
    [{ installed: false, running: true, state: 'ready', hub: '127.0.0.1:8443' }],
  ])('FRN-008: %j has no origin', (status) => {
    expect(localHubOrigin(status)).toBeNull();
  });
});
