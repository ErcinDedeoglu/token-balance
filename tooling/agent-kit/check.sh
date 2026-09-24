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

# When invoked from a git hook, GIT_DIR / GIT_INDEX_FILE / GIT_WORK_TREE point at
# the superproject and leak into `git -C <submodule>` calls, silently breaking
# submodule gates. Drop them; ROOT is already resolved.
unset GIT_DIR GIT_INDEX_FILE GIT_WORK_TREE GIT_PREFIX

err() { echo "fail: $*" >&2; fail=1; }
ok() { echo "ok: $*"; }
# warn: real but non-blocking. Stale vendored notes mean "re-read before you
# trust this", not "the repo is broken" — blocking would train people to bypass
# the gate. Counted and re-surfaced in STEER so it cannot be silently ignored.
warns=0
WARN_LINES=()
warn() { echo "warn: $*"; warns=$((warns+1)); WARN_LINES+=("$*"); }
# note: a fact with a caveat attached; never a verdict.
note() { echo "note: $*"; }

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

# ---- submodules: foreign repos. Scaffold around them, never inside. ----
# Source of truth is .gitmodules (committed, readable even when uninitialized).
SUBMODULE_PATHS=()
if [[ -f "$ROOT/.gitmodules" ]]; then
  while IFS= read -r sm; do
    [[ -n "$sm" ]] && SUBMODULE_PATHS+=("${sm%/}")
  done < <(git -C "$ROOT" config -f .gitmodules --get-regexp '^submodule\..*\.path$' 2>/dev/null | awk '{print $2}')
fi
HAS_SUBMODULES=0
((${#SUBMODULE_PATHS[@]})) && HAS_SUBMODULES=1

# True when $1 (repo-relative path) is a submodule root or lives under one.
in_submodule() {
  local p=${1#./}
  p=${p%/}
  local sm
  for sm in "${SUBMODULE_PATHS[@]+"${SUBMODULE_PATHS[@]}"}"; do
    [[ "$p" == "$sm" || "$p" == "$sm"/* ]] && return 0
  done
  return 1
}

# Absolute-path variant for find(1) results.
abs_in_submodule() {
  local p=${1#"$ROOT"/}
  [[ "$p" == "$1" ]] && return 1
  in_submodule "$p"
}

# find(1) prune args so no walk ever descends into a foreign repo.
SM_PRUNE=()
for sm in "${SUBMODULE_PATHS[@]+"${SUBMODULE_PATHS[@]}"}"; do
  SM_PRUNE+=(-path "$ROOT/$sm" -prune -o)
done


HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck disable=SC1091
source "$HERE/kit_files.sh"
# shellcheck disable=SC1091
source "$HERE/kit_submodules.sh"
# shellcheck disable=SC1091
source "$HERE/kit_steer.sh"

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
    horizontal|vertical|mixed|vendored)
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

state_line() { (( NO_WRITE )) || echo "state: $KIT_STATE/STATUS.md"; }

if (( fail )); then
  echo "agent-kit check failed" >&2
  state_line
  exit 1
fi
if (( warns )); then
  echo "agent-kit check passed with $warns warning(s):"
  printf '  ! %s\n' "${WARN_LINES[@]}"
  echo "Warnings do not block a commit. They mean a recorded fact may no longer be true — verify before relying on it."
else
  echo "agent-kit check passed"
fi
state_line
