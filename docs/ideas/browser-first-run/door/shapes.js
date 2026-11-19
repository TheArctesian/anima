// Bundled from the Anima.ing site at 0373144 (src/lib/story/{field,shapes,timeline}.ts).
const particleGeometry = (() => {
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

  return {
    camera: CAMERA,
    points(count, random) {
      const logo = shuffled(doors(count, random), random);
      const scenes = [network, dashboard, terminal, chip, laptop, orbits, doors].map((shape) => shuffled(shape(count, random), random));
      return { logo, scenes };
    },
  };
})();
