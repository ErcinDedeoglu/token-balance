#!/usr/bin/env bash
# Deterministic agent-kit checker. Copied into the target repo.
# Fail closed. No network. No LLM.

set -euo pipefail

NO_WRITE=0
[[ "${AGENT_KIT_NO_WRITE:-}" == "1" ]] && NO_WRITE=1
ARGS=()
for a in "$@"; do
  if [[ "$a" == "--no-write" ]]; then
    NO_WRITE=1
  else
    ARGS+=("$a")
  fi
done
ROOT=$(cd "${ARGS[0]:-.}" && pwd)
fail=0

err() { echo "fail: $*" >&2; fail=1; }
ok() { echo "ok: $*"; }

need() {
  if [[ -e "$ROOT/$1" ]]; then ok "exists $1"; else err "missing $1"; fi
}

lines() { wc -l <"$1" | tr -d ' '; }

is_forbidden_domain() {
  case "$1" in
    utils|helpers|common|misc|shared) return 0 ;;
    *) return 1 ;;
  esac
}

need AGENTS.md
need docs/QUALITY.md
need docs/GROWTH.md
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

if [[ -f "$ROOT/.gitignore" ]] && grep -q '\.env' "$ROOT/.gitignore"; then
  ok ".gitignore has .env"
else
  err ".gitignore missing .env"
fi

root_agents="$ROOT/AGENTS.md"
while IFS= read -r f; do
  rel=${f#"$ROOT"/}
  [[ "$rel" == AGENTS.md ]] && continue
  n=$(lines "$f")
  if (( n > 200 )); then err "$rel has $n lines (nested max 200)"; else ok "$rel $n lines"; fi
  if [[ -f "$root_agents" ]] && cmp -s "$f" "$root_agents"; then
    err "$rel is a copy of root AGENTS.md (must be a delta)"
  fi
done < <(find "$ROOT" -name AGENTS.md -not -path '*/.git/*' 2>/dev/null)

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
    *) return 1 ;;
  esac
}

HAS_PRODUCT=0
shopt -s nullglob
for d in "$ROOT"/*/; do
  name=$(basename "$d")
  if kit_dir "$name"; then continue; fi
  if find "$d" -type f ! -name .gitkeep | grep -q .; then
    HAS_PRODUCT=1
    if [[ -f "$d/AGENTS.md" ]]; then
      ok "$name/ has nested AGENTS.md"
    else
      err "$name/ has product files but no AGENTS.md (read docs/GROWTH.md)"
    fi
  fi
done
shopt -u nullglob

if [[ -d "$ROOT/.claude/rules" ]]; then
  while IFS= read -r f; do
    rel=${f#"$ROOT"/}
    if ! grep -q '^paths:' "$f"; then
      err "$rel has no paths: (unscoped vendor rule)"
    else
      ok "$rel is path-scoped"
    fi
  done < <(find "$ROOT/.claude/rules" -type f -name '*.md' 2>/dev/null)
fi

if grep -R -E -n --exclude-dir=.git --exclude-dir=templates \
  'AKIA[0-9A-Z]{16}|-----BEGIN (RSA |OPENSSH )?PRIVATE KEY-----' \
  "$ROOT/AGENTS.md" "$ROOT/memory" "$ROOT/docs" "$ROOT/.github" 2>/dev/null | grep -q .; then
  err "secret-like pattern in kit files"
else
  ok "no secret-like patterns in kit files"
fi

KIT_STATE="$ROOT/tooling/agent-kit"
file_sha() {
  local f="$ROOT/$1"
  if [[ -f "$f" ]]; then
    shasum -a 256 "$f" 2>/dev/null | awk '{print substr($1,1,16)}'
  else
    echo "missing"
  fi
}

STEER_GROWTH=none
print_steer() {
  mkdir -p "$KIT_STATE/log"
  echo ""
  echo "======== AGENT KIT STEER ========"
  if ! git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    STEER_GROWTH=unknown
    echo "growth: unknown (not a git repo)"
    echo "PROMPT: Run this check from a git worktree. Follow docs/GROWTH.md and AGENTS.md MUST."
    echo "======== END STEER ========"
    return
  fi

  skip_state() {
    case "$1" in
      tooling/agent-kit/STATUS.md|tooling/agent-kit/status.json|tooling/agent-kit/log/*|tooling/agent-kit/last-steer.md|tooling/agent-kit/last.json) return 0 ;;
    esac
    return 1
  }

  local added=() modified=() deleted=() moved=()
  local st a b path top
  if git -C "$ROOT" rev-parse --verify HEAD >/dev/null 2>&1; then
    while IFS=$'\t' read -r st a b; do
      [[ -z "$st" ]] && continue
      case "$st" in
        A*) skip_state "$a" || added+=("$a") ;;
        M*) skip_state "$a" || modified+=("$a") ;;
        D*) skip_state "$a" || deleted+=("$a") ;;
        R*|C*) skip_state "$b" || moved+=("$a -> $b") ;;
      esac
    done < <(git -C "$ROOT" diff --name-status -M HEAD)
  fi
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    skip_state "$path" && continue
    added+=("$path")
  done < <(git -C "$ROOT" ls-files --others --exclude-standard)

  local head_tops=""
  if git -C "$ROOT" rev-parse --verify HEAD >/dev/null 2>&1; then
    head_tops=$(git -C "$ROOT" ls-tree -d --name-only HEAD)
  fi

  local h=0 v=0 k=0
  local p tops=()
  for p in "${added[@]+"${added[@]}"}" "${modified[@]+"${modified[@]}"}" "${moved[@]+"${moved[@]}"}"; do
    [[ "$p" == *" -> "* ]] && p=${p#* -> }
    top=${p%%/*}
    if [[ "$p" == "$top" ]]; then
      k=1
      continue
    fi
    if kit_dir "$top"; then
      k=1
      continue
    fi
    if [[ -n "$head_tops" ]] && printf '%s\n' "$head_tops" | grep -qx "$top"; then
      v=1
    else
      h=1
    fi
    tops+=("$top")
  done

  STEER_GROWTH=none
  if (( h && v )); then STEER_GROWTH=mixed
  elif (( h )); then STEER_GROWTH=horizontal
  elif (( v )); then STEER_GROWTH=vertical
  elif (( k )); then STEER_GROWTH=kit
  fi
  local growth=$STEER_GROWTH

  echo "growth: $growth"
  echo "added (${#added[@]}):"
  if ((${#added[@]})); then printf '  + %s\n' "${added[@]}"; else echo "  (none)"; fi
  echo "modified (${#modified[@]}):"
  if ((${#modified[@]})); then printf '  ~ %s\n' "${modified[@]}"; else echo "  (none)"; fi
  echo "deleted (${#deleted[@]}):"
  if ((${#deleted[@]})); then printf '  - %s\n' "${deleted[@]}"; else echo "  (none)"; fi
  echo "moved (${#moved[@]}):"
  if ((${#moved[@]})); then printf '  > %s\n' "${moved[@]}"; else echo "  (none)"; fi
  echo ""
  echo "PROMPT — follow this now (not optional):"
  case "$growth" in
    horizontal)
      echo "HORIZONTAL GROWTH: new domain/folder. Read docs/GROWTH.md."
      echo "Write nested <domain>/AGENTS.md as a delta BEFORE more product files."
      echo "Do not copy root AGENTS.md. Do not name folders utils/helpers/common/misc."
      echo "Copy memory/templates/lesson.md if this domain has a non-obvious constraint."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    vertical)
      echo "VERTICAL GROWTH: deepening an existing domain."
      echo "Stay inside that domain. One concern per file. No new top-level folders."
      echo "If you learned a non-obvious constraint, copy memory/templates/lesson.md."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    mixed)
      echo "MIXED GROWTH: split the work. Finish nested AGENTS.md for every new top-level folder"
      echo "(horizontal / docs/GROWTH.md) before deepening files (vertical)."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    kit)
      echo "KIT-ONLY change. Keep AGENTS.md ≤80 lines. Do not invent product structure here."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    *)
      echo "No file changes. If you are about to add a product folder, read docs/GROWTH.md first."
      echo "MUST: nested AGENTS.md before product files; check.sh green before claiming done."
      ;;
  esac
  echo "======== END STEER ========"

  (( NO_WRITE )) && return

  local ts date result
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  date=$(date -u +%F)
  if (( fail )); then result=failed; else result=passed; fi
  local delta=$((${#added[@]} + ${#modified[@]} + ${#deleted[@]} + ${#moved[@]}))

  local status_tmp
  status_tmp=$(mktemp)
  {
    echo "# agent-kit STATUS"
    echo ""
    echo "- result: $result"
    echo "- growth: $growth"
    echo "- added: ${#added[@]}"
    echo "- modified: ${#modified[@]}"
    echo "- deleted: ${#deleted[@]}"
    echo "- moved: ${#moved[@]}"
    echo ""
    echo "## added"
    if ((${#added[@]})); then
      local a
      for a in "${added[@]}"; do echo "- \`$a\` $(file_sha "$a")"; done
    else
      echo "- (none)"
    fi
    echo ""
    echo "## modified"
    if ((${#modified[@]})); then
      local m
      for m in "${modified[@]}"; do echo "- \`$m\` $(file_sha "$m")"; done
    else
      echo "- (none)"
    fi
    echo ""
    echo "## moved"
    if ((${#moved[@]})); then
      local mv
      for mv in "${moved[@]}"; do echo "- \`$mv\`"; done
    else
      echo "- (none)"
    fi
    echo ""
    echo "## prompt"
    echo ""
    case "$growth" in
      horizontal) echo "HORIZONTAL: nested AGENTS.md before more files. Read docs/GROWTH.md." ;;
      vertical) echo "VERTICAL: stay in domain. One concern per file." ;;
      mixed) echo "MIXED: nested AGENTS.md for new folders first, then deepen." ;;
      kit) echo "KIT-ONLY: do not invent product structure." ;;
      *) echo "No product diff. Read docs/GROWTH.md before a new folder." ;;
    esac
  } >"$status_tmp"
  if [[ ! -f "$KIT_STATE/STATUS.md" ]] || ! cmp -s "$status_tmp" "$KIT_STATE/STATUS.md"; then
    mv "$status_tmp" "$KIT_STATE/STATUS.md"
  else
    rm -f "$status_tmp"
  fi

  if (( delta > 0 || fail )); then
  {
    echo "## $ts result=$result growth=$growth +${#added[@]} ~${#modified[@]} -${#deleted[@]} >${#moved[@]}"
    if ((${#added[@]})); then printf -- '+ %s\n' "${added[@]}"; fi
    if ((${#modified[@]})); then printf -- '~ %s\n' "${modified[@]}"; fi
    if ((${#deleted[@]})); then printf -- '- %s\n' "${deleted[@]}"; fi
    if ((${#moved[@]})); then printf -- '> %s\n' "${moved[@]}"; fi
    echo ""
  } >>"$KIT_STATE/log/$date.md"
  fi

  python3 - "$KIT_STATE/status.json" "$result" "$growth" "${#added[@]}" "${#modified[@]}" "${#deleted[@]}" "${#moved[@]}" <<'PY' || true
import json, sys
from pathlib import Path
path, result, growth, na, nm, nd, nmv = sys.argv[1:8]
data = {
    "result": result,
    "growth": growth,
    "counts": {
        "added": int(na),
        "modified": int(nm),
        "deleted": int(nd),
        "moved": int(nmv),
    },
}
text = json.dumps(data, indent=2) + "\n"
p = Path(path)
if not p.exists() or p.read_text() != text:
    p.write_text(text)
PY
}

print_steer

trivial=0
if [[ -f "$ROOT/tooling/agent-kit/trivial.md" ]]; then
  tn=$(wc -c <"$ROOT/tooling/agent-kit/trivial.md" | tr -d ' ')
  if (( tn >= 40 )); then
    trivial=1
    ok "trivial.md opt-out present"
  else
    err "tooling/agent-kit/trivial.md too short (need ≥40 chars)"
  fi
fi

if (( trivial == 0 )); then
  today=$(date +%F)
  lesson_n=$(find "$ROOT/memory/lessons" -type f -name '*.md' ! -name .gitkeep 2>/dev/null | wc -l | tr -d ' ')
  case "$STEER_GROWTH" in
    horizontal|vertical|mixed)
      if [[ -f "$ROOT/memory/daily/$today.md" ]]; then
        ok "daily log for $today"
      else
        err "product diff without memory/daily/$today.md (copy templates/daily.md) or write tooling/agent-kit/trivial.md"
      fi
      ;;
  esac
  if [[ "$STEER_GROWTH" == "horizontal" ]]; then
    if (( lesson_n > 0 )); then
      ok "lesson exists for horizontal growth"
    else
      err "horizontal growth without memory/lessons/<domain>/<slug>.md or tooling/agent-kit/trivial.md"
    fi
  fi
  if (( HAS_PRODUCT )); then
    real_cmd=0
    while IFS= read -r row; do
      val=$(printf '%s' "$row" | awk -F'|' '{gsub(/^ +| +$/,"",$3); print $3}')
      if [[ -n "$val" && "$val" != "n/a" && "$val" != *"check.sh"* ]]; then
        real_cmd=1
      fi
    done < <(grep -E '^\| (Install|Dev|Test|Lint|Typecheck|Build) \|' "$ROOT/AGENTS.md" || true)
    if (( real_cmd )); then
      ok "at least one real command"
    else
      err "product folders exist but Install/Dev/Test/Lint/Typecheck/Build are all n/a (fill one or write tooling/agent-kit/trivial.md)"
    fi
  fi
fi

if (( fail )); then
  echo "agent-kit check failed" >&2
  echo "state: $KIT_STATE/STATUS.md"
  exit 1
fi
echo "agent-kit check passed"
echo "state: $KIT_STATE/STATUS.md"
