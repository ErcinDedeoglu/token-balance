# OpenSpec

`specs/` is current truth. `changes/` is one unit of work.

Approved product design (not an OpenSpec spec): `docs/design/`. Do not dump the whole product into `specs/`.

If `openspec/config.yaml` is missing, run `openspec init` at the repo root. Do not invent a parallel spec system.

- Do not spec the whole product.
- Record deltas in `openspec/changes/<name>/`.
- Archive merges deltas into `openspec/specs/<domain>/`.
