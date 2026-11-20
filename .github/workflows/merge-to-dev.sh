#!/bin/sh
# The steps of merge-to-dev.yml, in the checkout of `dev`. It needs BRANCH, HEAD_SHA, RUN_ID and
# GH_TOKEN. It ends with 0 and merges nothing when the rule is off, when the branch or `dev`
# moved, or when the branch is already in `dev`.
set -eu

if ! grep -Eq '"mergeWithoutCheck"[[:space:]]*:[[:space:]]*true' quality/merge-rule.json ||
  ! grep -Eq '"fullGates"[[:space:]]*:[[:space:]]*"github"' quality/merge-rule.json; then
  echo "The rule of dev is off (quality/merge-rule.json): nothing is merged."
  exit 0
fi

branch_now="$(git rev-parse --verify "origin/$BRANCH^{commit}" 2>/dev/null || true)"
if [ "$branch_now" != "$HEAD_SHA" ]; then
  echo "The branch moved after its gates, or is gone; a new run of the gates will come."
  exit 0
fi
if git merge-base --is-ancestor "$HEAD_SHA" HEAD; then
  echo "dev already holds the branch."
  exit 0
fi

# The commit of dev that the gates tested the branch with.
gh run download "$RUN_ID" --name tested-base --dir "$RUNNER_TEMP/tested-base"
tested="$(tr -d '[:space:]' < "$RUNNER_TEMP/tested-base/tested-base")"
dev_now="$(git rev-parse HEAD)"
if [ "$tested" != "$dev_now" ]; then
  echo "dev moved from ${tested:-nothing} to $dev_now after the gates: the gates run again with the new dev."
  gh run rerun "$RUN_ID"
  exit 0
fi

git config user.name "anima merge"
git config user.email "merge@anima.invalid"
bun quality/bin/merge.ts "$BRANCH" --push
# A push by a workflow starts no workflow: start the gates of the new dev by hand.
gh workflow run gates.yml --ref dev
