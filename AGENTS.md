# AGENTS.md

**Precedence:** closest `AGENTS.md` wins. Explicit user prompts override files.

## Commands

| Task | Command |
|------|---------|
| Install | n/a |
| Dev | `cargo run -p token-balance --bin token-balance -- --fixture mixed` |
| Test | `cargo test -p token-balance` |
| Lint | n/a |
| Typecheck | n/a |
| Build | `cargo build -p token-balance` |
| Quality | `.githooks/check-quality.sh` |
| Agent kit | `tooling/agent-kit/check.sh` |

Leave `n/a` until the command exists and has been run once. Never guess. Agent kit is not `n/a`.

## Pointers

| Need | Read |
|------|------|
| Design | `docs/design/` |
| Quality | `docs/QUALITY.md` |
| Growth | `docs/GROWTH.md` |
| Memory index | `memory/MEMORY.md` |
| Templates | `memory/templates/` |
| Lessons | `memory/lessons/` |
| Specs | `openspec/specs/` |
| Active change | `openspec/changes/` |

## MUST

These are gates, not suggestions. Git pre-commit and the OpenCode plugin enforce 1–2.

1. Nested `AGENTS.md` in a product folder **before** any product file there.
2. `tooling/agent-kit/check.sh` green before claiming done (paste output). Then read `tooling/agent-kit/STATUS.md`. Red = not done.
3. Non-obvious constraint → copy `memory/templates/lesson.md` to `memory/lessons/<domain>/`.
4. Read `docs/GROWTH.md` before adding a root-level folder.

## Boundaries

### Ask first
- New dependencies
- Public API, schema, or migration changes
- CI, secrets, production, force-push
- Promoting a lesson into this file

### Never
- Commit secrets or `.env`
- Edit generated or vendored trees
- Duplicate docs into this file
- Auto-write rules here without user approval
- Name files or folders `utils`, `helpers`, `common`, `misc`

## Memory loop

1. Capture → copy `memory/templates/daily.md` to `memory/daily/YYYY-MM-DD.md`
2. Distill → copy `memory/templates/lesson.md` to `memory/lessons/<domain>/<slug>.md`
3. After 3 recurrences of one Pattern-Key → copy `memory/templates/pattern.md`; propose one rule here; wait for approval
4. Prune stale entries; stale is worse than missing

Quality contract: `docs/QUALITY.md` — read before structural changes.
Growth contract: `docs/GROWTH.md` — split by domain before this file grows.
