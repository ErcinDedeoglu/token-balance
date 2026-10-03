# kit-refresh-invalidates-cohesion-hashes

- **Domain:** quality
- **Date:** 2026-10-03
- **Pattern-Key:** kit-refresh-invalidates-cohesion-hashes
- **Confidence:** high
- **Evidence:** `repo-scaffold` init overwrote `tooling/agent-kit/check.sh` (422 lines) and `tooling/agent-kit/submodules.sh` (330); `bash .githooks/check-quality.sh` then failed `size-split` "cohesion review hash mismatch" until `quality/cohesion-reviews.json` was re-hashed
- **Status:** active

## Situation

Refreshing the agent kit replaced both kit scripts and edited root `AGENTS.md`. Each is at its class split-review and carried a hash-bound cohesion review. The staged quality hook rejected the change as a mismatch.

## Decision

Recompute the sha256 for every reviewed file the refresh rewrote and update `quality/cohesion-reviews.json` in the same change. Add reviews for any newly added kit script that lands at/above split-review (here `check.sh`, `agents_chain_lib.py`). Rejected: regenerating the whole reviews file, or raising the size budget.

## Reasoning

`check-quality.sh` binds a `keep-cohesive` review to the staged content hash; any byte change invalidates it. A kit refresh rewrites kit-owned files that the repository never authored.

## Outcome

After re-hashing `AGENTS.md`, `submodules.sh`, and adding `check.sh` / `agents_chain_lib.py` reviews, the staged hook and the whole-tree audit pass.

## Lesson

After a repo-scaffold refresh, recompute the cohesion-review hashes for every rewritten kit file and root `AGENTS.md` in the same change; the staged quality hook fails `size-split` otherwise.

<!-- agent-kit:context:begin -->
Branch: main
Revision: 7fa5023fed6de5e1db2a2c9147d0e8d8af156924
Scope: checkout
Verification: observed — staged check-quality.sh failed on hash mismatch, then passed after re-hash
Integration: unknown
Target: unknown
Integration-evidence: none
Deployment: unknown
Owner: repository
<!-- agent-kit:context:end -->
