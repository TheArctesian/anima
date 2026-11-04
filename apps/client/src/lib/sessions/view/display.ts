import type { SessionDisplay } from '../api';

/** A size in pixels. */
export interface Size {
  readonly width: number;
  readonly height: number;
}

/** The size range the hub forwards (K4 §4, R-018). */
export const SIZE_MIN = 1;
export const SIZE_MAX = 8192;
/** The DPI range the hub forwards (K4 §4). */
export const DPI_MIN = 48;
export const DPI_MAX = 480;
/** CSS pixels per inch: a `devicePixelRatio` of 1 is 96 DPI. */
const CSS_DPI = 96;

/**
 * The pane size used when the pane has no size yet (a hidden pane measures 0 × 0); the first real resize then
 * corrects it.
 */
export const UNMEASURED_PANE: Size = { width: 1024, height: 768 };

/** How long the pane size must stay the same before `size` is sent. */
export const RESIZE_SETTLE_MS = 150;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/** A ratio that is not a positive finite number counts as 1. */
function ratio(devicePixelRatio: number): number {
  return Number.isFinite(devicePixelRatio) && devicePixelRatio > 0 ? devicePixelRatio : 1;
}

/** Whether a measured pane has an area; a hidden pane measures 0 in at least one direction. */
export function isMeasured(pane: Size): boolean {
  return pane.width > 0 && pane.height > 0;
}

/** The remote display size for a pane of `pane` CSS px: device pixels, rounded and clamped to 1..8192 (D-7). */
export function remoteSize(pane: Size, devicePixelRatio: number): Size {
  const scale = ratio(devicePixelRatio);
  const pixels = (css: number) => clamp(Math.round((Number.isFinite(css) ? css : 0) * scale), SIZE_MIN, SIZE_MAX);
  return { width: pixels(pane.width), height: pixels(pane.height) };
}

/** 96 × `devicePixelRatio`, rounded and clamped to 48..480 (D-7). */
export function displayDpi(devicePixelRatio: number): number {
  return clamp(Math.round(CSS_DPI * ratio(devicePixelRatio)), DPI_MIN, DPI_MAX);
}

/**
 * The `DisplayInput` a session starts with (GW-012, D-7): the pane in device pixels, its DPI and the IANA zone.
 * An unmeasured pane counts as `UNMEASURED_PANE`.
 */
export function displayInput(pane: Size, devicePixelRatio: number, timezone: string): SessionDisplay {
  const size = remoteSize(isMeasured(pane) ? pane : UNMEASURED_PANE, devicePixelRatio);
  return { widthPx: size.width, heightPx: size.height, dpi: displayDpi(devicePixelRatio), timezone };
}

/** The timer a `SizeSender` waits with (CQ-021). */
export interface SettleTimer {
  set(callback: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

/** Hears remote sizes and sends one after they settle. */
export interface SizeSender {
  resized(size: Size): void;
  /** Drop a waiting size. */
  cancel(): void;
}

/**
 * Send `size` once the remote size has not changed for `RESIZE_SETTLE_MS` (D-7), and only when it differs from
 * the last size sent, starting from `initial` (the size the session started with). The pane's size never depends
 * on what is sent, so a sent size cannot cause another resize.
 */
export function sizeSender(initial: Size, send: (size: Size) => void, timer: SettleTimer): SizeSender {
  let last = initial;
  let waiting: unknown;
  const cancel = () => {
    if (waiting !== undefined) timer.clear(waiting);
    waiting = undefined;
  };
  return {
    resized: (size) => {
      cancel();
      waiting = timer.set(() => {
        waiting = undefined;
        if (size.width === last.width && size.height === last.height) return;
        last = size;
        send(size);
      }, RESIZE_SETTLE_MS);
    },
    cancel,
  };
}
