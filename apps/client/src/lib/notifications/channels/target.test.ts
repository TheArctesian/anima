import { describe, expect, it } from 'vitest';
import type { ChannelSettings } from '../api';
import { FAKE_SECRET, FIXTURE_CHANNELS } from '../notifications.fixture';
import { hostOf, mailHostOf, originOf, TARGET_HIDDEN, targetOf } from './target';

describe('targetOf', () => {
  it('NOT-002, D-17: each kind shows only an origin or a host', () => {
    expect(FIXTURE_CHANNELS.map((channel) => [channel.kind, targetOf(channel.settings)])).toEqual([
      ['WEBHOOK', 'https://hooks.example.net'],
      ['NTFY', 'ntfy.example.net'],
      ['EMAIL', 'mail.example.net'],
      ['APPRISE', 'https://apprise.example.net'],
    ]);
  });

  it('D-17, F-8: a path, query, fragment or user name and password the hub sends is never shown', () => {
    const leaky = `https://user:${FAKE_SECRET}@hooks.example.net:8443/hook/${FAKE_SECRET}?token=${FAKE_SECRET}#${FAKE_SECRET}`;
    const settings: ChannelSettings[] = [
      { __typename: 'WebhookSettings', origin: leaky, method: 'POST', headerNames: [] },
      { __typename: 'NtfySettings', server: leaky, priority: 3, tokenStored: true },
      { __typename: 'AppriseSettings', server: leaky, mode: 'CONFIG_KEY', tags: [] },
    ];
    expect(settings.map(targetOf)).toEqual(['https://hooks.example.net:8443', 'hooks.example.net:8443', 'https://hooks.example.net:8443']);
    for (const each of settings) expect(targetOf(each)).not.toContain(FAKE_SECRET);
  });

  it('D-17: a webhook with no path still shows its origin, and ntfy.sh shows as its host', () => {
    expect(originOf('https://hooks.example.com')).toBe('https://hooks.example.com');
    expect(hostOf('https://ntfy.sh')).toBe('ntfy.sh');
  });

  it('F-8: anything that is not an http(s) URL is not shown at all', () => {
    for (const text of [`discord://${FAKE_SECRET}/${FAKE_SECRET}`, `hooks.example.net/${FAKE_SECRET}`, '', `javascript:${FAKE_SECRET}`]) {
      expect(originOf(text)).toBe(TARGET_HIDDEN);
      expect(hostOf(text)).toBe(TARGET_HIDDEN);
    }
  });

  it('F-8: an SMTP host that carries more than a host name is not shown', () => {
    expect(mailHostOf('SMTP.Example.com')).toBe('smtp.example.com');
    expect(mailHostOf('[2001:db8::1]')).toBe('[2001:db8::1]');
    for (const host of [`user:${FAKE_SECRET}@smtp.example.com`, `smtp.example.com/${FAKE_SECRET}`, 'smtp.example.com:587', ' smtp.example.com', '']) {
      expect(mailHostOf(host)).toBe(TARGET_HIDDEN);
    }
  });
});
