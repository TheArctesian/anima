import { codeOf, type Bounds, type Fingerprint, type WebViewShell, type WebViewStep } from '$lib/desktop/webView/webView';

/** Where the tab's native web view stands. `popped` lives in its own window; `closed` takes no more commands. */
export type ViewState = 'none' | 'opening' | 'open' | 'popped' | 'closed';

/** What opening the view needs besides the tab and the bounds. */
export interface ViewTarget {
  readonly deviceId: string;
  readonly url: string;
  readonly pin: Fingerprint | null;
}

export interface NativeViewOptions {
  readonly shell: WebViewShell;
  readonly tabId: string;
  /** A command failed; `code` is the shell's code only (LOG-007). */
  readonly onfail: (code: string) => void;
  readonly onstate: (state: ViewState) => void;
}

/** The tab's native web view, driven through the shell. Commands run one at a time, in order. */
export interface NativeView {
  /** Open the view once, at `bounds`; does nothing unless no view is open. */
  open(target: ViewTarget, bounds: Bounds): void;
  /** Move the view to `bounds`; a burst of moves sends only the latest. */
  place(bounds: Bounds): void;
  /** Show or hide the view; only a change is sent. */
  show(shown: boolean): void;
  go(to: WebViewStep): void;
  pin(pin: Fingerprint): void;
  popOut(): void;
  /** Close the view, after any command still running; a popped-out view stays in its window. */
  close(): void;
  /** Close the view, popped out or not, so that the next `open` makes a new one after it (H-4). */
  restart(): void;
}

function sameBounds(a: Bounds, b: Bounds): boolean {
  return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
}

/** Whether `bounds` has an area: a hidden or unlaid tab measures zero and is not placed. */
export function hasArea(bounds: Bounds): boolean {
  return bounds.width > 0 && bounds.height > 0;
}

/** `rect` rounded to whole CSS pixels, as the shell places views. */
export function boundsOf(rect: DOMRectReadOnly): Bounds {
  return { x: Math.round(rect.x), y: Math.round(rect.y), width: Math.round(rect.width), height: Math.round(rect.height) };
}

type Run = (command: () => Promise<void>) => void;

/** A command that sends only the latest wanted value, and only when it differs from the last one sent. */
function coalesced<T>(run: Run, live: () => boolean, same: (sent: T | undefined, wanted: T) => boolean, send: (value: T) => Promise<void>) {
  let sent: T | undefined;
  let wanted: T | undefined;
  let queued = false;
  return {
    want(value: T) {
      wanted = value;
      if (queued) return;
      queued = true;
      run(async () => {
        queued = false;
        if (!live() || wanted === undefined || same(sent, wanted)) return;
        const to = wanted;
        await send(to);
        sent = to;
      });
    },
    /** What the shell holds now, as the open set it. */
    sent(value: T | undefined) {
      sent = value;
    },
  };
}

/** The shell's code for a tab it holds no web view for (T3). */
export const UNKNOWN_TAB = 'unknown_tab';

/** Runs each command after the ones before it; a failure is reported with its code and the chain goes on. */
function chained(onfail: (code: string) => void): Run {
  let chain: Promise<void> = Promise.resolve();
  return (command) => {
    chain = chain.then(command).catch((reason: unknown) => {
      onfail(codeOf(reason));
    });
  };
}

export function nativeView({ shell, tabId, onfail, onstate }: NativeViewOptions): NativeView {
  let state: ViewState = 'none';
  const become = (next: ViewState) => {
    state = next;
    onstate(next);
  };
  const run = chained((code) => {
    // The operator closed a popped-out window, or the shell lost the view: the tab has none now, so a reload opens one.
    if (code === UNKNOWN_TAB && state !== 'closed') become('none');
    onfail(code);
  });
  /**
   * Leave the view for `next` and close it in the shell, on the chain: an open still running finishes first, so the
   * view it makes is the one closed, and a new open runs after. A popped-out view stays only when the tab closes.
   */
  const end = (next: 'none' | 'closed') => {
    const was = state;
    become(next);
    if (was !== 'none' && (was !== 'popped' || next === 'none')) run(() => shell.close(tabId));
  };
  const live = () => state === 'open';
  const started = () => state === 'open' || state === 'opening';
  const placing = coalesced<Bounds>(run, live, (sent, wanted) => sent !== undefined && sameBounds(sent, wanted), (to) => shell.place(tabId, to));
  const showing = coalesced<boolean>(run, live, (sent, wanted) => sent === wanted, (to) => shell.show(tabId, to));

  return {
    open(target, bounds) {
      if (state !== 'none') return;
      become('opening');
      run(async () => {
        // A failure is told first, so the tab leaves its loading step before a new open could start.
        const opened = await shell.open({ tabId, ...target, bounds }).then(() => true, (reason: unknown) => (onfail(codeOf(reason)), false));
        if (state !== 'opening') return;
        placing.sent(bounds);
        showing.sent(undefined);
        become(opened ? 'open' : 'none');
      });
    },
    place(bounds) {
      if (started()) placing.want(bounds);
    },
    show(shown) {
      if (started()) showing.want(shown);
    },
    go(to) {
      if (live()) run(() => shell.go(tabId, to));
    },
    pin(pin) {
      // A popped-out view is still the tab's, so it takes the new pin too.
      if (live() || state === 'popped') run(() => shell.pin(tabId, pin));
    },
    popOut() {
      if (!live()) return;
      run(async () => {
        await shell.popOut(tabId);
        if (live()) become('popped');
      });
    },
    restart() {
      if (state !== 'none' && state !== 'closed') end('none');
    },
    close() {
      if (state !== 'closed') end('closed');
    },
  };
}
