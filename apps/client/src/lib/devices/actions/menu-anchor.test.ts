import { describe, expect, it } from 'vitest';
import { ANCHOR_TOLERANCE_PX, anchorOf, menuLeftBehind } from './menu-anchor';

const opened = anchorOf({ top: 300, left: 120 }, 1280);

describe('menuLeftBehind', () => {
  it('WEB-011: a scroll that leaves the button where it was does not close the menu', () => {
    expect(menuLeftBehind(opened, { top: 300, left: 120 }, 1280, 'scroll')).toBe(false);
  });

  it('WEB-011: a drift within the tolerance does not close the menu', () => {
    expect(menuLeftBehind(opened, { top: 300 + ANCHOR_TOLERANCE_PX, left: 120 }, 1280, 'scroll')).toBe(false);
  });

  it('MOT-002: a scroll that moves the button down, up or sideways closes the menu', () => {
    expect(menuLeftBehind(opened, { top: 250, left: 120 }, 1280, 'scroll')).toBe(true);
    expect(menuLeftBehind(opened, { top: 340, left: 120 }, 1280, 'scroll')).toBe(true);
    expect(menuLeftBehind(opened, { top: 300, left: 90 }, 1280, 'scroll')).toBe(true);
  });

  it('WEB-011: a resize that keeps the width (an address bar collapsing) does not close the menu', () => {
    expect(menuLeftBehind(opened, { top: 300, left: 120 }, 1280, 'resize')).toBe(false);
  });

  it('MOT-002: a resize that changes the width closes the menu', () => {
    expect(menuLeftBehind(opened, { top: 300, left: 120 }, 1100, 'resize')).toBe(true);
  });
});
