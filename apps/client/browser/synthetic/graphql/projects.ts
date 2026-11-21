// The projects of the synthetic hub: a few made-up repositories with what the hub found in them (PRO-002, PRO-004). The
// gallery shoots the Projects page from them, and the picture of the Projects tile of the Extensions page.

const FOUND_AT = '2026-11-01T09:00:00Z';
const SYNCED_AT = '2026-11-06T00:55:00Z';

interface Sample {
  readonly name: string;
  readonly description: string;
  readonly signs: readonly (readonly [kind: string, sign: string, path: string])[];
  readonly private?: boolean;
}

const SAMPLES: readonly Sample[] = [
  { name: 'example/site', description: 'The public web site.', signs: [['CLOUDFLARE', 'WRANGLER_CONFIG', 'wrangler.toml']] },
  { name: 'example/anima', description: 'Devices, sessions and checks.', signs: [['DESKTOP_APPLICATION', 'TAURI_CONF', 'apps/desktop/src-tauri/tauri.conf.json'], ['SERVER', 'DOCKERFILE', 'deploy/hub/Dockerfile']] },
  { name: 'example/blog', description: 'Notes and posts.', signs: [['VERCEL', 'VERCEL_JSON', 'vercel.json']] },
  { name: 'example/homelab', description: 'The services of the home network.', signs: [['SERVER', 'COMPOSE_FILE', 'compose.yaml']], private: true },
  { name: 'example/scratch', description: 'Small trials.', signs: [] },
];

function project(sample: Sample, index: number): Record<string, unknown> {
  const id = `0190f000-0000-7000-8000-00000000000${String(index + 1)}`;
  const slug = sample.name.split('/')[1] ?? sample.name;
  const repository = {
    id: `${id}-r`,
    fullName: sample.name,
    url: `https://github.com/${sample.name}`,
    defaultBranch: 'main',
    private: sample.private === true,
    archived: false,
    fork: false,
    description: sample.description,
    pushedAt: SYNCED_AT,
    leftOut: false,
    gone: false,
    access: 'READY',
  };
  const resources = sample.signs.map(([kind, sign, path], each) => ({
    id: `${id}-x${String(each)}`,
    kind,
    state: each === 0 ? 'CONFIRMED' : 'FOUND_IN_REPOSITORY',
    evidence: [{ sign, path }],
    foundAt: FOUND_AT,
    confirmedAt: each === 0 ? FOUND_AT : null,
  }));
  return { id, slug, name: slug, subjectKey: sample.name, repositories: [repository], resources, syncedAt: SYNCED_AT };
}

/** The `projects` field: the sample projects, none left out. */
export function projectConnection({ leftOut }: { leftOut?: boolean | null }): Record<string, unknown> {
  const nodes = leftOut === true ? [] : SAMPLES.map(project);
  return {
    languages: [],
    edges: nodes.map((node, index) => ({ cursor: String(index), node })),
    pageInfo: { hasNextPage: false, endCursor: null },
  };
}
