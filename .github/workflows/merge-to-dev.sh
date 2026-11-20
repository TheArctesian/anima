#!/bin/sh
# The step of merge-to-dev.yml, in the checkout of `dev`: one step of the merge train (R-252).
# It needs GH_TOKEN. It ends with 0 and changes nothing when the rule is off. It never starts
# the gates of a branch again and never starts the gates of `dev`: the train tests the commit
# that `dev` becomes (quality/bin/train.ts).
set -eu

if ! grep -Eq '"mergeWithoutCheck"[[:space:]]*:[[:space:]]*true' quality/merge-rule.json ||
  ! grep -Eq '"fullGates"[[:space:]]*:[[:space:]]*"github"' quality/merge-rule.json; then
  echo "The rule of dev is off (quality/merge-rule.json): nothing is merged."
  exit 0
fi

git config user.name "anima merge"
git config user.email "merge@anima.invalid"
bun quality/bin/train.ts
