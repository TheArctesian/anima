#!/bin/sh
# The rustc wrapper of .cargo/config.toml: runs the compiler through sccache when sccache is
# on PATH, and directly when it is not, so a machine without sccache builds as before.
if command -v sccache >/dev/null 2>&1; then
  exec sccache "$@"
fi
exec "$@"
