import type { FieldError } from '$lib/hub/error';
import { isValidHost } from './host';
import type { DeviceFormValues } from './values';

const PRIVATE_KEY_ARMOUR = /^-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----\s*$/m;
const MAX_PORT = 65535;

function validPort(text: string): boolean {
  return /^\d{1,5}$/.test(text) && Number(text) >= 1 && Number(text) <= MAX_PORT;
}

function sshErrors(values: DeviceFormValues): FieldError[] {
  const { ssh, credential } = values;
  if (!ssh.enabled) return [];
  const errors: FieldError[] = [];
  if (!validPort(ssh.port.trim())) errors.push({ field: 'ssh.port', message: 'Enter a port from 1 to 65535.' });
  if (ssh.username.trim() === '') errors.push({ field: 'ssh.username', message: 'Enter the SSH user name.' });
  if (ssh.auth === 'PRIVATE_KEY' && credential.privateKey.trim() !== '' && !PRIVATE_KEY_ARMOUR.test(credential.privateKey)) {
    errors.push({
      field: 'credential.privateKey',
      message: 'This is not a private key. It must contain a "-----BEGIN ... PRIVATE KEY-----" line.',
    });
  }
  return errors;
}

/** DEV-010, DEV-012: every reason the form cannot be saved, with stable field paths. A password credential is optional. */
export function validateDevice(values: DeviceFormValues): FieldError[] {
  const errors: FieldError[] = [];
  if (values.name.trim() === '') errors.push({ field: 'name', message: 'Enter a name.' });
  if (values.host.trim() === '') errors.push({ field: 'host', message: 'Enter a host name or IP address.' });
  else if (!isValidHost(values.host)) errors.push({ field: 'host', message: 'Enter a valid host name or IP address.' });
  return [...errors, ...sshErrors(values)];
}
