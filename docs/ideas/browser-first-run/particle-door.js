// Bundled from the Anima.ing site at 0373144 (src/lib/story/{field,shapes,timeline}.ts).
(() => {
  // anima-site/src/lib/story/shapes.ts
  var spread = (random, amount) => (random() * 2 - 1) * amount;
  function alongSegments(segments, count, random, jitter = 0.01) {
    const lengths = segments.map(([x1, y1, z1, x2, y2, z2]) => Math.hypot(x2 - x1, y2 - y1, z2 - z1));
    const total = lengths.reduce((sum, length) => sum + length, 0);
    const points = [];
    for (let i = 0;i < count; i++) {
      let along = random() * total;
      let k = 0;
      while (k < lengths.length - 1 && along > lengths[k])
        along -= lengths[k++];
      const [x1, y1, z1, x2, y2, z2] = segments[k];
      const t = along / lengths[k];
      points.push([x1 + (x2 - x1) * t + spread(random, jitter), y1 + (y2 - y1) * t + spread(random, jitter), z1 + (z2 - z1) * t + spread(random, jitter)]);
    }
    return points;
  }
  var rect = (x, y, w, h, z = 0) => [
    [x, y, z, x + w, y, z],
    [x + w, y, z, x + w, y + h, z],
    [x + w, y + h, z, x, y + h, z],
    [x, y + h, z, x, y, z]
  ];
  function ball(centre, radius, random) {
    const r = radius * Math.cbrt(random());
    const theta = Math.acos(spread(random, 1));
    const phi = random() * Math.PI * 2;
    return [centre[0] + r * Math.sin(theta) * Math.cos(phi), centre[1] + r * Math.cos(theta), centre[2] + r * Math.sin(theta) * Math.sin(phi)];
  }
  var CAMERA = 2.6;
  var LOGO_FRAMES = [
    { left: -1, right: 1, top: -1, band: 0.205 },
    { left: -0.564, right: 0.728, top: -0.549, band: 0.135 },
    { left: -0.267, right: 0.544, top: -0.195, band: 0.092 },
    { left: -0.077, right: 0.41, top: 0.01, band: 0.06 },
    { left: -0.01, right: 0.338, top: 0.082, band: 0.036 }
  ];
  var FRAME_GAP = 0.3;
  function hillAt(x) {
    if (x <= -0.85)
      return 1;
    if (x < 0.2)
      return 1 - 0.55 * Math.sin((x + 0.85) / 1.05 * (Math.PI / 2));
    if (x < 0.4)
      return 0.45;
    return Math.min(1, 0.45 + 0.55 * ((x - 0.4) / 0.4) ** 1.4);
  }
  function doors(count, random) {
    const at = (x, y, z) => {
      const grow = (CAMERA + z) / CAMERA;
      return [x * grow, y * grow, z];
    };
    const line = (x1, y1, z1, x2, y2, z2) => [...at(x1, y1, z1), ...at(x2, y2, z2)];
    const segments = [];
    LOGO_FRAMES.forEach(({ left, right, top, band }, i) => {
      const z = i * FRAME_GAP;
      for (const inset of [0, band]) {
        const l = left + inset, r = right - inset, t = top + inset;
        segments.push(line(l, hillAt(l), z, l, t, z), line(l, t, z, r, t, z), line(r, t, z, r, hillAt(r), z));
      }
      const next = LOGO_FRAMES[i + 1];
      if (next) {
        const l = left + band, r = right - band, t = top + band;
        segments.push(line(l, t, z, next.left, next.top, z + FRAME_GAP), line(r, t, z, next.right, next.top, z + FRAME_GAP));
      }
    });
    for (let x = -0.9;x < 0.8; x += 0.05)
      segments.push(line(x, hillAt(x), FRAME_GAP, x + 0.05, hillAt(x + 0.05), FRAME_GAP));
    return alongSegments(segments, count, random);
  }
  function network(count, random) {
    const nodes = Array.from({ length: 16 }, () => {
      const theta = Math.acos(spread(random, 1));
      const phi = random() * Math.PI * 2;
      return [Math.sin(theta) * Math.cos(phi) * 0.95, Math.cos(theta) * 0.95, Math.sin(theta) * Math.sin(phi) * 0.95];
    });
    const links = [];
    nodes.forEach((a, i) => nodes.forEach((b, j) => {
      if (j > i && Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) < 1.05)
        links.push([...a, ...b]);
    }));
    const points = alongSegments(links, Math.floor(count * 0.6), random, 0.008);
    while (points.length < count)
      points.push(ball(nodes[points.length % nodes.length], 0.09, random));
    return points;
  }
  function ring(x, y, r, z = 0, steps = 18) {
    return Array.from({ length: steps }, (_, i) => {
      const a = i / steps * Math.PI * 2, b = (i + 1) / steps * Math.PI * 2;
      return [x + Math.cos(a) * r, y + Math.sin(a) * r, z, x + Math.cos(b) * r, y + Math.sin(b) * r, z];
    });
  }
  function dashboard(count, random) {
    const W = 0.92, H = 0.8;
    const cards = [
      { x: -1, y: -0.86, rhythm: 1.3, phase: 0.2, z: -0.1 },
      { x: 0.08, y: -0.86, rhythm: 2.1, phase: 1.6, z: 0.1 },
      { x: -1, y: 0.06, rhythm: 0.9, phase: 2.6, z: -0.1 },
      { x: 0.08, y: 0.06, rhythm: 1.7, phase: 4.1, z: 0.1 }
    ];
    const segments = [];
    const lights = [];
    cards.forEach(({ x, y, rhythm, phase, z }, i) => {
      segments.push(...rect(x, y, W, H, z));
      const lightX = x + 0.11, lightY = y + 0.13;
      segments.push(...ring(lightX, lightY, 0.045, z));
      lights.push([lightX, lightY, z]);
      segments.push([x + 0.22, lightY, z, x + 0.22 + 0.3 + i % 2 * 0.12, lightY, z], [x + 0.22, lightY + 0.1, z, x + 0.42, lightY + 0.1, z]);
      const graph = (t) => [x + 0.08 + t * (W - 0.16), y + H - 0.16 - (0.5 + 0.5 * Math.sin(t * Math.PI * 2 * rhythm + phase) * Math.cos(t * 2.3 + phase)) * 0.3, z];
      for (let k = 0;k < 28; k++)
        segments.push([...graph(k / 28), ...graph((k + 1) / 28)]);
      segments.push([x + 0.08, y + H - 0.12, z, x + W - 0.08, y + H - 0.12, z]);
    });
    const points = alongSegments(segments, Math.floor(count * 0.9), random, 0.007);
    while (points.length < count)
      points.push(ball(lights[points.length % lights.length], 0.035, random));
    return points;
  }
  function terminal(count, random) {
    const segments = [...rect(-1, -0.8, 2, 1.5), [-1, -0.6, 0, 1, -0.6, 0]];
    for (const x of [-0.92, -0.85, -0.78])
      segments.push(...rect(x, -0.74, 0.04, 0.06));
    segments.push([-0.85, -0.35, 0, -0.72, -0.28, 0], [-0.72, -0.28, 0, -0.85, -0.21, 0], [-0.6, -0.21, 0, 0.2, -0.21, 0]);
    for (const [y, w] of [[-0.08, 0.5], [0.08, 0.3], [0.24, 0.65], [0.4, 0.2]])
      segments.push([-0.85, y, 0, -0.85 + w * 1.6, y, 0]);
    segments.push([-0.85, 0.56, 0, -0.85, 0.48, 0]);
    return alongSegments(segments, count, random, 0.009);
  }
  function chip(count, random) {
    const segments = [...rect(-0.55, -0.55, 1.1, 1.1), ...rect(-0.32, -0.32, 0.64, 0.64, 0.12)];
    for (let i = 0;i < 8; i++) {
      const o = -0.45 + i * 0.128;
      segments.push([o, -0.55, 0, o, -0.85, 0], [o, 0.55, 0, o, 0.85, 0], [-0.55, o, 0, -0.85, o, 0], [0.55, o, 0, 0.85, o, 0]);
    }
    [0.4, 0.25, 0.44, 0.18].forEach((w, i) => segments.push([-0.22, -0.2 + i * 0.13, 0.12, -0.22 + w, -0.2 + i * 0.13, 0.12]));
    return alongSegments(segments, count, random, 0.01);
  }
  function laptop(count, random) {
    const hinge = 0.32, front = 0.6, near = -0.7;
    const segments = [...rect(-0.8, -0.82, 1.6, 1.14), ...rect(-0.71, -0.73, 1.42, 0.96)];
    segments.push([-0.8, hinge, 0, 0.8, hinge, 0], [0.8, hinge, 0, 0.98, front, near], [0.98, front, near, -0.98, front, near], [-0.98, front, near, -0.8, hinge, 0]);
    const base = (u, v) => [u * (0.8 + 0.18 * v), hinge + (front - hinge) * v, near * v];
    for (const v of [0.18, 0.32, 0.46])
      segments.push([...base(-0.82, v), ...base(0.82, v)]);
    segments.push([...base(-0.22, 0.62), ...base(0.22, 0.62)], [...base(0.22, 0.62), ...base(0.22, 0.88)], [...base(0.22, 0.88), ...base(-0.22, 0.88)], [...base(-0.22, 0.88), ...base(-0.22, 0.62)]);
    segments.push([-0.6, -0.5, 0, -0.5, -0.44, 0], [-0.5, -0.44, 0, -0.6, -0.38, 0], [-0.42, -0.38, 0, 0.1, -0.38, 0], [-0.6, -0.2, 0, -0.2, -0.2, 0]);
    return alongSegments(segments, count, random, 0.009);
  }
  function orbits(count, random) {
    const rings = [[0.95, 0.35, 0], [0.75, -0.5, 1.1], [0.55, 0.9, 2.1]];
    const points = [];
    for (let i = 0;i < count; i++) {
      if (i < count * 0.12) {
        points.push(ball([0, 0, 0], 0.16, random));
        continue;
      }
      const [radius, tilt, offset] = rings[i % 3];
      let angle = random() * Math.PI * 2;
      let lift = 0;
      if (random() < 0.25) {
        angle = Math.floor(random() * 5) / 5 * Math.PI * 2 + offset;
        lift = spread(random, 0.05);
      }
      const x = Math.cos(angle) * radius + spread(random, 0.01);
      const z = Math.sin(angle) * radius;
      points.push([x, lift * Math.cos(tilt) - z * Math.sin(tilt), lift * Math.sin(tilt) + z * Math.cos(tilt)]);
    }
    return points;
  }
  function shuffled(points, random) {
    const copy = [...points];
    for (let i = copy.length - 1;i > 0; i--) {
      const j = Math.floor(random() * (i + 1));
      [copy[i], copy[j]] = [copy[j], copy[i]];
    }
    return copy;
  }

  // anima-site/src/lib/story/timeline.ts
  var STAGES = 7;
  var END = STAGES + 0.55;
  var clamp = (value, low = 0, high = 1) => Math.min(high, Math.max(low, value));
  var ease = (t) => t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;
  var WALK_DEPTH = 3.6;
  function phaseAt(g) {
    const stage = Math.min(STAGES, Math.floor(clamp(g, 0, END)));
    const within = clamp(g, 0, END) - stage;
    if (stage === STAGES)
      return { stage, within, change: 0, depth: 0 };
    const holdEnd = stage === 0 ? 0.3 : 0.56;
    const change = clamp((within - holdEnd) / (1 - holdEnd));
    return { stage, within, change, depth: stage === 0 ? ease(change) * WALK_DEPTH : 0 };
  }
  function changeMix(stage, change, stagger) {
    if (stage === 0)
      return ease(clamp((change - 0.35 - stagger * 0.3) / 0.35));
    return ease(clamp((change - stagger * 0.3) / 0.7));
  }
  var TRACK_SCREENS = END * 1.4 + 1;

  // anima-site/src/lib/story/field.ts
  var COUNT = 7000;
  var PALETTE = [[94, 129, 172], [129, 161, 193], [136, 192, 208], [143, 188, 187], [216, 222, 233]];
  var HERO_MOST = 0.72;

  class ParticleField {
    canvas;
    context;
    logo;
    scenes;
    stagger;
    from = new Float32Array(3);
    to = new Float32Array(3);
    spots = [];
    boxes = [];
    logoMost = HERO_MOST;
    width = 0;
    height = 0;
    ratio = 1;
    scale = 1;
    constructor(canvas, random = Math.random) {
      this.canvas = canvas;
      const context = canvas.getContext("2d");
      if (context === null)
        throw new Error("This browser cannot draw on a canvas.");
      this.context = context;
      this.logo = shuffled(doors(COUNT, random), random);
      this.scenes = [network, dashboard, terminal, chip, laptop, orbits, doors].map((shape) => shuffled(shape(COUNT, random), random));
      this.stagger = Float32Array.from({ length: COUNT }, () => random());
      this.resize();
    }
    resize() {
      this.ratio = Math.min(devicePixelRatio, 2);
      const width = this.canvas.clientWidth || innerWidth, height = this.canvas.clientHeight || innerHeight;
      this.width = this.canvas.width = Math.round(width * this.ratio);
      this.height = this.canvas.height = Math.round(height * this.ratio);
      this.scale = Math.min(this.width, this.height) * (width <= 820 ? 0.26 : 0.28);
      this.layout(this.boxes, this.logoMost);
    }
    layout(boxes, logoMost = HERO_MOST) {
      this.boxes = boxes;
      this.logoMost = logoMost;
      this.spots = boxes.map((box, stage) => this.fit(box, stage === 0 ? logoMost : 1));
    }
    fit(box, most) {
      const { ratio, scale } = this;
      const room = Math.max(0, Math.min(box.width, box.height)) * ratio * 0.92;
      const perspective = clamp(room / (2 * scale), 0.12, most);
      const across = scale * perspective;
      return {
        x: ((box.x + box.width / 2) * ratio - this.width / 2) / across,
        y: ((box.y + box.height / 2) * ratio - this.height / 2) / across,
        z: CAMERA / perspective - CAMERA
      };
    }
    spot(stage) {
      return this.spots[stage] ?? (stage === 0 ? { x: 0, y: 0.9, z: 1 } : { x: 1.35, y: 0, z: 0 });
    }
    place(points, i, at, turnY, turnX, depth, out) {
      const [x, y, z] = points[i];
      const cy = Math.cos(turnY), sy = Math.sin(turnY), cx = Math.cos(turnX), sx = Math.sin(turnX);
      const rx = x * cy + z * sy, rz = -x * sy + z * cy;
      out[0] = rx + at.x;
      out[1] = y * cx - rz * sx + at.y;
      out[2] = y * sx + rz * cx + at.z - depth;
    }
    draw(g, time, pointer, still) {
      const { stage, change, depth } = phaseAt(g);
      const { context, from, to, stagger, scale } = this;
      context.clearRect(0, 0, this.width, this.height);
      const sway = still ? -0.2 : Math.sin(time / 3800) * 0.3 - 0.2;
      const lookY = pointer.x * 0.4, lookX = pointer.y * 0.25;
      const hero = stage === 0;
      const current = hero ? null : this.scenes[stage - 1];
      const next = this.scenes[stage] ?? null;
      const door = this.spot(0);
      const here = this.spot(stage), there = this.spot(stage + 1);
      const walk = depth * (CAMERA + door.z) / (CAMERA + 1);
      const tintHere = hero ? door.z - 1 : here.z, tintThere = there.z;
      const turnOf = (scene) => (scene === this.scenes.length ? 0.35 : 1) * (sway + lookY);
      for (let i = 0;i < COUNT; i++) {
        let tint = tintHere;
        if (current === null) {
          this.place(this.logo, i, door, lookY * 1.5 + (still ? 0 : Math.sin(time / 2600) * 0.08), lookX * 0.6, walk, from);
        } else {
          this.place(current, i, here, turnOf(stage), lookX, 0, from);
        }
        if (next !== null && change > 0) {
          this.place(next, i, there, turnOf(stage + 1), lookX, 0, to);
          const m = changeMix(stage, change, stagger[i]);
          for (let j = 0;j < 3; j++)
            from[j] += (to[j] - from[j]) * m;
          tint += (tintThere - tint) * m;
        }
        const perspective = CAMERA / (CAMERA + from[2]);
        if (perspective <= 0 || perspective > 9)
          continue;
        const alpha = Math.min(1, 0.45 + perspective * 0.5) * clamp((4 - perspective) / 2.5) * (i % 7 ? 0.75 : 1);
        if (alpha < 0.02)
          continue;
        const [r, gr, b] = PALETTE[clamp(Math.floor(2 - (from[2] - tint) * 1.2), 0, 4)];
        context.fillStyle = `rgba(${r},${gr},${b},${alpha})`;
        const size = (i % 11 ? 1 : 1.8) * this.ratio * Math.min(perspective, 3) * 0.9;
        context.fillRect(this.width / 2 + from[0] * scale * perspective, this.height / 2 + from[1] * scale * perspective, size, size);
      }
    }
  }

  const canvas = document.querySelector("#doors");
  if (!canvas?.getContext("2d")) return;
  const field = new ParticleField(canvas);
  const preference = matchMedia("(prefers-reduced-motion: reduce)");
  const pointer = { x: 0, y: 0 };
  let frame = 0, lastTime = 0, shown = 0, velocity = 0, visible = false;
  let priorState = "";

  function paint(time) {
    frame = 0;
    if (!visible || !canvas.clientWidth || !canvas.clientHeight) return;
    const state = document.body.dataset.door;
    const target = state === "ready" ? 1 : state === "starting" ? 0.52 : 0;
    const still = preference.matches;
    if (still || (state === "ready" && priorState !== "starting" && priorState !== "ready")) {
      shown = still && state === "starting" ? 0 : target;
      velocity = 0;
    } else {
      // Advance a critically damped spring from its current position and velocity.
      const elapsed = Math.min((time - lastTime) / 1000, 0.04);
      const offset = shown - target;
      const response = 9;
      const decay = Math.exp(-response * elapsed);
      const impulse = velocity + response * offset;
      shown = target + (offset + impulse * elapsed) * decay;
      velocity = (velocity - response * impulse * elapsed) * decay;
    }
    priorState = state;
    lastTime = time;
    const settled = Math.abs(shown - target) < 0.001 && Math.abs(velocity) < 0.01;
    if (!still && settled) shown = target;
    // The settled network is a static landmark beside the device list.
    field.draw(shown, time, pointer, still || state === "ready");
    if (!still && (!settled || state === "signin")) requestPaint();
  }

  function requestPaint() {
    if (!visible || frame) return;
    frame = requestAnimationFrame(paint);
  }

  function redraw() {
    const { clientWidth: width, clientHeight: height } = canvas;
    if (!width || !height) return;
    field.resize();
    const box = { x: 0, y: 0, width, height };
    field.layout([box, box], 1);
    requestPaint();
  }

  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible) redraw();
    else {
      cancelAnimationFrame(frame);
      frame = 0;
    }
  }).observe(canvas);
  new ResizeObserver(redraw).observe(canvas);
  new MutationObserver(requestPaint).observe(document.body, {
    attributes: true,
    attributeFilter: ["data-door"],
  });
  preference.addEventListener("change", requestPaint);
})();
