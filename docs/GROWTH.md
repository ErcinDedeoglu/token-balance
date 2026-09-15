# Growth contract

Split by **domain** (bounded context), then by **concern**. Never by line count, date, or technical layer (`controllers/`, `services/`, `utils/`).

Read before adding a root rule, lesson, spec, or path-scoped rule. Product-tree SoC: `docs/QUALITY.md` (or the `repo-quality` skill). This file owns **agent-file** growth only.

`AGENTS.md` is the portable instruction file. Every coding agent should read it. Harness-specific files are adapters, not a second source of truth.

## Target shape

```text
AGENTS.md                                 # global defaults only
<product>/<domain>/AGENTS.md              # delta: commands/boundaries that differ
memory/MEMORY.md                          # one-line index
memory/lessons/<domain>/<slug>.md
memory/patterns/<domain>/<key>.md
openspec/specs/<domain>/
docs/QUALITY.md
docs/GROWTH.md
```

`<product>/` keeps the stack name (`src/`, `app/`, `packages/`). `<domain>/` is a bounded context, not a layer.

Beside every nested `AGENTS.md`, add a one-line adapter (symlink to that `AGENTS.md`) only if a harness in use cannot load nested `AGENTS.md`. Do not copy rules into the adapter. Do not invent a parallel rule tree per vendor.

## Triggers — split that class now

| Artifact | Split when |
|----------|------------|
| Root `AGENTS.md` | ≥60 lines, **or** a rule applies to only one domain |
| Nested `AGENTS.md` | A root-level product folder has files, **or** that domain has different commands/Never-do |
| `memory/lessons/` | 3+ lessons share one domain |
| `openspec/specs/` | A capability belongs to one domain |
| Product file | Two concerns or over budget → `docs/QUALITY.md`, not this file |

If root is long from duplication, **delete first**. Nesting is not a filing cabinet.

After any split or new lesson/pattern/daily file, run `tooling/agent-kit/check.sh`. A red check is a failed change.

## How — AGENTS.md

1. Root keeps repo-wide commands, global Never, and pointers.
2. Move domain-only rules into `<product>/<domain>/AGENTS.md` as a **delta** — do not copy root.
3. Closest `AGENTS.md` wins. User prompt overrides files.
4. File-type conventions that differ by directory belong in that nested `AGENTS.md`, not in a vendor rules folder.

## How — memory

1. Copy `memory/templates/lesson.md` → `memory/lessons/<domain>/<slug>.md`. Unknown domain → `memory/lessons/_unsorted/<slug>.md`, sort within a week.
2. Index: `- [slug](lessons/<domain>/<slug>.md) — <domain>/<pattern-key> — one line`
3. Never chronological names. Never one growing `lessons.md`.

## How — specs

`openspec/specs/<domain>/`. Changes stay `openspec/changes/<name>/`.

## SoC ladder

| Level | Splits | Wrong |
|-------|--------|-------|
| Repo | domains | one AGENTS.md owns every package CLI |
| Domain | concepts | billing lesson under `auth/` |
| File | one concern | lesson file is a week of chat |
| Nested AGENTS.md | one domain’s delta | restating root, or a vendor-only rules dump |

## Huge repos

- Hot layer (root AGENTS.md) stays ≤80 lines. Cold files load on demand.
- Do not pre-create nested files. Add the nested file the first time a domain rule would pollute root.
- Large monorepos nest AGENTS.md per package because commands differ — not to store essays.

## Never

- Nested AGENTS.md that restates root
- A second instruction body in a vendor file (keep adapters as pointers/symlinks)
- Chronological memory filenames
- `utils`, `helpers`, `common`, `misc` as domain names
- Split by line count or `part-2` suffixes
- Auto-write root AGENTS.md without user approval
