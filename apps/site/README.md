# Anima website

One static page that says what Anima is and gives the install links (SITE-001 to SITE-012, spec Section 3.12).
It is built and not deployed. The host and the domain are the operator's decision (OD-20, SITE-011, VPS-005);
nobody changes a cloud account for this site without that decision.

Plain HTML from a bun script. No script, form, cookie, tracker or third-party request (SITE-007, SITE-008).
The page reuses `apps/client/src/styles/tokens.css`, three font files and the logo at build time.

## Build

```sh
# Production: reads the real release host, writes apps/site/dist
bun run --cwd apps/site build

# Local preview from the fixture feed (fake version and checksums), serves http://127.0.0.1:4173
bun run --cwd apps/site build:preview
bun run --cwd apps/site preview
```

## Settings (environment)

| Name | Default | Meaning |
| --- | --- | --- |
| `ANIMA_SITE_RELEASE_SOURCE` | the release base | Where the build reads `<folder>/latest.json` and `<folder>/<version>/SHA256SUMS`: a URL or a local folder. The layout is that of the R2 bucket (BLD-060, BLD-067). |
| `ANIMA_SITE_RELEASE_BASE` | `https://releases.anima.ing` (DNS-007) | The origin of the download links and the only origin the feed may name. |
| `ANIMA_SITE_REPOSITORY` | the public repository | The source link. Set it to an empty text for no link. |
| `ANIMA_SITE_OUTPUT` | `dist` | The output folder, relative to `apps/site`. |

The build fails with a message, and writes nothing, when a file is missing, is not valid, names another host or
version, or has a checksum that is not 64 lower-case hex digits. The fixture is in `fixtures/releases/`.
The version is never in the source (SITE-003): build again after each release.

## Headers for the host

The page carries its own Content-Security-Policy in a `meta` element. When a host is chosen, also send these headers
(`frame-ancestors` is not allowed in a `meta` element):

```text
Content-Security-Policy: default-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
```
