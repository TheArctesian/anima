/**
 * When a driven thread sends a system notification, and its words (THR-059): one for each passage to a state that needs
 * the person, none while the person has that thread open in the window in front. Pure.
 */

import { folderName } from '../thread';
import type { RequestKind, Update } from './run';
import type { LiveThread } from './timeline';

/** The words of a request, as the notification and the list say them. */
export const ASK_WORDS: Readonly<Record<RequestKind, string>> = {
  command: 'Asks to run a command',
  file_change: 'Asks to change a file',
  tool: 'Asks to use a tool',
  question: 'Asks a question',
};

export interface Notice {
  readonly thread: string;
  readonly title: string;
  readonly words: string;
}

/** Where the person looks: whether the window is in front, and the thread that is open. */
export interface Looking {
  readonly front: boolean;
  readonly chosen: string | null;
}

/**
 * The notification that `update` calls for, from the thread `before` it to the thread `after` it, or `null`. A request
 * notifies only when no request of the thread waited before it; the end of a turn notifies as "Your turn" when the
 * program passes from work to idle.
 */
export function noticeFor(before: LiveThread | undefined, after: LiveThread, update: Update, looking: Looking): Notice | null {
  if (looking.front && looking.chosen === after.row.id) return null;
  const title = after.row.title === '' ? folderName(after.row.folder) : after.row.title;
  const change = update.change;
  if (change.change === 'asked' && (before?.asks.length ?? 0) === 0) {
    return { thread: after.row.id, title, words: ASK_WORDS[change.ask.kind] };
  }
  if (change.change === 'state' && change.state === 'idle' && before?.row.state === 'working') {
    return { thread: after.row.id, title, words: 'Your turn' };
  }
  return null;
}
