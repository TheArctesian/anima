import type { FieldError } from '$lib/hub/hub';
import { isValidPort } from '../values';
import { isValidMac, type RemoteValues } from './values';

/** DEV-010, RA-031: every reason the remote part of the form cannot be saved. A password is optional. */
export function validateRemote(values: RemoteValues): FieldError[] {
  const errors: FieldError[] = [];
  const { rdp, vnc } = values;
  if (rdp.enabled) {
    if (!isValidPort(rdp.port.trim())) errors.push({ field: 'rdp.port', message: 'Enter a port from 1 to 65535.' });
    if (rdp.username.trim() === '') errors.push({ field: 'rdp.username', message: 'Enter the RDP user name.' });
  }
  if (vnc.enabled && !isValidPort(vnc.port.trim())) errors.push({ field: 'vnc.port', message: 'Enter a port from 1 to 65535.' });
  if (!isValidMac(values.macAddress)) {
    errors.push({ field: 'macAddress', message: 'Enter six pairs of hex digits, such as aa:bb:cc:dd:ee:ff.' });
  }
  return errors;
}
