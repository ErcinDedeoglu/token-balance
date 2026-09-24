#!/usr/bin/env python3
"""AGENTS.md chain gate. Fail closed. No network. No opt-out."""

import fnmatch
import os
import subprocess
import sys

CROWD_THRESHOLD = 8

KIT_ROOTS = {
    "docs",
    "memory",
    "openspec",
    "tooling",
    ".github",
    ".opencode",
    ".claude",
    ".githooks",
}
SKIP_DIRS = {
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    ".turbo",
    ".git",
    "target",
    "vendor",
    "vendors",
    "vendored",
    "third_party",
    "external",
    "generated",
    ".tools",
}

# repo-quality's committed exclusion list. Exact repo-relative paths, trailing-slash
# directory names, and shell-style globs; negation unsupported. This is the
# authoritative signal that a tree is not ours to gate — several repos already ship
# one with `vendor/` in it, and a `.qualityignore` entry is a human decision the
# audit still re-checks, unlike a `.gitmodules` entry which is structural.
QUALITY_IGNORE = ".qualityignore"
FAIL_CLOSED_OWNERS = {"vendor", "vendors", "vendored", "third_party", "external", "generated"}
SKIP_FILES = {"AGENTS.md", "CLAUDE.md", "GEMINI.md", ".gitkeep"}

# repo-quality's control dir holds `size-baseline.json` and
# `cohesion-reviews.json` — control data, not product. But a product folder could
# legitimately be named `quality/`, so it only counts as a kit dir once the contract
# that owns it exists. Keyed by the top-level dir, then the marker that proves the
# kit is installed. Never blanket-skip a name a product might use.
CONDITIONAL_KIT_ROOTS = {"quality": "docs/QUALITY.md"}
conditional_kit_roots = set()


def detect_kit_roots(root):
    global conditional_kit_roots
    conditional_kit_roots = {
        name
        for name, marker in CONDITIONAL_KIT_ROOTS.items()
        if os.path.isfile(os.path.join(root, marker))
    }
    return conditional_kit_roots

fails = []


def fail(msg):
    fails.append(msg)
    print(f"fail: {msg}", file=sys.stderr)


def ok(msg):
    print(f"ok: {msg}")


def run(cmd):
    return subprocess.run(cmd, capture_output=True)


def is_git(root):
    r = run(["git", "-C", root, "rev-parse", "--is-inside-work-tree"])
    return r.returncode == 0 and r.stdout.strip() == b"true"


def submodule_paths(root):
    paths = []
    gm = os.path.join(root, ".gitmodules")
    if not os.path.isfile(gm):
        return paths
    with open(gm, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.strip()
            if line.startswith("path") and "=" in line:
                paths.append(line.split("=", 1)[1].strip().strip("/"))
    return paths


def parse_stage(root):
    files = []
    subs = list(submodule_paths(root))
    r = run(["git", "-C", root, "ls-files", "-z", "--stage"])
    if r.returncode != 0:
        return files, subs
    for rec in r.stdout.split(b"\0"):
        if not rec or b"\t" not in rec:
            continue
        meta, path = rec.split(b"\t", 1)
        mode = meta.split(b" ", 1)[0].decode()
        rel = path.decode("utf-8", "surrogateescape").replace("\\", "/").strip("/")
        if mode == "160000":
            subs.append(rel)
        else:
            files.append(rel)
    return files, subs


def list_others(root):
    r = run(["git", "-C", root, "ls-files", "-z", "--others", "--exclude-standard"])
    if r.returncode != 0:
        return []
    out = []
    for rec in r.stdout.split(b"\0"):
        if not rec:
            continue
        out.append(rec.decode("utf-8", "surrogateescape").replace("\\", "/").strip("/"))
    return out


def walk_files(root):
    out = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        rel_dir = os.path.relpath(dirpath, root).replace("\\", "/")
        if rel_dir == ".":
            rel_dir = ""
        kept = []
        for name in dirnames:
            rel = name if not rel_dir else f"{rel_dir}/{name}"
            if skipped_path(rel) or os.path.islink(os.path.join(dirpath, name)):
                continue
            kept.append(name)
        dirnames[:] = kept
        for name in filenames:
            rel = name if not rel_dir else f"{rel_dir}/{name}"
            if not skipped_path(rel):
                out.append(rel)
    return out


def under_submodule(rel, subs):
    for sm in subs:
        if rel == sm or rel.startswith(sm + "/"):
            return True
    return False


ignore_patterns = []


def load_quality_ignore(root):
    """Read the repo's own exclusion declarations. Fail closed on a malformed line:
    a pattern we cannot parse is a pattern that silently stops gating a tree."""
    global ignore_patterns
    path = os.path.join(root, QUALITY_IGNORE)
    if not os.path.isfile(path):
        return []
    patterns = []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for lineno, line in enumerate(fh, 1):
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if stripped.startswith("!") or stripped.startswith("/") or ".." in stripped.split("/"):
                fail(
                    f"{QUALITY_IGNORE}:{lineno} unsafe or unsupported pattern '{stripped}' "
                    "(only relative paths, dir names, and globs; no negation)"
                )
                continue
            patterns.append(stripped)
    ignore_patterns = patterns
    return patterns


def is_ignored(rel):
    """True when the repo has declared this path not ours to gate."""
    if not ignore_patterns:
        return False
    parts = rel.split("/")
    for pat in ignore_patterns:
        core = pat.rstrip("/")
        if not core:
            continue
        if "/" in core:
            if rel == core or rel.startswith(core + "/") or fnmatch.fnmatch(rel, core):
                return True
        elif core in parts:
            return True
        elif fnmatch.fnmatch(parts[-1], core):
            return True
    return False


def ignored_owner(rel):
    """The declared exclusion that owns this path, when it means 'not ours'."""
    if not ignore_patterns:
        return None
    parts = rel.split("/")
    for pat in ignore_patterns:
        core = pat.rstrip("/")
        if "/" in core:
            if rel == core or rel.startswith(core + "/") or fnmatch.fnmatch(rel, core):
                hit = core
            else:
                continue
        elif core in parts or fnmatch.fnmatch(parts[-1], core):
            hit = core
        else:
            continue
        for part in parts:
            if part in FAIL_CLOSED_OWNERS or part in SKIP_DIRS:
                return hit
        return None
    return None


def skipped_path(rel):
    if not rel:
        return False
    parts = rel.split("/")
    if parts[0] in KIT_ROOTS or parts[0] in conditional_kit_roots:
        return True
    for part in parts:
        if part in SKIP_DIRS or part.startswith(".") or part.endswith(".agent"):
            return True
    return False


ADAPTER_NAMES = {"CLAUDE.md", "GEMINI.md", "claude.md", "gemini.md"}


def nested_adapters(root):
    found = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        rel = os.path.relpath(dirpath, root).replace("\\", "/")
        if rel == ".":
            rel = ""
        dirnames[:] = [
            name
            for name in dirnames
            if name != ".git" and not os.path.islink(os.path.join(dirpath, name))
        ]
        if rel == "":
            continue
        for name in filenames:
            if name in ADAPTER_NAMES:
                found.append(f"{rel}/{name}")
    return found


def dirname(rel):
    if "/" not in rel:
        return ""
    return rel.rsplit("/", 1)[0]


def basename(rel):
    return rel.rsplit("/", 1)[-1]


def eligible_dir(rel, subs):
    if rel == "":
        return True
    if skipped_path(rel) or under_submodule(rel, subs):
        return False
    return True


def collect(root, index_only):
    subs = submodule_paths(root)
    if is_git(root):
        files, subs = parse_stage(root)
        subs = list(dict.fromkeys(subs))
        if not index_only:
            files = files + list_others(root)
    else:
        files = walk_files(root)
    cleaned = []
    for rel in files:
        rel = rel.replace("\\", "/").strip("/")
        if not rel or under_submodule(rel, subs) or skipped_path(rel) or is_ignored(rel):
            continue
        cleaned.append(rel)
    return cleaned, subs

