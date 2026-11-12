import { DEVICE_LIST_SCREENS } from './screens-devices';
import { DEVICE_PAGE_SCREENS } from './screens-device-page';
import { SESSION_SCREENS } from './screens-sessions';
import { SETTINGS_SCREENS } from './screens-settings';
import { SIGN_IN_SCREENS, STATE_SCREENS } from './screens-states';
import { VPN_SCREENS } from './screens-vpn';
import type { Screen } from './shots';

/** Every screen, in the order that numbers its pictures. A screen added in the middle moves the numbers after it. */
export const SCREENS: readonly Screen[] = [
  ...SIGN_IN_SCREENS,
  ...DEVICE_LIST_SCREENS,
  ...DEVICE_PAGE_SCREENS,
  ...SETTINGS_SCREENS,
  ...VPN_SCREENS,
  ...SESSION_SCREENS,
  ...STATE_SCREENS,
];
