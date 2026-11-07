import { graphql } from '$lib/hub/generated';
import type { WakeDeviceMutation } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const WakeDevice = graphql(`
  mutation WakeDevice($deviceId: ID!) {
    wakeDevice(deviceId: $deviceId) {
      sentAt
      relayName
    }
  }
`);

/** The hub's answer to a manual wake: when the packet left, and the relay that sent it (K1 `WakeResult`, RA-031). */
export type WakeSent = WakeDeviceMutation['wakeDevice'];

/** The wake button's call through a `Hub` (CQ-045). Failures stay the hub's typed K7 errors. */
export interface WakeApi {
  /** Ask the hub to send a Wake-on-LAN packet to `deviceId` through its site's wake relay (RA-031). */
  wakeDevice(deviceId: string): Promise<HubResult<WakeSent>>;
}

export function wakeApi(hub: Hub): WakeApi {
  return {
    wakeDevice: async (deviceId) => {
      const result = await hub.request(WakeDevice, { deviceId });
      return result.ok ? { ok: true, value: result.value.wakeDevice } : result;
    },
  };
}
