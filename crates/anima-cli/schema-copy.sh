#!/bin/sh
# Makes the copy of the schema of K1 that the package `anima-cli` holds (INS-011, task R8a).
# contracts/schema.graphql is the only source (PLN-016): change that file, then run this
# script. The test `the_packaged_schema_is_k1` fails when the two differ.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
cp "$HERE/../../contracts/schema.graphql" "$HERE/schema.graphql"
