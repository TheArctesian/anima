// A test may close its context while the hub is still answering a request or socket. That one failure is the end of the
// test, not a hub fault; every other error still surfaces.

function isClosed(error: unknown): boolean {
  return error instanceof Error && error.name === 'TargetClosedError';
}

/** Runs `answer`; a rejection because the page, context or browser closed is dropped, any other is thrown on. */
export async function untilClosed(answer: () => Promise<void>): Promise<void> {
  try {
    await answer();
  } catch (error) {
    if (!isClosed(error)) throw error;
  }
}
