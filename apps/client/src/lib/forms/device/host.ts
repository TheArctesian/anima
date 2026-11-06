const MAX_HOST_LENGTH = 253;
const MAX_LABEL_LENGTH = 63;

/** A host as stored: trimmed, and an IPv6 literal without its brackets. */
export function normalizeHost(raw: string): string {
  const host = raw.trim();
  const inner = host.startsWith('[') && host.endsWith(']') ? host.slice(1, -1) : host;
  return inner.includes(':') ? inner : host;
}

function isIpv4(host: string): boolean {
  const octets = host.split('.');
  return octets.length === 4 && octets.every((octet) => /^(0|[1-9]\d{0,2})$/.test(octet) && Number(octet) <= 255);
}

function isIpv6Group(group: string): boolean {
  return /^[0-9A-Fa-f]{1,4}$/.test(group);
}

/** Groups on one side of a `::` split; an IPv4 tail counts as two groups. Undefined when malformed. */
function countGroups(side: string, allowIpv4Tail: boolean): number | undefined {
  if (side === '') return 0;
  const parts = side.split(':');
  const last = parts[parts.length - 1] ?? '';
  const tail = allowIpv4Tail && last.includes('.');
  if (tail && !isIpv4(last)) return undefined;
  const groups = tail ? parts.slice(0, -1) : parts;
  return groups.every(isIpv6Group) ? groups.length + (tail ? 2 : 0) : undefined;
}

function isIpv6(host: string): boolean {
  if (!host.includes(':') || !/^[0-9A-Fa-f:.]+$/.test(host)) return false;
  const sides = host.split('::');
  if (sides.length > 2) return false;
  const [head = '', rest] = sides;
  if (rest === undefined) return countGroups(head, true) === 8;
  const before = countGroups(head, false);
  const after = countGroups(rest, true);
  return before !== undefined && after !== undefined && before + after <= 7;
}

function isDnsName(host: string): boolean {
  if (host.length > MAX_HOST_LENGTH) return false;
  const labels = host.split('.');
  const labelOk = (label: string): boolean =>
    label.length <= MAX_LABEL_LENGTH && /^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?$/.test(label);
  const last = labels[labels.length - 1] ?? '';
  return labels.every(labelOk) && !/^\d+$/.test(last);
}

/** DEV-010: whether `raw` is an IPv4 or IPv6 literal or a DNS name (D-3). */
export function isValidHost(raw: string): boolean {
  const host = normalizeHost(raw);
  return host !== '' && (isIpv4(host) || isIpv6(host) || isDnsName(host));
}
