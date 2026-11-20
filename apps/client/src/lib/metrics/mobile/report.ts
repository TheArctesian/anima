import { enrolAndroidReporter, hasAndroidReporter, pushAndroidReadings } from '$lib/desktop/metrics/android';
import { devicesApi } from '$lib/devices/client';
import { comparable } from '$lib/hosted/this-computer';
import type { Hub } from '$lib/hub/hub';
import { metricsApi } from '$lib/metrics/api';

const REPORT_INTERVAL = 20_000;

export async function enableAndroidMetrics(hub: Hub, host: string): Promise<boolean> {
  const listed = await devicesApi(hub).deviceHosts();
  if (!listed.ok) return false;
  const matches = listed.value.filter((device) => comparable(device.host) === comparable(host));
  const [device] = matches;
  if (matches.length !== 1 || device === undefined) return false;
  const origin = hub.address.origin;
  try {
    if (await hasAndroidReporter(origin, device.id)) return pushAndroidReadings(origin);
    // An existing reporter owns its token. Enrolment would replace it.
    if (device.metricsSource !== 'NONE') return false;
    const enrolment = await metricsApi(hub).enrolDaemon(device.id);
    if (!enrolment.ok) return false;
    await enrolAndroidReporter(origin, device.id, enrolment.value.code);
    return pushAndroidReadings(origin);
  } catch {
    return false;
  }
}

export function watchAndroidMetrics(origin: string): () => void {
  let inFlight = false;
  function report(): void {
    if (document.visibilityState !== 'visible' || inFlight) return;
    inFlight = true;
    void pushAndroidReadings(origin).finally(() => { inFlight = false; });
  }
  const timer = window.setInterval(report, REPORT_INTERVAL);
  document.addEventListener('visibilitychange', report);
  report();
  return () => {
    window.clearInterval(timer);
    document.removeEventListener('visibilitychange', report);
  };
}
