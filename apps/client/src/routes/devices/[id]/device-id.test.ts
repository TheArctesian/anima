import { expect, it } from 'vitest';
import { parseDeviceId } from './device-id';

it('D-14: a UUID is a device ID, in either case', () => {
  expect(parseDeviceId('01890a5d-ac96-774b-bcce-b302099a8057')).toBe('01890a5d-ac96-774b-bcce-b302099a8057');
  expect(parseDeviceId('01890A5D-AC96-774B-BCCE-B302099A8057')).not.toBeNull();
});

it.each(['new', '', '01890a5d-ac96-774b-bcce-b302099a805', '01890a5d-ac96-774b-bcce-b302099a8057x', '../01890a5d-ac96-774b-bcce-b302099a8057'])(
  'D-14: %j is not a device ID',
  (text) => {
    expect(parseDeviceId(text)).toBeNull();
  },
);
