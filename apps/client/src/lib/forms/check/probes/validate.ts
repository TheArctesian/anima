import type { FieldError } from '$lib/hub/error';
import { isValidHost } from '$lib/forms/device/host';
import type { PendingCheckType } from '$lib/forms/hub-pending';
import { lengthError, portError, timeoutError } from '../model/range';
import type { ProbeValues } from './values';

/** CHK-012, defence in depth: the characters Docker allows in a container name. The hub is the real guard. */
const CONTAINER_NAME = /^[a-zA-Z0-9][a-zA-Z0-9_.-]*$/;
/**
 * CHK-012: the characters systemd allows in a unit name. A bare name ("nginx") is accepted, as
 * `systemctl` reads it as a service. It cannot start with "-", so it is never read as an option.
 */
const UNIT_NAME = /^[A-Za-z0-9:_.@\\][A-Za-z0-9:_.@\\-]*$/;
/** A dotted numeric OID with at least two arcs, such as 1.3.6.1.2.1.1.3.0. */
const OID = /^\d+(\.\d+)+$/;
const NAME_BYTES = 255;
const SECRET_BYTES = 256;
const EXPECTED_BYTES = 1024;

function dockerErrors({ container, timeoutMs }: ProbeValues['docker']): FieldError[] {
  const name = container.trim();
  const found: FieldError[] =
    name === ''
      ? [{ field: 'container', message: 'Enter the container name.' }]
      : !CONTAINER_NAME.test(name)
        ? [{ field: 'container', message: 'A container name has letters, digits, "_", "." and "-" only, and starts with a letter or digit.' }]
        : lengthError('container', name, NAME_BYTES, 'The container name');
  return [...found, ...timeoutError(timeoutMs)];
}

function systemdErrors({ unit, timeoutMs }: ProbeValues['systemd']): FieldError[] {
  const name = unit.trim();
  const found: FieldError[] =
    name === ''
      ? [{ field: 'unit', message: 'Enter the unit name, such as nginx.service.' }]
      : !UNIT_NAME.test(name)
        ? [{ field: 'unit', message: 'A unit name has letters, digits and ":", "_", ".", "@", "\\" and "-" only, and does not start with "-".' }]
        : lengthError('unit', name, NAME_BYTES, 'The unit name');
  return [...found, ...timeoutError(timeoutMs)];
}

function tailscaleErrors({ target, timeoutMs }: ProbeValues['tailscalePing']): FieldError[] {
  const text = target.trim();
  const found: FieldError[] =
    text !== '' && !isValidHost(text) ? [{ field: 'target', message: 'Enter a host name or IP address, or leave it empty to use the device host.' }] : [];
  return [...found, ...timeoutError(timeoutMs)];
}

/** A stored secret may stay empty ("keep") while the version is the stored one; a new one must be entered. */
function secretError(field: string, text: string, label: string, snmp: ProbeValues['snmp']): FieldError[] {
  if (text === '') {
    return snmp.storedVersion === snmp.version ? [] : [{ field, message: `Enter the ${label}.` }];
  }
  return lengthError(field, text, SECRET_BYTES, `The ${label}`);
}

function snmpErrors(snmp: ProbeValues['snmp'], configSent: boolean): FieldError[] {
  const oid = snmp.oid.trim();
  const errors: FieldError[] = [
    ...portError(snmp.port),
    ...timeoutError(snmp.timeoutMs),
    ...lengthError('expected', snmp.expected.trim(), EXPECTED_BYTES, 'The expected value'),
  ];
  if (oid === '') errors.push({ field: 'oid', message: 'Enter the OID, such as 1.3.6.1.2.1.1.3.0.' });
  else if (!OID.test(oid)) errors.push({ field: 'oid', message: 'Enter the OID as dotted numbers, such as 1.3.6.1.2.1.1.3.0.' });
  if (!configSent) return errors;
  if (snmp.version === 'V2C') return [...errors, ...secretError('community', snmp.community, 'community', snmp)];
  if (snmp.username.trim() === '') errors.push({ field: 'username', message: 'Enter the user name.' });
  return [
    ...errors,
    ...secretError('authPassword', snmp.authPassword, 'authentication password', snmp),
    ...secretError('privacyPassword', snmp.privacyPassword, 'privacy password', snmp),
  ];
}

/** Section 6.4 (Phase 2 types): every reason the settings cannot be saved. Secrets are checked only when the config is sent. */
export function probeErrors(values: ProbeValues, type: PendingCheckType, configSent: boolean): FieldError[] {
  switch (type) {
    case 'DOCKER':
      return dockerErrors(values.docker);
    case 'SYSTEMD':
      return systemdErrors(values.systemd);
    case 'SNMP':
      return snmpErrors(values.snmp, configSent);
    case 'TAILSCALE_PING':
      return tailscaleErrors(values.tailscalePing);
  }
}
