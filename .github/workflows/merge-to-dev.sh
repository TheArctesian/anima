#!/bin/sh
# The step of merge-to-dev.yml, in the checkout of `dev`: the steps of the merge train (R-252).
# It needs GH_TOKEN. It ends with 0 and changes nothing when the rule is off. It never starts
# the gates of a branch again and never starts the gates of `dev`: the train tests the commit
# that `dev` becomes (quality/bin/train.ts).
#
# While a train is on the way, it stays and does a step each minute, then starts this workflow
# again before the time of its job ends. The reason: the token of a workflow starts the gates of
# a train, and GitHub sends no event at the end of a run that such a token started, so nothing
# else wakes this workflow when those gates end (seen 2026-11-20, three trains).
set -eu

if ! grep -Eq '"mergeWithoutCheck"[[:space:]]*:[[:space:]]*true' quality/merge-rule.json ||
  ! grep -Eq '"fullGates"[[:space:]]*:[[:space:]]*"github"' quality/merge-rule.json; then
  echo "The rule of dev is off (quality/merge-rule.json): nothing is merged."
  exit 0
fi

git config user.name "anima merge"
git config user.email "merge@anima.invalid"

# The job has 15 minutes; a step takes less than 2.
until=$(($(date +%s) + ${TRAIN_WATCH_SECONDS:-600}))
while :; do
  bun quality/bin/train.ts
  if ! git ls-remote --heads origin 'refs/heads/train/*' | grep -q .; then
    exit 0
  fi
  if [ "$(date +%s)" -ge "$until" ]; then
    break
  fi
  sleep "${TRAIN_WATCH_PAUSE:-60}"
done
echo "A train is still on the way: this workflow starts again."
gh workflow run merge-to-dev.yml
