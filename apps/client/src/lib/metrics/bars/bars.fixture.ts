import type { CardReadingsFragment } from '$lib/hub/generated/graphql';

export const GPU = { index: 1, name: 'RTX 4090', vendor: 'NVIDIA', hasVram: true } as const;

/** Readings of a device with no GPU; `over` changes some. */
export function readings(over: Partial<CardReadingsFragment> = {}): CardReadingsFragment {
  return { at: '2026-01-01T00:00:00Z', cpuPercent: 10, memoryPercent: 20, diskPercent: 30, diskMount: '/', otherDrives: 0, vramPercent: null, vramGpu: null, ...over };
}

/** jsdom has no Web Animations: a Svelte transition ends at once. Call once at the top of a component test. */
export function endTransitionsAtOnce(): void {
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: function animate() {
      const playback = { onfinish: null as (() => void) | null, cancel: () => undefined, finish: () => undefined, currentTime: 0 };
      queueMicrotask(() => playback.onfinish?.());
      return playback;
    },
  });
}
