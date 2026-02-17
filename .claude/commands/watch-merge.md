---
allowed-tools: Bash(gh pr checks:*), Bash(gh pr merge:*), Bash(gh run view:*), Bash(git pull:*)
description: Watch PR checks, fix failures, and squash merge
---

## Context

- Current branch: !`git branch --show-current`
- Latest PR: !`gh pr list --author @me --limit 1 --json number,title,url --jq '.[] | "#\(.number) \(.title) \(.url)"'`

## Your task

1. Watch the most recent PR's checks using `gh pr checks <number> --watch` until they all complete
2. If all checks pass, squash merge the PR with `gh pr merge <number> --squash --delete-branch`, then `git pull`
3. If any checks fail:
   a. Inspect the failed check logs using `gh run view` to understand what went wrong
   b. Fix the issue in the code
   c. Commit and push the fix
   d. Go back to step 1 and watch the checks again
4. Repeat until all checks pass and the PR is merged
