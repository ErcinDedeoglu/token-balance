# Kit file, adapter, lesson, and chain-invoke checks. Sourced by check.sh.
need AGENTS.md
need docs/QUALITY.md
need docs/GROWTH.md
need docs/SUBMODULES.md
need memory/MEMORY.md
need memory/templates/lesson.md
need memory/templates/pattern.md
need memory/templates/daily.md
need memory/lessons
need memory/patterns
need memory/daily

if [[ -f "$ROOT/AGENTS.md" ]]; then
  n=$(lines "$ROOT/AGENTS.md")
  if (( n > 80 )); then err "root AGENTS.md has $n lines (max 80)"; else ok "root AGENTS.md $n lines"; fi
  if grep -qE '^## Architecture' "$ROOT/AGENTS.md"; then
    err "root AGENTS.md has ## Architecture (Context Bloat)"
  else
    ok "root AGENTS.md has no architecture heading"
  fi
  if grep -q 'memory/templates/' "$ROOT/AGENTS.md" && grep -q 'docs/GROWTH.md' "$ROOT/AGENTS.md"; then
    ok "root AGENTS.md points at templates and GROWTH.md"
  else
    err "root AGENTS.md missing templates or GROWTH.md pointer"
  fi
fi

if [[ -f "$ROOT/memory/MEMORY.md" ]]; then
  n=$(lines "$ROOT/memory/MEMORY.md")
  b=$(wc -c <"$ROOT/memory/MEMORY.md" | tr -d ' ')
  if (( n > 200 )); then err "MEMORY.md has $n lines (max 200)"; else ok "MEMORY.md $n lines"; fi
  if (( b > 25600 )); then err "MEMORY.md is $b bytes (max 25600)"; else ok "MEMORY.md $b bytes"; fi
  if grep -qi 'session start' "$ROOT/memory/MEMORY.md"; then
    err "MEMORY.md tells agents to load at session start"
  else
    ok "MEMORY.md is on-demand"
  fi
fi

if [[ -f "$ROOT/AGENTS.md" ]]; then
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    if [[ -e "$ROOT/$path" ]]; then ok "pointer $path"; else err "broken pointer $path"; fi
  done < <(awk -F'`' '/\| `/{print $2}' "$ROOT/AGENTS.md" | grep -E '^(docs|memory|openspec|tooling)/' || true)
fi

check_adapter() {
  local name=$1
  if [[ -L "$ROOT/$name" ]]; then
    t=$(readlink "$ROOT/$name")
    if [[ "$t" == AGENTS.md ]]; then ok "$name symlink"; else err "$name symlink -> $t"; fi
  elif [[ -f "$ROOT/$name" ]]; then
    if grep -q 'AGENTS.md' "$ROOT/$name" && [[ $(lines "$ROOT/$name") -le 4 ]]; then
      ok "$name pointer"
    else
      err "$name is not a thin AGENTS.md adapter"
    fi
  fi
}

check_adapter CLAUDE.md
check_adapter GEMINI.md

while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  case "$rel" in
    CLAUDE.md|GEMINI.md) continue ;;
  esac
  in_submodule "$rel" && continue
  err "$rel is a nested adapter — CLAUDE.md and GEMINI.md exist only at the repo root and must point at AGENTS.md"
done < <(find "$ROOT" -name .git -prune -o "${SM_PRUNE[@]+"${SM_PRUNE[@]}"}" \( -name CLAUDE.md -o -name GEMINI.md -o -name claude.md -o -name gemini.md \) -print 2>/dev/null)

if [[ -f "$ROOT/.gitignore" ]] && grep -q '\.env' "$ROOT/.gitignore"; then
  ok ".gitignore has .env"
else
  err ".gitignore missing .env"
fi

root_agents="$ROOT/AGENTS.md"
while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  [[ "$rel" == AGENTS.md ]] && continue
  # Belt-and-braces: SM_PRUNE already stops the walk at submodule roots, and the
  # dedicated submodule gate decides ownership (vendor's own file vs our leak).
  in_submodule "$rel" && continue
  n=$(lines "$f")
  if (( n > 200 )); then err "$rel has $n lines (nested max 200)"; else ok "$rel $n lines"; fi
  if [[ -f "$root_agents" ]] && cmp -s "$f" "$root_agents"; then
    err "$rel is a copy of root AGENTS.md (must be a delta)"
  fi
done < <(find "$ROOT" -name .git -prune -o "${SM_PRUNE[@]+"${SM_PRUNE[@]}"}" -name AGENTS.md -print 2>/dev/null)

while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  base=$(basename "$f")
  [[ "$base" == .gitkeep ]] && continue
  dir=$(basename "$(dirname "$f")")
  parent=$(basename "$(dirname "$(dirname "$f")")")
  if [[ "$parent" != lessons ]]; then
    err "$rel must be memory/lessons/<domain>/<slug>.md"
    continue
  fi
  if is_forbidden_domain "$dir"; then
    err "$rel uses forbidden domain '$dir'"
  fi
  if [[ "$base" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2} ]]; then
    err "$rel is chronological; use a slug"
  fi
  for h in '## Situation' '## Decision' '## Reasoning' '## Outcome' '## Lesson'; do
    if ! grep -q "^$h" "$f"; then err "$rel missing $h"; fi
  done
  if ! grep -q 'Domain:' "$f"; then err "$rel missing Domain"; fi
  if ! grep -q 'Pattern-Key:' "$f"; then err "$rel missing Pattern-Key"; fi
done < <(find "$ROOT/memory/lessons" -type f -name '*.md' 2>/dev/null)

while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  base=$(basename "$f")
  [[ "$base" == .gitkeep ]] && continue
  dir=$(basename "$(dirname "$f")")
  parent=$(basename "$(dirname "$(dirname "$f")")")
  if [[ "$parent" != patterns ]]; then
    err "$rel must be memory/patterns/<domain>/<key>.md"
    continue
  fi
  if is_forbidden_domain "$dir"; then
    err "$rel uses forbidden domain '$dir'"
  fi
  if ! grep -q '^## Rule' "$f"; then err "$rel missing ## Rule"; fi
  if ! grep -q 'Domain:' "$f"; then err "$rel missing Domain"; fi
done < <(find "$ROOT/memory/patterns" -type f -name '*.md' 2>/dev/null)

while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  base=$(basename "$f")
  [[ "$base" == .gitkeep ]] && continue
  if [[ ! "$base" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}\.md$ ]]; then
    err "$rel must be memory/daily/YYYY-MM-DD.md"
  fi
done < <(find "$ROOT/memory/daily" -type f -name '*.md' 2>/dev/null)

if [[ -f "$ROOT/AGENTS.md" ]] && ! grep -q '^## MUST' "$ROOT/AGENTS.md"; then
  err "AGENTS.md missing ## MUST gate"
else
  ok "AGENTS.md has ## MUST gate"
fi

kit_dir() {
  case "$1" in
    docs|memory|openspec|tooling|.github|.git|.opencode|.claude|.githooks|node_modules|dist|build) return 0 ;;
  esac
  # repo-quality's control dir: control data, not product. Only a control dir once
  # its contract exists — a product folder could legitimately be named quality/.
  [[ "$1" == quality && -f "$ROOT/docs/QUALITY.md" ]] && return 0
  return 1
}

HAS_PRODUCT=0
shopt -s nullglob
for d in "$ROOT"/*/; do
  name=$(basename "$d")
  if kit_dir "$name"; then continue; fi
  # A submodule root is vendored, not product. It never needs a nested AGENTS.md.
  if in_submodule "$name"; then
    ok "$name/ is a submodule (vendored; not product)"
    continue
  fi
  # Deliberately ignored: the user already declared this untracked. Ignored trees
  # are not ours to gate — flagging them would punish a documented decision
  # (build output, local-dev bridges into a real submodule elsewhere).
  if git -C "$ROOT" check-ignore -q "$name" 2>/dev/null; then
    ok "$name/ is gitignored (not product)"
    continue
  fi
  # A symlink is a bridge, not a tree. Its target is gated wherever it really
  # lives; treating the link as a clone would double-report it.
  if [[ -L "${d%/}" ]]; then
    ok "$name/ is a symlink (bridge; gated at its target)"
    continue
  fi
  # A foreign repo not declared in .gitmodules. One unambiguous error: do not
  # tell the agent to scaffold into it.
  if [[ -e "$d/.git" ]]; then
    err "$name/ contains .git but is not in .gitmodules (stray clone — add it as a submodule, or gitignore it; do not scaffold into it)"
    continue
  fi
  # Count real product files only. Sidecar trees (`<submodule>.agent/`) are
  # kit-owned knowledge ABOUT a vendored repo, so a folder holding nothing but a
  # submodule and its sidecar is vendored, not product, and must not be told to
  # grow an AGENTS.md.
    if find "$d" "${SM_PRUNE[@]+"${SM_PRUNE[@]}"}" -name '*.agent' -prune -o \
        -type f ! -name .gitkeep -print 2>/dev/null | grep -q .; then
      HAS_PRODUCT=1
    fi
done
shopt -u nullglob

chain_py="$(cd "$(dirname "$0")" && pwd)/agents-chain.py"
if [[ ! -f "$chain_py" ]]; then
  err "missing $chain_py (re-run repo-scaffold init)"
elif ! command -v python3 >/dev/null 2>&1; then
  err "python3 required for the AGENTS.md chain gate"
else
  if env -u AGENT_KIT_CHAIN_TEST -u AGENT_KIT_CROWD_THRESHOLD -u AGENT_KIT_SKIP \
      python3 "$chain_py" "$ROOT"; then
    :
  else
    fail=1
  fi
fi
