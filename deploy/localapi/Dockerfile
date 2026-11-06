# LocalAPI proxy (D-4, VPN-005): two Tailscale LocalAPI calls, nothing else. Sources only; no installs.
FROM oven/bun:1.3.14-distroless

WORKDIR /app
COPY allow.ts server.ts ./

# Numeric, because the distroless image has no user database to look names up in.
USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/bun", "/app/server.ts"]
