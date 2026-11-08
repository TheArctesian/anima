// The two LocalAPI calls the hub may make (D-4, VPN-005). Pure: no I/O, no globals.
// An allowed call is rebuilt from validated parts, so the raw request never reaches tailscaled.

export type Decision =
  | { kind: "forward"; method: "GET" | "POST"; upstream: string; logIp?: string }
  | { kind: "refuse"; reason: string };

export const STATUS_PATH = "/localapi/v0/status";
export const PING_PATH = "/localapi/v0/ping";

const refuse = (reason: string): Decision => ({ kind: "refuse", reason });

/** One decimal octet 0-255 without leading zeros, or null. */
function octet(part: string | undefined): number | null {
  if (part === undefined || !/^(?:0|[1-9]\d{0,2})$/.test(part)) return null;
  const value = Number(part);
  return value > 255 ? null : value;
}

/** Strict dotted quad inside 100.64.0.0/10 (the tailnet range): no leading zeros, no IPv6. */
export function tailnetIpv4(text: string): string | null {
  const [first, second, third, fourth, ...extra] = text.split(".");
  if (extra.length > 0) return null;
  const a = octet(first);
  const b = octet(second);
  const c = octet(third);
  const d = octet(fourth);
  if (a === null || b === null || c === null || d === null) return null;
  if (a !== 100 || b < 64 || b > 127) return null;
  return `${a}.${b}.${c}.${d}`;
}

function status(params: URLSearchParams): Decision {
  const entries = [...params.entries()];
  if (entries.length === 0) return { kind: "forward", method: "GET", upstream: STATUS_PATH };
  const [first] = entries;
  if (entries.length !== 1 || first?.[0] !== "peers") return refuse("status takes only peers");
  if (first[1] !== "true" && first[1] !== "false") return refuse("peers must be true or false");
  return { kind: "forward", method: "GET", upstream: `${STATUS_PATH}?peers=${first[1]}` };
}

function ping(params: URLSearchParams): Decision {
  const entries = [...params.entries()];
  if (entries.length !== 2) return refuse("ping takes exactly ip and type");
  const ips = params.getAll("ip");
  const types = params.getAll("type");
  if (ips.length !== 1 || types.length !== 1) return refuse("ping takes exactly ip and type");
  if (types[0] !== "disco") return refuse("ping type must be disco");
  const ip = tailnetIpv4(ips[0] ?? "");
  if (ip === null) return refuse("ping ip must be an IPv4 address in 100.64.0.0/10");
  return { kind: "forward", method: "POST", upstream: `${PING_PATH}?ip=${ip}&type=disco`, logIp: ip };
}

export function decide(method: string, pathname: string, params: URLSearchParams): Decision {
  if (method === "GET" && pathname === STATUS_PATH) return status(params);
  if (method === "POST" && pathname === PING_PATH) return ping(params);
  return refuse("call not allowed");
}
