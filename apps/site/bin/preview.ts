// Serves apps/site/dist on 127.0.0.1 for a local look. Not a deployment: it listens on this computer only.
import { join, normalize } from "node:path";

const dist = join(import.meta.dir, "..", process.env["ANIMA_SITE_OUTPUT"] ?? "dist");
const port = Number(process.env["PORT"] ?? 4173);

const server = Bun.serve({
  hostname: "127.0.0.1",
  port,
  async fetch(request) {
    const path = new URL(request.url).pathname;
    // Vercel serves this script only on its own host; the preview answers with an empty script so that the page loads without an error.
    if (path === "/_vercel/insights/script.js") return new Response("", { headers: { "content-type": "text/javascript" } });
    const file = normalize(join(dist, path === "/" ? "index.html" : path));
    if (!file.startsWith(dist)) return new Response("Not found", { status: 404 });
    const body = Bun.file(file);
    return (await body.exists()) ? new Response(body) : new Response("Not found", { status: 404 });
  },
});

console.log(`Previewing the built site at http://${server.hostname}:${server.port}/`);
