import { describe, expect, it } from 'vitest';
import { openPortsOf } from '$lib/desktop/link/shell';
import { portRows, systemOf } from './ports';

describe('the switches of the open ports (HMD-049)', () => {
  it('a Mac and a Linux computer offer SSH and screen sharing, each naming the setting that it needs', () => {
    const mac = portRows('mac', []);
    expect(mac.map((row) => row.port)).toEqual([22, 5900]);
    expect(mac.map((row) => row.on)).toEqual([false, false]);
    expect(mac[0]?.needs).toContain('Remote Login');
    expect(mac[1]?.needs).toContain('Screen Sharing');
    expect(portRows('linux', []).map((row) => row.port)).toEqual([22, 5900]);
  });

  it('only Windows offers Remote Desktop', () => {
    const windows = portRows('windows', [3389]);
    expect(windows.map((row) => row.port)).toEqual([22, 3389, 5900]);
    expect(windows.find((row) => row.port === 3389)).toMatchObject({ on: true, label: 'Remote Desktop (port 3389)' });
    expect(windows.find((row) => row.port === 3389)?.needs).toContain('Remote Desktop');
  });

  it('a row is on only when the list of the computer holds its port', () => {
    expect(portRows('mac', [5900]).map((row) => row.on)).toEqual([false, true]);
  });

  it('the system comes from the user agent of the web view', () => {
    expect(systemOf('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15')).toBe('mac');
    expect(systemOf('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows');
    expect(systemOf('Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux');
  });

  it('the answer of the shell is read strictly', () => {
    expect(openPortsOf({ open: [22, 5900] })).toEqual({ open: [22, 5900] });
    for (const value of [null, {}, { open: '22' }, { open: [0] }, { open: [70_000] }, { open: [22.5] }]) {
      expect(openPortsOf(value), JSON.stringify(value)).toBeUndefined();
    }
  });
});
