import { checksApi, type NewHeartbeat } from '$lib/checks/api';
import type { Hub, HubError } from '$lib/hub/hub';
import type { ReadingsChange, StatusChange } from '../api';
import { devicesApi } from '../client';

export interface LiveListeners {
  onStatus: (change: StatusChange) => void;
  /** The device list wants every device's readings; the detail page reads one device's through `lib/metrics`, so it leaves this out. */
  onReadings?: (change: ReadingsChange) => void;
  onBeat: (added: NewHeartbeat) => void;
  onError: (error: HubError) => void;
}

/** Listens for status changes, new readings and new heartbeats of every device; returns the function that stops them. */
export function watchLive(hub: Hub, listeners: LiveListeners): () => void {
  const { onReadings } = listeners;
  const stops = [
    devicesApi(hub).watchStatus({
      next: listeners.onStatus,
      error: listeners.onError,
      complete: () => undefined,
    }),
    ...(onReadings === undefined
      ? []
      : [devicesApi(hub).watchReadings({ next: onReadings, error: listeners.onError, complete: () => undefined })]),
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
