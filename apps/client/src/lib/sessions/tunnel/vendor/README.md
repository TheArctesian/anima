# Vendored Apache Guacamole 1.5.0 `guacamole-common-js` (R-045)

`guacamole.js` is the Apache Guacamole 1.5.0 JavaScript client, copied without change. Do not edit it.

| | |
|---|---|
| Release | Apache Guacamole 1.5.0, source release `guacamole-client-1.5.0.tar.gz` |
| URL | <https://archive.apache.org/dist/guacamole/1.5.0/source/guacamole-client-1.5.0.tar.gz> |
| Release SHA-256 | `e44d96fd736b2e5c2d73fcc48a9424f3869b8aaa75215ebf00af4afa2d20691c` (matches Apache's `.sha256`) |
| Release signature | `guacamole-client-1.5.0.tar.gz.asc`: good signature from key `F467 E54A CC52 F1D2 7788 2686 5B29 77AE E5E4 518F` (Michael Jumper, CODE SIGNING KEY, `mjumper@apache.org`), listed in <https://downloads.apache.org/guacamole/KEYS>, checked 2026-11-04 |
| Taken from | `guacamole-client-1.5.0/guacamole-common-js/src/main/webapp/modules/*.js` (34 files) |
| Licence | Apache License 2.0; each module keeps its licence header |

## How the file is built

The 34 module files are joined in byte order of their names (`LC_ALL=C`), as the build script of the npm
repackaging does, and one line is appended so that the client can import it as an ES module:

```sh
cd guacamole-client-1.5.0/guacamole-common-js/src/main/webapp/modules
LC_ALL=C cat $(ls *.js | LC_ALL=C sort) > guacamole.js
echo 'export default Guacamole;' >> guacamole.js
```

`vendor.test.ts` pins the SHA-256 of the file without its last line, so any edit fails the tests.

## Rules

- Only `$lib/sessions/tunnel/` imports this folder (ESLint `no-restricted-imports`).
- `guacamole.d.ts` is hand-written and covers only what the client calls; keep it true to 1.5.0.
- The quality checks skip `guacamole.js` (`quality/generated.txt`); ESLint ignores it.
