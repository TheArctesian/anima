import { readLink } from '$lib/desktop/link/shell';
import { deviceLinkAddress } from '$lib/link/devices';

const KEY = 'anima.enrollment.public-link';

/** Carries only the public link identity through the sign-in page, scoped to the link's origin. */
export async function markPendingEnrolment(storage: Storage): Promise<void> {
  const link = await readLink();
  if (!link.ok || !link.value.joined || link.value.state !== 'up') return;
  const { port, publicKey } = link.value;
  const origin = port === undefined ? undefined : deviceLinkAddress(port)?.origin;
  if (origin !== undefined && publicKey !== undefined) storage.setItem(KEY, `${origin} ${publicKey}`);
}

export function pendingEnrolment(storage: Storage, origin: string): string | undefined {
  const saved = storage.getItem(KEY);
  const prefix = `${origin} `;
  return saved?.startsWith(prefix) ? saved.slice(prefix.length) : undefined;
}

export function clearPendingEnrolment(storage: Storage): void {
  storage.removeItem(KEY);
}
