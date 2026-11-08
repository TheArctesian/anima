import type { BrowserContext } from '@playwright/test';

// A test may close its context while the hub is still answering a request or socket. That one failure is the end of the
// test, not a hub fault; every other error still surfaces.
// playwright-core 1.65 does not name these errors `TargetClosedError` on the client: a protocol error carries only text,
// so the text is what tells them (the cookie jar of a closed context answers "Could not find browser context").

const CLOSED_TEXT = ['has been closed', 'Could not find browser context', 'Failed to find browser context'];

/** Whether `error` says the page, context or browser it needed is gone. */
export function isClosedError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  return error.name === 'TargetClosedError' || CLOSED_TEXT.some((text) => error.message.includes(text));
}

const closedContexts = new WeakSet<BrowserContext>();

/** Starts noting when `context` closes, so any error after that moment counts as the close. Call once per context. */
export function noteClosing(context: BrowserContext): void {
  context.on('close', () => closedContexts.add(context));
}

/**
 * Runs `answer`; a rejection because the page, context or browser closed is dropped, any other is thrown on. With `context`,
 * every rejection once that context has closed is dropped too (an error raised by the closing itself).
 */
export async function untilClosed(answer: () => Promise<void>, context?: BrowserContext): Promise<void> {
  try {
    await answer();
  } catch (error) {
    const contextGone = context !== undefined && closedContexts.has(context);
    if (!contextGone && !isClosedError(error)) throw error;
  }
}
