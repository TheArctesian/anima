import { checksApi, type NewHeartbeat } from '$lib/checks/api';
import type { Hub, HubError } from '$lib/hub/hub';
import type { StatusChange } from '../api';
import { devicesApi } from '../client';

export interface LiveListeners {
  onStatus: (change: StatusChange) => void;
  onBeat: (added: NewHeartbeat) => void;
  onError: (error: HubError) => void;
}

/** Listens for status changes and new heartbeats of every device; returns the function that stops both. */
export function watchLive(hub: Hub, listeners: LiveListeners): () => void {
  const stops = [
    devicesApi(hub).watchStatus({
      next: listeners.onStatus,
      error: listeners.onError,
      complete: () => undefined,
    }),
    checksApi(hub).watchHeartbeats({
      next: listeners.onBeat,
      error: listeners.onError,
      complete: () => undefined,
    }),
  ];
  return () => {
    for (const stop of stops) stop();
  };
}
