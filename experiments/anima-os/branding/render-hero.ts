// Render the existing anima-site hero at rest; do not recreate its particle geometry.
// Usage: bun render-hero.ts /path/to/anima-site > /tmp/anima-hero.svg
import { pathToFileURL } from "node:url";
const source = process.argv[2];
if (!source) throw new Error("Pass the anima-site checkout path");
const { ParticleField } = await import(pathToFileURL(`${source}/src/lib/story/field.ts`).href);
Object.assign(globalThis, { devicePixelRatio: 2, innerWidth: 1920, innerHeight: 1080 });
let rectangles: string[] = [];
const context = {
  fillStyle: "",
  clearRect() { rectangles = []; },
  fillRect(x: number, y: number, width: number, height: number) {
    rectangles.push(`<rect x="${x.toFixed(2)}" y="${y.toFixed(2)}" width="${width.toFixed(2)}" height="${height.toFixed(2)}" fill="${this.fillStyle}"/>`);
  },
};
const canvas = { clientWidth: 1920, clientHeight: 1080, getContext: () => context };
let seed = 20261120;
const random = () => { seed = (1664525 * seed + 1013904223) >>> 0; return seed / 4294967296; };
const field = new ParticleField(canvas, random);
field.layout([{ x: 560, y: 90, width: 800, height: 800 }], 1);
field.draw(0, 0, { x: 0, y: 0 }, true);
console.log(`<svg xmlns="http://www.w3.org/2000/svg" width="3840" height="2160" viewBox="0 0 3840 2160"><rect width="3840" height="2160" fill="#1B1D21"/>${rectangles.join("")}<text x="1920" y="1850" text-anchor="middle" fill="#ECEFF4" font-family="DejaVu Sans,sans-serif" font-size="72">Anima.ing</text></svg>`);
