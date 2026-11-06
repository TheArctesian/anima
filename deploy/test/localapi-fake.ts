// A fake tailscaled for localapi-server.test.ts, run in its own process: `bun localapi-fake.ts <endless|silent> <socket>`.
const [behaviour, socketPath] = process.argv.slice(2);
if (socketPath === undefined) throw new Error("socket path missing");

const endless = (): Response =>
  new Response(
    new ReadableStream({
      async pull(controller) {
        controller.enqueue(new Uint8Array(64 * 1024));
        await Bun.sleep(0);
      },
    }),
  );

Bun.serve({
  unix: socketPath,
  fetch: () => (behaviour === "endless" ? endless() : new Promise<Response>(() => undefined)),
});
