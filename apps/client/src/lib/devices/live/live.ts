import { checksApi, type NewHeartbeat } from '$lib/checks/api';
import type { Hub, HubError } from '$lib/hub/hub';
import { devicesApi, type StatusChange } from '../api';

export interface LiveHandlers {
  onStatus: (change: StatusChange) => void;
  onBeat: (added: NewHeartbeat) => void;
  onError: (error: HubError) => void;
}

/** Listens for status changes and new heartbeats of every device; returns the function that stops both. */
export function watchLive(hub: Hub, handlers: LiveHandlers): () => void {
  const stops = [
    devicesApi(hub).watchStatus({
      next: handlers.onStatus,
      error: handlers.onError,
      complete: () => undefined,
    }),
    checksApi(hub).watchHeartbeats({
      next: handlers.onBeat,
      error: handlers.onError,
      complete: () => undefined,
    }),
  ];
  return () => {
    for (const stop of stops) stop();
  };
}
