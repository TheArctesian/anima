import { describe, expect, it } from 'vitest';
import { RESIZE_SETTLE_MS, UNMEASURED_PANE, displayDpi, displayInput, remoteSize, sizeSender, type Size } from './display';

describe('display size (D-7)', () => {
  it.each([
    [{ width: 800, height: 600 }, 1, { width: 800, height: 600 }],
    [{ width: 640.4, height: 400.2 }, 2, { width: 1281, height: 800 }],
    [{ width: 1000, height: 700 }, 1.5, { width: 1500, height: 1050 }],
    [{ width: 0.2, height: 0 }, 1, { width: 1, height: 1 }],
    [{ width: 6000, height: 5000 }, 2, { width: 8192, height: 8192 }],
    [{ width: Number.NaN, height: Number.POSITIVE_INFINITY }, 1, { width: 1, height: 1 }],
    [{ width: 800, height: 600 }, 0, { width: 800, height: 600 }],
    [{ width: 800, height: 600 }, Number.NaN, { width: 800, height: 600 }],
  ])('K4 §4: pane %o at ratio %s is %o device pixels, within 1..8192', (pane, ratio, size) => {
    expect(remoteSize(pane, ratio)).toEqual(size);
  });

  it.each([
    [1, 96],
    [2, 192],
    [1.25, 120],
    [0.25, 48],
    [6, 480],
    [Number.NaN, 96],
    [-1, 96],
  ])('K4 §4: ratio %s is %s DPI, within 48..480', (ratio, dpi) => {
    expect(displayDpi(ratio)).toBe(dpi);
  });

  it('GW-012: DisplayInput carries the pane in device pixels, the DPI and the IANA zone as given', () => {
    expect(displayInput({ width: 640, height: 400 }, 2, 'Europe/London')).toEqual({
      widthPx: 1280,
      heightPx: 800,
      dpi: 192,
      timezone: 'Europe/London',
    });
  });

  it('R-018: any ratio and pane, however extreme, gives integer sizes in 1..8192 and a DPI in 48..480', () => {
    const inRange = (value: number, min: number, max: number) => Number.isInteger(value) && value >= min && value <= max;
    for (const ratio of [Number.NaN, 0, -1, Number.POSITIVE_INFINITY, 1e308, 1e-308]) {
      for (const pane of [{ width: 0, height: 0 }, { width: Number.NaN, height: 5 }, { width: Number.POSITIVE_INFINITY, height: 1e9 }, { width: 800, height: 600 }]) {
        const display = displayInput(pane, ratio, 'UTC');
        const size = remoteSize(pane, ratio);
        for (const value of [display.widthPx, display.heightPx, size.width, size.height]) expect(inRange(value, 1, 8192)).toBe(true);
        expect(inRange(display.dpi ?? 0, 48, 480)).toBe(true);
      }
    }
  });

  it('D-7: an unmeasured (hidden) pane starts at the fallback size', () => {
    const display = displayInput({ width: 0, height: 300 }, 1, 'UTC');
    expect([display.widthPx, display.heightPx]).toEqual([UNMEASURED_PANE.width, UNMEASURED_PANE.height]);
  });
});

/** A hand-driven timer: `run` fires every callback whose time has come. */
function manualTimer() {
  let now = 0;
  let next = 0;
  const waiting = new Map<number, { at: number; callback: () => void }>();
  return {
    timer: {
      set: (callback: () => void, ms: number) => {
        next += 1;
        waiting.set(next, { at: now + ms, callback });
        return next;
      },
      clear: (handle: unknown) => {
        waiting.delete(handle as number);
      },
    },
    advance(ms: number) {
      now += ms;
      for (const [handle, { at, callback }] of [...waiting]) {
        if (at > now) continue;
        waiting.delete(handle);
        callback();
      }
    },
    pending: () => waiting.size,
  };
}

describe('sizeSender (RA-003)', () => {
  const START: Size = { width: 800, height: 600 };

  function sender() {
    const clock = manualTimer();
    const sizes: Size[] = [];
    return { clock, sizes, sender: sizeSender(START, (size) => sizes.push(size), clock.timer) };
  }

  it('RA-003: sizes are debounced; only the last one is sent, once it settled', () => {
    const { clock, sizes, sender: s } = sender();
    s.resized({ width: 900, height: 600 });
    clock.advance(RESIZE_SETTLE_MS - 1);
    s.resized({ width: 1000, height: 600 });
    clock.advance(RESIZE_SETTLE_MS - 1);
    expect(sizes).toEqual([]);
    clock.advance(1);
    expect(sizes).toEqual([{ width: 1000, height: 600 }]);
  });

  it('D-7: the start size and a repeat of the last sent size are not sent again', () => {
    const { clock, sizes, sender: s } = sender();
    s.resized(START);
    clock.advance(RESIZE_SETTLE_MS);
    s.resized({ width: 900, height: 600 });
    clock.advance(RESIZE_SETTLE_MS);
    s.resized({ width: 900, height: 600 });
    clock.advance(RESIZE_SETTLE_MS);
    expect(sizes).toEqual([{ width: 900, height: 600 }]);
  });

  it('cancel drops a waiting size and leaves no timer', () => {
    const { clock, sizes, sender: s } = sender();
    s.resized({ width: 900, height: 600 });
    s.cancel();
    clock.advance(RESIZE_SETTLE_MS);
    expect(sizes).toEqual([]);
    expect(clock.pending()).toBe(0);
  });
});
