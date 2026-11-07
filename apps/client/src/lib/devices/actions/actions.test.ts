import { describe, expect, it } from 'vitest';
import { connectProtocols, panelPosition, warningLines, webInterfacesOf } from './actions';

const KEY = { fingerprintSha256: 'SHA256:abc' };

describe('connectProtocols (RA-030, D-4)', () => {
  it.each([
    [{ ssh: { enabled: true, hostKey: KEY }, rdp: { enabled: true }, vnc: { enabled: true } }, ['SSH', 'RDP', 'VNC']],
    [{ ssh: { enabled: true, hostKey: null }, rdp: { enabled: true }, vnc: { enabled: false } }, ['RDP']],
    [{ ssh: { enabled: false, hostKey: null }, rdp: { enabled: false }, vnc: { enabled: true } }, ['VNC']],
    [{ ssh: { enabled: false, hostKey: null }, rdp: { enabled: false }, vnc: { enabled: false } }, []],
  ])('RA-030: %j offers %j', (device, expected) => {
    expect(connectProtocols(device)).toEqual(expected);
  });
});

describe('webInterfacesOf (WEB-003)', () => {
  it('WEB-003: a device the hub does not yet send interfaces for has none', () => {
    expect(webInterfacesOf({})).toEqual([]);
  });
});

describe('warningLines (RA-020)', () => {
  it('RA-020: each warning is a sentence, in the hub order', () => {
    const lines = warningLines(['UNENCRYPTED_VNC', 'IGNORED_CERTIFICATE', 'RDP_SECURITY']);
    expect(lines).toHaveLength(3);
    expect(lines[0]).toContain('VNC');
    expect(lines[1]).toContain('certificate');
    expect(lines[2]).toContain('RDP');
  });
});

describe('panelPosition (WEB-003)', () => {
  const viewport = { width: 800, height: 600 };
  const panel = { width: 200, height: 100 };

  it('WEB-003: the panel lies under its button, level with its left edge', () => {
    expect(panelPosition({ left: 50, top: 100, bottom: 130 }, panel, viewport, { margin: 8, gap: 4 })).toEqual({ left: 50, top: 134 });
  });

  it('WEB-003: it moves left to stay inside the window, one margin from the edge', () => {
    expect(panelPosition({ left: 700, top: 100, bottom: 130 }, panel, viewport, { margin: 8, gap: 4 }).left).toBe(592);
  });

  it('R-079: a wide panel near the right edge ends one margin inside the window; a panel wider than the window starts at the margin', () => {
    const wide = { width: 320, height: 60 };
    const anchor = { left: 1000, top: 100, bottom: 130 };
    const placed = panelPosition(anchor, wide, { width: 1100, height: 600 }, { margin: 8, gap: 4 });
    expect(placed.left + wide.width).toBe(1092);
    expect(panelPosition(anchor, { width: 900, height: 60 }, { width: 800, height: 600 }, { margin: 8, gap: 4 }).left).toBe(8);
  });

  it('WEB-003: it goes above the button when the window ends below', () => {
    expect(panelPosition({ left: 50, top: 540, bottom: 570 }, panel, viewport, { margin: 8, gap: 4 }).top).toBe(436);
  });
});
