# Anima website

One static page that says what Anima is and gives the install links (SITE-001 to SITE-012, spec Section 3.12).
It is built and not deployed. The operator decided on Vercel as the host (OD-20, 2026-11-13). The domain is still open.

Plain HTML from a bun script. No form, cookie or server code (SITE-007). The only script is Vercel Web Analytics,
loaded from the site's own origin (SITE-008, OD-21). The page reuses `apps/client/src/styles/tokens.css`, three font
files and the logo at build time.

## Build

```sh
# Production build: reads the release host named by ANIMA_SITE_RELEASE_SOURCE, writes apps/site/dist
ANIMA_SITE_RELEASE_SOURCE=https://releases.anima.ing bun run --cwd apps/site build

# Local preview from the fixture feed (fake version and checksums), serves http://127.0.0.1:4173
bun run --cwd apps/site build:preview
bun run --cwd apps/site preview
```

## Settings (environment)

| Name | Default | Meaning |
| --- | --- | --- |
| `ANIMA_SITE_RELEASE_SOURCE` | none, required | Where the build reads `<folder>/latest.json` and `<folder>/<version>/SHA256SUMS`: the release host URL, for example `https://releases.anima.ing`. The layout is that of the R2 bucket (BLD-060, BLD-067). The build fails when it is unset. |
| `ANIMA_SITE_PREVIEW` | unset | Set to `1` to allow a local folder as the source (the fixture in `fixtures/releases/`). Refused when `VERCEL` is set, so fixture data cannot be published. |
| `ANIMA_SITE_RELEASE_BASE` | `https://releases.anima.ing` (DNS-007) | The origin of the download links and the only origin the feed may name. |
| `ANIMA_SITE_REPOSITORY` | the public repository | The source link. Set it to an empty text for no link. |
| `ANIMA_SITE_OUTPUT` | `dist` | The output folder, relative to `apps/site`. |

The build fails with a message, and writes nothing, when a file is missing, is not valid, names another host or
version, or has a checksum that is not 64 lower-case hex digits. The version is never in the source (SITE-003):
build again after each release.

## Deploy on Vercel (the operator does this)

`vercel.json` makes a static deployment: no framework, no function, no rewrite. It sets the build command, the output
directory `dist`, `cleanUrls`, and for every route the Content-Security-Policy (the page's own policy plus
`frame-ancestors 'none'`), `X-Content-Type-Options`, `Referrer-Policy` and a `Permissions-Policy` that turns every
feature off. No asset has a hashed name, so the file sets no long cache time.

1. From the repository root, run `vercel` in `apps/site`, or import the repository in the dashboard. Set the project
   **Root Directory** to `apps/site`, and keep "Include source files outside of the Root Directory" on: the build reads
   the tokens, fonts and logo of `apps/client` and the lock file of the repository.
2. Add the project environment variable `ANIMA_SITE_RELEASE_SOURCE=https://releases.anima.ing`. It is not a secret.
3. In the project, enable **Web Analytics** (Analytics tab). Vercel then serves `/_vercel/insights/script.js` and takes
   events at `/_vercel/insights/*` on the site's own origin. Do not enable Speed Insights. Until Web Analytics is on,
   the script path answers 404, which does no harm.
4. A release does not rebuild the site by itself. Redeploy the project after a release so that the version and the
   checksums on the page match it.
5. The domain is a separate decision (OD-20, SITE-011). Every change to the Cloudflare account or the `anima.ing` zone
   needs the operator's confirmation (DNS-008, VPS-005).
