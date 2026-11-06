import type { RemoteProtocol } from '../../api';
import type { Size } from '../display';

/**
 * The scale the remote display is drawn at in a pane of `pane` CSS px (D-5). Scaling is layout: the library sets
 * it once per change as a static transform, never animated (MAT-008).
 * - RDP: the remote is sized to the pane in device pixels (RA-004), so one remote pixel is one device pixel:
 *   `1 / devicePixelRatio`.
 * - VNC: the remote size is the device's own, so the display shrinks to fit the pane whole, keeping its aspect.
 *   It is never drawn larger than one remote pixel per CSS pixel: a small screen stays sharp and sits in the
 *   pane's corner instead of being blown up and blurred. Until both sizes are known it is 1.
 */
export function displayScale(protocol: RemoteProtocol, remote: Size, pane: Size, devicePixelRatio: number): number {
  if (protocol === 'RDP') return Number.isFinite(devicePixelRatio) && devicePixelRatio > 0 ? 1 / devicePixelRatio : 1;
  const known = (size: Size) => Number.isFinite(size.width) && Number.isFinite(size.height) && size.width > 0 && size.height > 0;
  if (!known(remote) || !known(pane)) return 1;
  return Math.min(1, pane.width / remote.width, pane.height / remote.height);
}
