#!/bin/sh
# Makes a job of gates.yml test the branch as merged with `dev` (R-252): merges the
# commit DEV_SHA into the checkout. With an empty DEV_SHA the job tests the commit as it is.
set -eu

if [ -z "${DEV_SHA:-}" ]; then
  echo "No merge: this run tests the commit as it is."
  exit 0
fi
git fetch --no-tags origin "$DEV_SHA"
if git merge-base --is-ancestor "$DEV_SHA" HEAD; then
  echo "The branch already holds dev at $DEV_SHA."
  exit 0
fi
if ! git -c user.name="anima gates" -c user.email="gates@anima.invalid" merge --no-edit "$DEV_SHA"; then
  echo "::error::The branch does not merge with dev. Merge origin/dev into the branch and solve the conflict there."
  exit 1
fi
# The generated files are never merged (PLN-020): write them again from the merged documents.
bun quality/bin/register.ts
echo "Tested as merged with dev at $DEV_SHA."
