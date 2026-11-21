import { describe, expect, it } from 'vitest';
import { lastUseText, repositoryName, spanText } from './sessions';

const NOW = Date.parse('2026-11-20T12:00:00Z');

describe('the words of the list of sessions (AIS-025, AIS-026)', () => {
  it('says the last use in minutes, hours, yesterday, then the day', () => {
    expect(lastUseText('2026-11-20T11:59:40Z', NOW)).toBe('Just now');
    expect(lastUseText('2026-11-20T11:48:00Z', NOW)).toBe('12 minutes ago');
    expect(lastUseText('2026-11-20T11:00:00Z', NOW)).toBe('1 hour ago');
    expect(lastUseText('2026-11-19T09:00:00Z', NOW)).toBe('Yesterday');
    expect(lastUseText('2026-11-18T09:00:00Z', NOW)).toBe('18 Nov');
  });

  it('says how long a session ran', () => {
    expect(spanText('2026-11-20T10:00:00Z', '2026-11-20T10:45:00Z')).toBe('45 min');
    expect(spanText('2026-11-20T08:00:00Z', '2026-11-20T10:10:00Z')).toBe('2 h 10 min');
    expect(spanText('2026-11-20T08:00:00Z', '2026-11-20T10:00:00Z')).toBe('2 h');
  });

  it('leaves the host out only when every line has it', () => {
    expect(repositoryName('github.com/acme/anima', 'github.com/')).toBe('acme/anima');
    expect(repositoryName('gitlab.com/acme/site', '')).toBe('gitlab.com/acme/site');
    expect(repositoryName(null, 'github.com/')).toBe('No repository');
  });
});
