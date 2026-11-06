import type { Terminal } from '@xterm/xterm';
import type { SessionTransfer } from '../../api';
import { safePaste } from './paste';

/** Edits that put outside text into xterm.js's hidden text field; paste and drop come through their own events. */
const OUTSIDE_TEXT = new Set(['insertFromPaste', 'insertFromPasteAsQuotation', 'insertFromDrop', 'insertFromYank']);

function refuse(event: Event): void {
  event.preventDefault();
  event.stopPropagation();
}

/**
 * Take the clipboard's ways into and out of the terminal in `host` before xterm.js sees them (RA-003, RA-033).
 * `allows` is asked at each event, so the session's policy holds from the moment it changes.
 * - In: a paste event or dropped text is pasted cleaned, and only while paste is on; any other edit that inserts
 *   outside text into the text field is cancelled, so nothing reaches the shell around the cleaning.
 * - Out: while copy is off, a `copy` or `cut` event (from an Edit or context menu) is cancelled before xterm.js
 *   writes the selection, and so is the context menu, for which xterm.js puts the selection in the text field.
 *   xterm.js also puts a mouse selection (on Linux) or a right-clicked one (in Firefox) in that field and selects
 *   it, which hands it to the system's primary selection; with copy off, the field is emptied and nothing selected.
 *   Called after `terminal.open`, which makes the field.
 */
export function guardClipboard(host: HTMLElement, terminal: Terminal, allows: () => SessionTransfer): void {
  const field = terminal.textarea;
  if (field !== undefined) {
    const select = field.select.bind(field);
    field.select = () => {
      if (allows().copy) select();
      else field.value = '';
    };
  }
  const pasteIn = (text: string) => {
    if (allows().paste) terminal.paste(safePaste(text));
  };
  host.addEventListener(
    'paste',
    (event) => {
      refuse(event);
      pasteIn(event.clipboardData?.getData('text/plain') ?? '');
    },
    true,
  );
  host.addEventListener(
    'drop',
    (event) => {
      refuse(event);
      pasteIn(event.dataTransfer?.getData('text/plain') ?? '');
    },
    true,
  );
  host.addEventListener(
    'beforeinput',
    (event) => {
      if (OUTSIDE_TEXT.has(event.inputType)) refuse(event);
    },
    true,
  );
  for (const type of ['copy', 'cut', 'contextmenu']) {
    host.addEventListener(
      type,
      (event) => {
        if (!allows().copy) refuse(event);
      },
      true,
    );
  }
}
