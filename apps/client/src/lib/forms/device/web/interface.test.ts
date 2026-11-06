import { expect, it } from 'vitest';
import type { WebInterface } from '../../hub-pending';
import { validateWeb, webChanged, webInput, webValuesFrom } from './interface';

const SAVED: WebInterface = { id: 'w1', deviceId: 'd1', label: 'Proxmox VE', url: 'https://pve.lan:8006', openIn: 'ANIMA_TAB' };

it('WEB-001: an http or https URL with a host is valid', () => {
  expect(validateWeb({ label: '', url: 'http://ha.lan:8123', openIn: 'ANIMA_TAB' })).toEqual([]);
  expect(validateWeb({ label: '', url: ' https://pve.lan:8006/ ', openIn: 'ANIMA_TAB' })).toEqual([]);
});

it('WEB-001: an empty URL, another scheme, or no host is refused on the url field', () => {
  for (const url of ['', '  ', 'ftp://nas.lan', 'pve.lan:8006', 'not a url', 'https://']) {
    const errors = validateWeb({ label: 'x', url, openIn: 'ANIMA_TAB' });
    expect(errors.map((error) => error.field), url).toEqual(['url']);
  }
});

it('Section 6.3.4: a new interface opens in an Anima tab and has an empty label', () => {
  expect(webValuesFrom()).toEqual({ label: '', url: '', openIn: 'ANIMA_TAB' });
  expect(webValuesFrom(SAVED)).toEqual({ label: 'Proxmox VE', url: 'https://pve.lan:8006', openIn: 'ANIMA_TAB' });
});

it('Section 6.3.4: an empty label is sent as null so the hub uses the host; text is trimmed', () => {
  expect(webInput({ label: '  ', url: ' https://pve.lan ', openIn: 'SYSTEM_BROWSER' })).toEqual({
    label: null,
    url: 'https://pve.lan',
    openIn: 'SYSTEM_BROWSER',
  });
  expect(webInput({ label: ' PVE ', url: 'https://pve.lan', openIn: 'ANIMA_TAB' }).label).toBe('PVE');
});

it('D-6: a row is changed only when a value differs from the stored one', () => {
  expect(webChanged(webValuesFrom(SAVED), SAVED)).toBe(false);
  expect(webChanged({ ...webValuesFrom(SAVED), label: 'PVE' }, SAVED)).toBe(true);
  expect(webChanged({ ...webValuesFrom(SAVED), url: 'https://pve.lan' }, SAVED)).toBe(true);
  expect(webChanged({ ...webValuesFrom(SAVED), openIn: 'SYSTEM_BROWSER' }, SAVED)).toBe(true);
  expect(webChanged(webValuesFrom(), undefined)).toBe(true);
});
