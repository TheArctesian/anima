export type NoticeTone = 'info' | 'success' | 'warning' | 'error';

/** An error interrupts the reader (`alert`); information, success and a warning wait their turn (`status`). */
export function noticeRole(tone: NoticeTone): 'alert' | 'status' {
  return tone === 'error' ? 'alert' : 'status';
}
