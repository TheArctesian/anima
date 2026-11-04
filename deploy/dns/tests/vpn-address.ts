/** True when `address` is a dotted-quad IPv4 address inside the VPN range 100.64.0.0/10 (DNS-005). */
export function isVpnAddress(address: string): boolean {
  const octets = parseIpv4(address);
  if (octets === undefined) return false;
  const [first, second] = octets;
  return first === 100 && second !== undefined && second >= 64 && second <= 127;
}

function parseIpv4(address: string): number[] | undefined {
  const parts = address.split('.');
  if (parts.length !== 4) return undefined;
  const octets: number[] = [];
  for (const part of parts) {
    if (!/^(0|[1-9]\d{0,2})$/.test(part)) return undefined;
    const octet = Number(part);
    if (octet > 255) return undefined;
    octets.push(octet);
  }
  return octets;
}
