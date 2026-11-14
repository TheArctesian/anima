/**
 * The doors mark as points (UR-7, D13): five nested door frames receding to the right, standing on a hill, with the
 * lines that join each frame to the next. Coordinates are in mark units, about -1 to 1 across; `depth` runs from 0
 * (the nearest frame) to 4 (the farthest), and the drawing colours and fades each point by it.
 */
export interface DoorDot {
  readonly x: number;
  readonly y: number;
  readonly depth: number;
  /** Where the point is in its slow drift, in radians. */
  readonly phase: number;
}

/** Each frame's left, right and top edge, and the thickness of its jamb. */
const FRAMES = [
  [-1, 1, -1, 0.205],
  [-0.564, 0.728, -0.549, 0.135],
  [-0.267, 0.544, -0.195, 0.092],
  [-0.077, 0.41, 0.01, 0.06],
  [-0.01, 0.338, 0.082, 0.036],
] as const;

/** The ground line: level at the left, dipping to a ledge in the middle, rising again to the right. */
function hillAt(x: number): number {
  if (x <= -0.85) return 1;
  if (x < 0.2) return 1 - 0.55 * Math.sin((((x + 0.85) / 1.05) * Math.PI) / 2);
  if (x < 0.4) return 0.45;
  return Math.min(1, 0.45 + 0.55 * ((x - 0.4) / 0.4) ** 1.4);
}

type Segment = readonly [number, number, number, number, number];

function segments(): Segment[] {
  const lines: Segment[] = [];
  FRAMES.forEach(([left, right, top, jamb], depth) => {
    for (const inset of [0, jamb]) {
      const l = left + inset;
      const r = right - inset;
      const t = top + inset;
      lines.push([l, hillAt(l), l, t, depth], [l, t, r, t, depth], [r, t, r, hillAt(r), depth]);
    }
    const next = FRAMES[depth + 1];
    if (next !== undefined) {
      lines.push([left + jamb, top + jamb, next[0], next[2], depth + 0.5], [right - jamb, top + jamb, next[1], next[2], depth + 0.5]);
    }
  });
  const step = 0.05;
  for (let x = -0.9; x < 0.8; x += step) lines.push([x, hillAt(x), x + step, hillAt(x + step), 1]);
  return lines;
}

/** `count` points spread evenly along the mark's lines; `random` gives numbers in [0, 1). */
export function doorDots(count: number, random: () => number = Math.random): DoorDot[] {
  const lines = segments();
  const lengths = lines.map(([x1, y1, x2, y2]) => Math.hypot(x2 - x1, y2 - y1));
  const total = lengths.reduce((sum, length) => sum + length, 0);
  return Array.from({ length: count }, () => {
    let along = random() * total;
    let index = 0;
    while (index < lines.length - 1 && along > (lengths[index] ?? 0)) along -= lengths[index++] ?? 0;
    const [x1, y1, x2, y2, depth] = lines[index] ?? lines[0] ?? [0, 0, 0, 0, 0];
    const share = along / (lengths[index] ?? 1);
    return { x: x1 + (x2 - x1) * share, y: y1 + (y2 - y1) * share, depth, phase: random() * Math.PI * 2 };
  });
}

/** The door the sign-in form stands in: the third frame, whose opening is about as wide as it is tall. */
const DOORWAY = 2;

/** The doorway's opening, inside its jambs, in mark units; its floor is the hill. */
export const OPENING = (() => {
  const [left, right, top, jamb] = FRAMES[DOORWAY];
  const inner = { left: left + jamb, right: right - jamb, top: top + jamb };
  return { ...inner, bottom: Math.min(hillAt(inner.left), hillAt(inner.right)) };
})();

/** Whether a point lies inside the doorway, where the form stands, so it is not drawn. */
export function inDoorway(dot: Pick<DoorDot, 'x' | 'y'>): boolean {
  const margin = 0.01;
  return dot.x > OPENING.left + margin && dot.x < OPENING.right - margin && dot.y > OPENING.top + margin && dot.y < hillAt(dot.x) - margin;
}
