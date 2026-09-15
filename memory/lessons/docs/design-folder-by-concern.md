# design-folder-by-concern

- **Domain:** docs
- **Date:** 2026-09-15
- **Pattern-Key:** design-docs-split-by-concern
- **Confidence:** high
- **Evidence:** `docs/QUALITY.md` Size budgets class `doc` (600 lines / 32768 bytes); approved design was 1341 lines / 91KB
- **Status:** active

## Situation

The approved remaining-quota TUI design was a single ~1341-line markdown file. `docs/**` is QUALITY class `doc`, hard-capped at 600 lines / 32KB. Putting the monolith at `docs/design.md` would fail `[size-hard]`. Punching a larger Path class for one file would bless an un-split document.

## Decision

Promote to `docs/design/` and split **by concern** (product, theme, layout, domain, providers, PR plan, …). Index in `docs/design/README.md`. Rejected: one file + quality-ignore; `part-2` suffixes; dumping the whole product into `openspec/specs/`.

## Reasoning

QUALITY one-concern: “Promote to a folder; never split by line count.” OpenSpec README says do not spec the whole product; `docs/design/` is the approved design, `openspec/changes/` is implementation deltas.

## Outcome

Each `docs/design/*.md` is under the `doc` split-review (300 lines) and hard cap. Root `AGENTS.md` and `README.md` point at the folder.

## Lesson

Put oversized design docs in `docs/design/` as one file per concern. Do not land a monolith under `docs/**` and do not raise the `doc` budget for a single file.
