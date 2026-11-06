import { paneKeyboard, type PaneKeyboard } from '../../tunnel/guacamole/input';
import type { SessionTunnel } from '../../tunnel/guacamole/tunnel';
import { isMeasured, remoteSize, sizeSender, type SettleTimer, type Size, type SizeSender } from '../display';
import { followPaneFocus } from '../guacamole/input';

/** What a remote pane tells its view. */
export interface PaneHooks {
  /** The keyboard focus entered (true) or left (false) the pane; on leaving, held keys are already released. */
  readonly focus: (inside: boolean) => void;
  /** The pane's size in CSS px, at every measure. */
  readonly measured: (css: Size) => void;
  /** The timer the size waits with (CQ-021). */
  readonly timer: SettleTimer;
}

/** The pane of one remote view: its keyboard, its focus and its size. */
export interface RemotePane {
  /** Listens to the pane only (D-8); the view binds it while the pane may take keys. */
  readonly keyboard: PaneKeyboard;
  /**
   * Send `client` the pane's size in device pixels once it settles (RA-004, D-5), starting from `initial`, the size
   * the session started with; the size now is heard at once in case it changed while the hub answered. Returns
   * the function that stops it.
   */
  sendSizes(client: SessionTunnel['client'], initial: Size): () => void;
  /** Hear the pane's last size again, as after a change of `devicePixelRatio`. */
  remeasure(): void;
  stop(): void;
}

/**
 * Call `change` whenever `devicePixelRatio` changes, as when the window moves to another monitor: a `resolution`
 * media query for the ratio now, made again after each change. Returns the function that stops. Without
 * `matchMedia` nothing is followed.
 */
export function followPixelRatio(change: (ratio: number) => void): () => void {
  if (typeof window.matchMedia !== 'function') return () => undefined;
  let query: MediaQueryList | undefined;
  const listen = () => {
    query = window.matchMedia(`(resolution: ${String(window.devicePixelRatio)}dppx)`);
    query.addEventListener('change', changed, { once: true });
  };
  const changed = () => {
    change(window.devicePixelRatio);
    listen();
  };
  listen();
  return () => {
    query?.removeEventListener('change', changed);
  };
}

/** Follow `element` as a remote pane, reporting to `hooks`. */
export function remotePane(element: HTMLElement, hooks: PaneHooks): RemotePane {
  const keyboard = paneKeyboard(element);
  let last: Size | undefined;
  let sender: SizeSender | undefined;
  const resized = (css: Size) => {
    if (isMeasured(css)) sender?.resized(remoteSize(css, window.devicePixelRatio));
  };
  const stopFollowing = followPaneFocus(element, (inside) => {
    // Release held keys now: the view's binding effect runs later, and a held key would repeat until then.
    if (!inside) keyboard.unbind();
    hooks.focus(inside);
  });
  const observer = new ResizeObserver((entries) => {
    const box = entries.at(-1)?.contentRect;
    if (box === undefined) return;
    last = { width: box.width, height: box.height };
    hooks.measured(last);
    resized(last);
  });
  observer.observe(element);
  return {
    keyboard,
    sendSizes: (client, initial) => {
      const mine = sizeSender(initial, (size) => client.sendSize(size.width, size.height), hooks.timer);
      sender = mine;
      if (last !== undefined) resized(last);
      return () => {
        mine.cancel();
        if (sender === mine) sender = undefined;
      };
    },
    remeasure: () => {
      if (last !== undefined) resized(last);
    },
    stop: () => {
      sender?.cancel();
      keyboard.unbind();
      stopFollowing();
      observer.disconnect();
    },
  };
}
