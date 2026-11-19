// Bundled from the Anima.ing site at 0373144 (src/lib/story/{field,shapes,timeline}.ts).
(() => {
  const CAMERA = particleGeometry.camera;
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
      const { logo, scenes } = particleGeometry.points(COUNT, random);
      this.logo = logo;
      this.scenes = scenes;
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
