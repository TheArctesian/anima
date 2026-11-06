// The LocalAPI allow list (D-4, VPN-005): the two allowed calls and every refused shape.
import { describe, expect, test } from "bun:test";
import { decide } from "../localapi/allow.ts";

const call = (method: string, path: string, query = "") => decide(method, path, new URLSearchParams(query));
const forwarded = (method: string, path: string, query = "") => {
  const decision = call(method, path, query);
  return decision.kind === "forward" ? decision.upstream : `refused: ${decision.reason}`;
};
const refused = (method: string, path: string, query = "") => call(method, path, query).kind;

describe("VPN-005 status call", () => {
  test("VPN-005 GET status without a query is forwarded", () => {
    expect(forwarded("GET", "/localapi/v0/status")).toBe("/localapi/v0/status");
  });

  test("VPN-005 GET status with peers=true or peers=false is forwarded", () => {
    expect(forwarded("GET", "/localapi/v0/status", "peers=true")).toBe("/localapi/v0/status?peers=true");
    expect(forwarded("GET", "/localapi/v0/status", "peers=false")).toBe("/localapi/v0/status?peers=false");
  });

  test.each(["peers=yes", "peers=", "peers=true&peers=false", "peers=true&x=1", "debug=1", "peers=TRUE"])(
    "VPN-005 status with query %p is refused",
    (query) => expect(refused("GET", "/localapi/v0/status", query)).toBe("refuse"),
  );

  test.each(["POST", "PUT", "DELETE", "HEAD", "get"])("VPN-005 %s on status is refused", (method) =>
    expect(refused(method, "/localapi/v0/status")).toBe("refuse"),
  );
});

describe("D-4 ping call", () => {
  test("D-4 POST ping with a tailnet ip and type=disco is forwarded, rebuilt from parts", () => {
    expect(forwarded("POST", "/localapi/v0/ping", "type=disco&ip=100.64.0.7")).toBe("/localapi/v0/ping?ip=100.64.0.7&type=disco");
    expect(forwarded("POST", "/localapi/v0/ping", "ip=100.127.255.254&type=disco")).toBe("/localapi/v0/ping?ip=100.127.255.254&type=disco");
  });

  test.each(["ip=100.63.255.255", "ip=100.128.0.1", "ip=10.0.0.1", "ip=100.064.0.1", "ip=100.64.0.01", "ip=100.64.0", "ip=100.64.0.1.1",
    "ip=100.64.0.256", "ip=fd7a:115c:a1e0::1", "ip=%3A%3A1", "ip=10.64.0.1", "ip=192.100.0.1", "ip=101.64.0.1", "ip=100.64.0.1/32", "ip=+100.64.0.1", "ip=1e2.64.0.1", "ip=", "ip=100.64.0.1%20"])(
    "D-4 ping with %p and type=disco is refused",
    (ipQuery) => expect(refused("POST", "/localapi/v0/ping", `${ipQuery}&type=disco`)).toBe("refuse"),
  );

  test.each(["type=disco", "ip=100.64.0.1", "ip=100.64.0.1&type=icmp", "ip=100.64.0.1&type=TSMP", "ip=100.64.0.1&type=disco&size=9000",
    "ip=100.64.0.1&ip=100.64.0.2&type=disco", "ip=100.64.0.1&type=disco&type=disco", "ip=100.64.0.1&type=disco&ip=100.64.0.1", ""])(
    "D-4 ping with query %p is refused",
    (query) => expect(refused("POST", "/localapi/v0/ping", query)).toBe("refuse"),
  );

  test.each(["GET", "PUT", "DELETE"])("D-4 %s on ping is refused", (method) =>
    expect(refused(method, "/localapi/v0/ping", "ip=100.64.0.1&type=disco")).toBe("refuse"),
  );
});

describe("D-4 other paths never reach tailscaled", () => {
  test.each([
    "/localapi/v0/prefs", "/localapi/v0/logout", "/localapi/v0/ping/", "/localapi/v0/status/", "//localapi/v0/status",
    "/localapi/v0/../v0/prefs", "/localapi/v0/status/..", "/localapi/v0/%73tatus", "/localapi/v0/status%2f", "/localapi/v0/%2e%2e/status",
    "/localapi//v0/status", "/localapi/v0/status;x", "/LOCALAPI/V0/STATUS", "/localapi/v0/", "/", "", "localapi/v0/status",
    "/localapi/v0/status\u0000", "/localapi/v0/status ",
  ])("D-4 path %p is refused for every method", (path) => {
    for (const method of ["GET", "POST", "PUT", "DELETE", "PATCH"]) {
      expect(refused(method, path, "ip=100.64.0.1&type=disco")).toBe("refuse");
    }
  });

  test("D-4 an allowed ping reports the ip for the log, a status call reports none", () => {
    const ping = call("POST", "/localapi/v0/ping", "ip=100.64.0.9&type=disco");
    const status = call("GET", "/localapi/v0/status");
    expect(ping.kind === "forward" ? ping.logIp : "none").toBe("100.64.0.9");
    expect(status.kind === "forward" ? status.logIp : "none").toBeUndefined();
  });
});
