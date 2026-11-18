/**
 * Empties the terminal's input box after each character typed on an on-screen keyboard (mobile plan M4-1).
 *
 * An Android keyboard sends a character as an `input` event, not as a key. xterm.js finds the character by comparing
 * the box's text before and after the event. Some keyboards put the caret back at the start of the box, so the next
 * character goes in front of the last one and the comparison sends the wrong text ("hello" arrives as "helloh" and
 * worse). An empty box makes each comparison start from nothing. The box is emptied after xterm.js has read it: its own
 * read is queued at the key-down, before this one. Text that the keyboard is still composing is left alone.
 */
export function clearAfterTyping(box: HTMLTextAreaElement): void {
  box.addEventListener('input', (event) => {
    if (!(event instanceof InputEvent) || event.isComposing || event.inputType !== 'insertText') return;
    setTimeout(() => {
      box.value = '';
    }, 0);
  });
}
