# Quality contract

Read before structural changes. Agent-file growth: `docs/GROWTH.md` (do not restate).
Agent-file shape (nested `AGENTS.md`, MEMORY caps, adapters): `tooling/agent-kit/check.sh`.

No product tree and no stack (`detect-project`: unknown). Do not invent `src/`, `app/`, or domains. `AGENTS.md` command cells stay `n/a` until that command exists and has been run once.

## Always-on

| Rule | Imperative | Verification |
|------|------------|--------------|
| nested-agents | Nested `AGENTS.md` in a product folder before any product file there | `tooling/agent-kit/check.sh` |
| no-loose-files | Root files must be on the mandated list; everything else lives in a folder | `.githooks/check-quality.sh` |
| no-generic-names | Never name a file or folder `utils`, `helpers`, `common`, or `misc` | `.githooks/check-quality.sh` |
| shared-contracts-only | `shared/` exists only as `shared/contracts/` | `.githooks/check-quality.sh` |
| size-hard | Do not exceed hard maximum for the file's class | `.githooks/check-quality.sh` |
| size-split | At or above split-review only with a current hash-bound cohesion review | `.githooks/check-quality.sh` |
| size-baseline | Baseline entries shrink-only; do not regenerate to bless growth | `.githooks/check-quality.sh` |
| unclassified-path | Every staged path matches Path class | `.githooks/check-quality.sh` |
| one-concern | One concern per file. Promote to a folder; never split by line count | enforced-by-review |
| no-invent-product-tree | Do not create a product top-level folder until stack files exist; keep the stack's name | enforced-by-review |
| commands-verified | Command cells are `n/a` or have been run once | enforced-by-review |

## Size budgets

First limit reached wins. Comments and blanks count. Split-review is lines; hard is lines + bytes + words (`0` words = no word cap).

| class | split_lines | hard_lines | hard_bytes | hard_words |
| --- | ---: | ---: | ---: | ---: |
| root-agent-entry | 60 | 80 | 8192 | 1000 |
| scoped-agent-entry | 120 | 200 | 12288 | 1500 |
| entrypoint | 150 | 250 | 20480 | 0 |
| source | 300 | 500 | 49152 | 0 |
| test | 400 | 700 | 65536 | 0 |
| doc | 300 | 600 | 32768 | 4000 |
| config | 300 | 500 | 49152 | 0 |

### Path class

First match wins. Unclassified staged files fail `[unclassified-path]`.

| pattern | class |
| --- | --- |
| AGENTS.md | root-agent-entry |
| **/AGENTS.md | scoped-agent-entry |
| CLAUDE.md | entrypoint |
| GEMINI.md | entrypoint |
| **/*.test.* | test |
| **/*_test.* | test |
| **/*.spec.* | test |
| docs/** | doc |
| memory/** | doc |
| README.md | doc |
| **/*.md | doc |
| **/*.{json,yml,yaml,toml,lock} | config |
| LICENSE-MIT | doc |
| LICENSE-APACHE | doc |
| .gitignore | config |
| .qualityignore | config |
| .githooks/** | source |
| tooling/** | source |
| .opencode/** | source |
| .github/** | doc |
| **/*.{sh,js,ts,tsx,jsx,mjs,cjs,py,go,rs,rb,java,kt} | source |

## Mandated root files

Staged files at repository root not in this list fail `[no-loose-files]`. Adding a name is a contract change in the same commit as the file.

| file |
| --- |
| AGENTS.md |
| CLAUDE.md |
| GEMINI.md |
| README.md |
| opencode.json |
| Cargo.toml |
| Cargo.lock |
| LICENSE-MIT |
| LICENSE-APACHE |
| .gitignore |
| .qualityignore |

Dot-directories (`.githooks/`, `.github/`, `.opencode/`, `.git/`) are folders, not loose files.

## Forbidden path segments

```
utils
helpers
common
misc
```

`shared` is allowed only as `shared/contracts` or `shared/contracts/**`. Otherwise `[shared-contracts-only]`.

## Concern taxonomy (enforced-by-review)

Orchestration, domain rules, validation, mapping, persistence, transport, presentation, events, policy — one per file. A description that needs "and" is a finding. Promote when: 3+ peer files, mixed interface+workflow+rules+I/O, independent change reasons, or over budget.

Dependencies inward through `<domain>/contracts/` only. Tests colocate at the level they prove. Moves update imports, tests, indexes, and links in the same change.

## Control files

| Path | Role |
| --- | --- |
| `.qualityignore` | Generated/vendored exclusions; each entry has a reason comment. Adds/broadens need user authorization. |
| `quality/size-baseline.json` | Adoption ratchet. Entries shrink-only. Never regenerate to bless growth. |
| `quality/cohesion-reviews.json` | Hash-bound `keep-cohesive` or `legacy-fix`. Content change invalidates the review. |

Missing, unreadable, duplicated, or malformed control files fail `[control-malformed]`. `.qualityignore` rejects absolute patterns and `..`.

## Exclusions

See `.qualityignore`. Current entries are generated agent-kit status/log — not unfixed violations.

## Enforcement

`.githooks/pre-commit` runs `.githooks/check-quality.sh` (staged files only, no build/test) then `tooling/agent-kit/check.sh`. `core.hooksPath=.githooks`.

Per violation: file, rule name in brackets, required fix, governing section. Ends with a pointer to this file.

## Maintenance

Update this file when the violation pattern, thresholds, Path class, mandated root list, or verification commands change. Change `.githooks/check-quality.sh` in the same commit as any hook-enforced rule change. Do not restate linter defaults. Do not copy this contract into `AGENTS.md`.
