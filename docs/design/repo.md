# Repo establishment (locked)

Keep agent-kit at repo root. **Single crate** in a Cargo workspace. Nested `AGENTS.md` files are created **before** any other product file (`Cargo.toml` and `.rs` alike).

```text
token-balance/                         # existing root AGENTS.md stays thin
  AGENTS.md
  Cargo.toml                           # workspace; mandated root list + same commit
  Cargo.lock                           # mandated list + .qualityignore (size)
  LICENSE-MIT                          # mandated list + Path class doc
  LICENSE-APACHE                       # mandated list + Path class doc
  .gitignore                           # add /target/
  crates/
    AGENTS.md                          # REQUIRED first file under crates/
    token-balance/
      AGENTS.md                        # REQUIRED before Cargo.toml / .rs
      Cargo.toml
      src/
        main.rs                        # both bins: token-balance and tb
        theme.rs                       # Ember Ledger tokens
        domain.rs                      # QuotaWindow, snapshot, sort, Clock, apply_fetch
        layout.rs                      # breakpoints, scroll, card grid
        tui.rs                         # ratatui app + overlay
        fixtures.rs                    # canned snapshots
        providers.rs                   # object-safe trait + registry
```

**Do not** name anything `utils`, `helpers`, `common`, or `misc`.

This `docs/design/` folder is kit (`docs/` is `kit_dir`). It does **not** create a product tree. `crates/` is the product tree and waits for PR1.

## QUALITY.md + `.qualityignore` in PR1

Same commit as the first `Cargo.toml` and LICENSE files:

1. Add `Cargo.toml`, `Cargo.lock`, `LICENSE-MIT`, and `LICENSE-APACHE` to **Mandated root files** (`[no-loose-files]`).
2. Add Path class entries `LICENSE-MIT` and `LICENSE-APACHE` → `doc` (they have no extension; without this they fail `[unclassified-path]`). Apache-2.0 text is ~200 lines — under the doc hard cap.
3. Authorize this `.qualityignore` entry (reason comment required):

```gitignore
# Cargo.lock is generated. Config class is 500 lines / 49KB; a real lockfile
# is thousands of lines. size-baseline.json is shrink-only and cannot absorb
# dependency PRs. Authorized by token-balance design 2026-09-15.
Cargo.lock
```

Do **not** bless lockfile growth via `quality/size-baseline.json`.

## agent-kit / `trivial.md`

`HAS_PRODUCT` is already 1 because of `quality/`. `tooling/agent-kit/trivial.md` currently skips the product-command and daily-log gates. PR1 sequence:

1. Write `crates/AGENTS.md` and `crates/token-balance/AGENTS.md` (empty crate tree otherwise).
2. Add workspace/crate `Cargo.toml` (`license`, both `[[bin]]`s), `LICENSE-MIT`, `LICENSE-APACHE`, `src/main.rs`, `.gitignore` `/target/`, QUALITY (mandated list + Path class) + `.qualityignore` changes.
3. Run `cargo run -p token-balance --bin token-balance -- --version` and `cargo run -p token-balance --bin tb -- --version` once.
4. Fill root `AGENTS.md` `Dev` / `Build` cells with the commands that were actually run (no new headings; stay ≤80 lines). Update the AGENTS.md cohesion-review sha256 in the same commit.
5. **Delete** `tooling/agent-kit/trivial.md` (it is no longer true).
6. `memory/daily/YYYY-MM-DD.md` for the product diff (kit requires it once trivial.md is gone).

Nested `AGENTS.md` files are **deltas**, not copies of root (kit check compares with `cmp -s`).

`crates/AGENTS.md` (workspace delta): workspace cargo commands once run; never live network from fixture tests; never commit `target/`.

`crates/token-balance/AGENTS.md` (crate delta): `cargo run -p token-balance --bin token-balance` / `--bin tb`; `cargo test -p token-balance` once run; never log secrets; never call live endpoints until an adapter PR; never fake 0% for `NotConfigured`.

If `tui.rs` approaches the 300-line split-review, promote by **concern** (e.g. extract overlay/card widgets into `cards.rs`), not by line-count suffixes. Do not pre-create that file.

Rust tests: colocate `#[cfg(test)]` in the same file. If a source file would exceed budget because of tests, extract to `src/domain_test.rs` (QUALITY class `test` via `**/*_test.*`) and `#[cfg(test)] #[path = "domain_test.rs"] mod tests;`. Do **not** put integration tests under `tests/*.rs` without a Path-class discussion — those match `**/*.rs` → `source`, not `test`.

**Live adapters:** start in `src/adapters/` from the first adapter PR (PR5). Avoid the “two files at crate root, third promotes” dance. `src/adapters/` is not a top-level product folder, so it does not need its own `AGENTS.md`. Trait + registry stay in `providers.rs`.
