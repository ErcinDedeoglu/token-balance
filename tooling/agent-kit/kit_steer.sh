# Growth classification and STATUS write. Sourced by check.sh.
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
  # --no-write must write NOTHING. Creating the state dir here would violate
  # that on every read-only run (pre-commit hook, or a check against a repo we
  # do not own). Deferred to the write section below.
  (( NO_WRITE )) || mkdir -p "$KIT_STATE/log"
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

  local h=0 v=0 k=0 s=0
  local p tops=()
  for p in "${added[@]+"${added[@]}"}" "${modified[@]+"${modified[@]}"}" "${moved[@]+"${moved[@]}"}"; do
    [[ "$p" == *" -> "* ]] && p=${p#* -> }
    # A submodule gitlink shows up as a plain path change. It is vendored,
    # never product growth.
    if in_submodule "$p"; then
      s=1
      continue
    fi
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
  elif (( s )); then STEER_GROWTH=vendored
  elif (( k )); then STEER_GROWTH=kit
  fi
  # A submodule change riding along with product work still must be reported.
  local vendored_touched=$s
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
      echo "At 8 direct product entries, write nested AGENTS.md and link ## Chain up and down BEFORE more files."
      echo "Below 8, the nearest ancestor AGENTS.md covers the folder. Do not copy root."
      echo "Do not name folders utils/helpers/common/misc."
      echo "Copy memory/templates/lesson.md if this domain has a non-obvious constraint."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done. --no-verify does not skip the chain gate."
      ;;
    vertical)
      echo "VERTICAL GROWTH: deepening an existing domain."
      echo "Stay inside that domain. One concern per file. No new top-level folders."
      echo "If you learned a non-obvious constraint, copy memory/templates/lesson.md."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    mixed)
      echo "MIXED GROWTH: split the work. Link ## Chain for every folder at the crowd threshold"
      echo "(docs/GROWTH.md) before deepening files (vertical)."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done. --no-verify does not skip the chain gate."
      ;;
    kit)
      echo "KIT-ONLY change. Keep AGENTS.md ≤80 lines. Do not invent product structure here."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    vendored)
      echo "VENDORED CHANGE: only submodule pointers moved. Submodules are foreign repos."
      echo "Do NOT write inside them. Record old sha -> new sha and why."
      echo "Run tooling/agent-kit/submodules.sh, then log the bump in memory/daily/."
      echo "Re-run tooling/agent-kit/check.sh. Red = not done."
      ;;
    *)
      echo "No file changes. If you are about to add a product folder, read docs/GROWTH.md first."
      echo "MUST: crowded folder (8 direct entries) has AGENTS.md and ## Chain; check.sh green before claiming done."
      ;;
  esac
  if (( vendored_touched )) && [[ "$growth" != vendored ]]; then
    echo ""
    echo "ALSO: a submodule pointer moved in this change. It is a foreign repo —"
    echo "never write inside it. Run tooling/agent-kit/submodules.sh and note the sha bump."
  fi
  echo "======== END STEER ========"

  (( NO_WRITE )) && return

  mkdir -p "$KIT_STATE/log"

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
      horizontal) echo "HORIZONTAL: at crowd threshold 8, nested AGENTS.md plus ## Chain. Read docs/GROWTH.md." ;;
      vertical) echo "VERTICAL: stay in domain. One concern per file." ;;
      mixed) echo "MIXED: ## Chain for every folder at the crowd threshold, then deepen." ;;
      kit) echo "KIT-ONLY: do not invent product structure." ;;
      vendored) echo "VENDORED: submodule pointer moved. Never edit inside it. Record old->new sha." ;;
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
    if ((${#added[@]})); then printf '+ %s\n' "${added[@]}"; fi
    if ((${#modified[@]})); then printf '~ %s\n' "${modified[@]}"; fi
    if ((${#deleted[@]})); then printf -- '- %s\n' "${deleted[@]}"; fi
    if ((${#moved[@]})); then printf '> %s\n' "${moved[@]}"; fi
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
