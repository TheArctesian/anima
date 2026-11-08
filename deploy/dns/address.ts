type Ipv4 = readonly [number, number, number, number];

/** A CIDR block as its base octets and prefix length. */
type Block = readonly [Ipv4, number];

const VPN_RANGE: Block = [[100, 64, 0, 0], 10];

/** Ranges that are never a VPS public address: RFC 1918, CGNAT, loopback, link-local, documentation and reserved. */
const NON_PUBLIC: readonly Block[] = [
  [[0, 0, 0, 0], 8],
  [[10, 0, 0, 0], 8],
  [[100, 64, 0, 0], 10],
  [[127, 0, 0, 0], 8],
  [[169, 254, 0, 0], 16],
  [[172, 16, 0, 0], 12],
  [[192, 0, 0, 0], 24],
  [[192, 0, 2, 0], 24],
  [[192, 168, 0, 0], 16],
  [[198, 18, 0, 0], 15],
  [[198, 51, 100, 0], 24],
  [[203, 0, 113, 0], 24],
  [[224, 0, 0, 0], 4],
  [[240, 0, 0, 0], 4],
];

/** One decimal octet 0–255 without leading zeros, or undefined. */
function octet(part: string | undefined): number | undefined {
  if (part === undefined || !/^(0|[1-9]\d{0,2})$/.test(part)) return undefined;
  const value = Number(part);
  return value > 255 ? undefined : value;
}

/** Parses a strict dotted quad: four decimal octets 0–255 without leading zeros. */
export function parseIpv4(address: string): Ipv4 | undefined {
  const [first, second, third, fourth, ...extra] = address.split('.');
  if (extra.length > 0) return undefined;
  const a = octet(first);
  const b = octet(second);
  const c = octet(third);
  const d = octet(fourth);
  if (a === undefined || b === undefined || c === undefined || d === undefined) return undefined;
  return [a, b, c, d];
}

function toNumber(octets: Ipv4): number {
  return octets.reduce((value, octet) => value * 256 + octet, 0);
}

function inBlock(octets: Ipv4, [base, prefix]: Block): boolean {
  const size = 2 ** (32 - prefix);
  return Math.floor(toNumber(octets) / size) === Math.floor(toNumber(base) / size);
}

/** True when `address` is a strict dotted quad inside the VPN range 100.64.0.0/10 (DNS-005). */
export function isVpnAddress(address: string): boolean {
  const octets = parseIpv4(address);
  return octets !== undefined && inBlock(octets, VPN_RANGE);
}

/** True when `address` is a strict dotted quad outside every non-public range (DNS-004). */
export function isPublicAddress(address: string): boolean {
  const octets = parseIpv4(address);
  return octets !== undefined && !NON_PUBLIC.some((block) => inBlock(octets, block));
}
