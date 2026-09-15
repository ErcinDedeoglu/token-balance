# control-files-need-nested-agents

- **Domain:** quality
- **Date:** 2026-09-15
- **Pattern-Key:** nested-agents-before-top-level-files
- **Confidence:** high
- **Evidence:** `tooling/agent-kit/check.sh` `kit_dir` (docs|memory|openspec|tooling|.github|.git|.opencode|.claude|.githooks|node_modules|dist|build)
- **Status:** active

## Situation

`repo-quality` installs control files at `quality/`. Agent-kit treats unknown top-level folders with files as product trees.

## Decision

Keep `quality/` (contract path). Add `quality/AGENTS.md` as a delta. Rejected: moving files into `tooling/` (hides them from the contract) or editing kit_dir (overwritten on scaffold refresh).

## Reasoning

`kit_dir` in `tooling/agent-kit/check.sh` does not list `quality`. `docs/GROWTH.md` requires nested `AGENTS.md` before product files in a new top-level folder. Closest-file rule still applies.

## Outcome

`quality/AGENTS.md` present. `tooling/agent-kit/check.sh` accepts the folder.

## Lesson

A new top-level folder needs nested `AGENTS.md` even when it only holds control files, unless it is already in agent-kit `kit_dir`.
