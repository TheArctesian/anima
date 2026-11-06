import type { SessionPolicy } from '../../transfer';

/** What one remote session may do with the clipboard (RA-005, RA-033), and how the view says so. */
export interface ClipboardRules {
  /** Text the device copies may reach the system clipboard. */
  readonly copy: boolean;
  /** The system clipboard may be sent to the device. */
  readonly paste: boolean;
  /** One fixed sentence naming what is off and why; undefined when both are on. */
  readonly notice: string | undefined;
  /** Why "Paste" is off, shown on the menu item; undefined when paste is on. */
  readonly pasteOff: string | undefined;
}

/** Fixed texts of the clipboard notices; never the clipboard's content (LOG-007). */
export const COPY_FAILED = 'Could not copy the text to your clipboard.';
export const PASTE_FAILED = 'Could not read your clipboard. Allow clipboard access, or use Paste in the Keys menu.';
export const PASTE_REFUSED = 'Paste to the device is off, so nothing was sent.';

const UNREAD = 'Copy and paste are off: the hub did not say whether this device allows them.';
const UNREAD_PASTE = 'Off: the hub did not answer';
const OFF_FOR_DEVICE = 'Off for this device';
const BOTH_OFF = 'Copy and paste are off for this device.';
const COPY_OFF = 'Copy from the device is off for this device.';
const PASTE_OFF = 'Paste to the device is off for this device.';

/**
 * The rules for a session started with `policy`. An unread policy turns both off (it fails closed, D-3); only a
 * flag that is exactly `true` turns its direction on.
 */
export function clipboardRules(policy: SessionPolicy): ClipboardRules {
  if (!policy.read) return { copy: false, paste: false, notice: UNREAD, pasteOff: UNREAD_PASTE };
  const copy = policy.copy === true;
  const paste = policy.paste === true;
  const notice = copy ? (paste ? undefined : PASTE_OFF) : paste ? COPY_OFF : BOTH_OFF;
  return { copy, paste, notice, pasteOff: paste ? undefined : OFF_FOR_DEVICE };
}

/** What a session that has no open tunnel may do: nothing. */
export const NO_CLIPBOARD: ClipboardRules = { copy: false, paste: false, notice: undefined, pasteOff: undefined };
