/** Every control character (C0, DEL, C1) but these three, which a paste legitimately holds. */
const KEPT = '\t\n\r';

/**
 * `text` as the operator may paste it (RA-003): C0 and C1 controls other than tab, line feed and carriage return are
 * removed, everything else is unchanged. An ESC in a paste could end the shell's bracketed paste early
 * (`ESC [ 201 ~`) and run what follows as typed commands.
 */
export function safePaste(text: string): string {
  return text.replace(/\p{Cc}/gu, (character) => (KEPT.includes(character) ? character : ''));
}
