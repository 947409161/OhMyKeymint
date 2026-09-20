# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues in **`947409161/OhMyKeymint`**. Use the `gh` CLI for all operations.

## Target repository

Every `gh issue`, `gh pr`, and `gh api` invocation must name the tracker repository explicitly with `-R 947409161/OhMyKeymint` (or a full `repos/947409161/OhMyKeymint/...` path for `gh api`). Do not rely on repo inference: `origin` points at the upstream `ITxiao6666/OhMyKeymint`, which this account can only read, and which has Issues disabled, so inference resolves to the wrong repository.

## Conventions

- **Create an issue**: `gh issue create -R 947409161/OhMyKeymint --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> -R 947409161/OhMyKeymint --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list -R 947409161/OhMyKeymint --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> -R 947409161/OhMyKeymint --body "..."`
- **Apply / remove labels**: `gh issue edit <number> -R 947409161/OhMyKeymint --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> -R 947409161/OhMyKeymint --comment "..."`

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> -R 947409161/OhMyKeymint --comments` and `gh pr diff <number> -R 947409161/OhMyKeymint` for the diff.
- **List external PRs for triage**: `gh pr list -R 947409161/OhMyKeymint --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close` — each with `-R 947409161/OhMyKeymint`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42 -R 947409161/OhMyKeymint` and fall back to `gh issue view 42 -R 947409161/OhMyKeymint`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue in `947409161/OhMyKeymint`.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> -R 947409161/OhMyKeymint --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets. All operations target `947409161/OhMyKeymint`.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create -R 947409161/OhMyKeymint --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/947409161/OhMyKeymint/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/947409161/OhMyKeymint/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only, the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list -R 947409161/OhMyKeymint --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> -R 947409161/OhMyKeymint --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment <n> -R 947409161/OhMyKeymint --body "<answer>"`, then `gh issue close <n> -R 947409161/OhMyKeymint`, then append a context pointer (gist + link) to the map's Decisions-so-far.
