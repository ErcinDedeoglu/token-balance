#!/usr/bin/env bash
# Generated from docs/QUALITY.md. Budgets and lists are parsed from the contract.
# Staged files only. Standard library only. Fail closed.
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
exec python3 - "$ROOT" <<'PY'
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[1])
CONTRACT = ROOT / "docs/QUALITY.md"
IGNORE = ROOT / ".qualityignore"
BASELINE = ROOT / "quality/size-baseline.json"
REVIEWS = ROOT / "quality/cohesion-reviews.json"
violations: list[tuple[str, str, str, str]] = []


def err(path: str, detail: str, rule: str, fix: str, section: str) -> None:
    violations.append((path, detail, rule, f"{fix} — see \"{section}\" in docs/QUALITY.md"))


def die_control(path: str, detail: str) -> None:
    err(path, detail, "control-malformed", "restore a valid control file", "Control files")
    flush(1)


def flush(code: int) -> None:
    for path, detail, rule, fix in violations:
        print(f"quality: {path} — {detail} [{rule}]", file=sys.stderr)
        print(f"         → {fix}", file=sys.stderr)
    n = len(violations)
    if n:
        print(f"{n} quality violation(s). Read docs/QUALITY.md, apply, re-stage.", file=sys.stderr)
        raise SystemExit(1)
    raise SystemExit(code)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except Exception:
        die_control(str(path.relative_to(ROOT)), "missing or unreadable")
        raise


def parse_tables(md: str) -> list[tuple[list[str], list[list[str]]]]:
    lines = md.splitlines()
    out: list[tuple[list[str], list[list[str]]]] = []
    i = 0

    def cells(line: str) -> list[str]:
        raw = [c.strip() for c in line.strip().strip("|").split("|")]
        return raw

    while i < len(lines):
        line = lines[i]
        if line.strip().startswith("|") and i + 1 < len(lines) and re.match(
            r"^\s*\|?\s*:?-{3,}", lines[i + 1]
        ):
            header = cells(line)
            i += 2
            rows: list[list[str]] = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                rows.append(cells(lines[i]))
                i += 1
            out.append((header, rows))
            continue
        i += 1
    return out


def table_by_header(tables, *names: str):
    want = [n.lower() for n in names]
    found = []
    for header, rows in tables:
        key = [h.lower() for h in header]
        if key == want:
            found.append((header, rows))
    return found


md = read_text(CONTRACT)
if not md.strip():
    die_control("docs/QUALITY.md", "empty")
tables = parse_tables(md)

budgets_t = table_by_header(tables, "class", "split_lines", "hard_lines", "hard_bytes", "hard_words")
if len(budgets_t) != 1:
    die_control("docs/QUALITY.md", "budget table missing or duplicated")
budgets: dict[str, dict[str, int]] = {}
for row in budgets_t[0][1]:
    if len(row) != 5:
        die_control("docs/QUALITY.md", "malformed budget row")
    cls, sl, hl, hb, hw = row
    if cls in budgets:
        die_control("docs/QUALITY.md", f"duplicated budget class {cls}")
    try:
        budgets[cls] = {
            "split_lines": int(sl),
            "hard_lines": int(hl),
            "hard_bytes": int(hb),
            "hard_words": int(hw),
        }
    except ValueError:
        die_control("docs/QUALITY.md", f"non-integer budget for {cls}")

path_t = table_by_header(tables, "pattern", "class")
if len(path_t) != 1:
    die_control("docs/QUALITY.md", "path-class table missing or duplicated")
path_class: list[tuple[str, str]] = []
for row in path_t[0][1]:
    if len(row) != 2:
        die_control("docs/QUALITY.md", "malformed path-class row")
    pat, cls = row
    if cls not in budgets:
        die_control("docs/QUALITY.md", f"path class {cls} has no budget")
    path_class.append((pat, cls))

mand_t = table_by_header(tables, "file")
if len(mand_t) != 1:
    die_control("docs/QUALITY.md", "mandated-root table missing or duplicated")
mandated = {row[0] for row in mand_t[0][1] if row and row[0]}
if not mandated:
    die_control("docs/QUALITY.md", "mandated root list empty")

forbidden: list[str] = []
m = re.search(r"^## Forbidden path segments\s*```(?:\w+)?\s*(.*?)```", md, re.M | re.S)
if not m:
    die_control("docs/QUALITY.md", "forbidden path segments block missing")
for line in m.group(1).splitlines():
    name = line.strip()
    if name:
        forbidden.append(name)
if not forbidden:
    die_control("docs/QUALITY.md", "forbidden path segments empty")


def expand_braces(pat: str) -> list[str]:
    m = re.search(r"\{([^{}]+)\}", pat)
    if not m:
        return [pat]
    out: list[str] = []
    for opt in m.group(1).split(","):
        out.extend(expand_braces(pat[: m.start()] + opt + pat[m.end() :]))
    return out


def glob_re(pat: str) -> re.Pattern[str]:
    i = 0
    parts: list[str] = ["^"]
    while i < len(pat):
        if pat.startswith("**", i):
            if i + 2 == len(pat):
                parts.append(".*")
                i += 2
            elif pat.startswith("**/", i):
                parts.append("(?:.*/)?")
                i += 3
            else:
                parts.append(".*")
                i += 2
        elif pat[i] == "*":
            parts.append("[^/]*")
            i += 1
        elif pat[i] == "?":
            parts.append("[^/]")
            i += 1
        else:
            parts.append(re.escape(pat[i]))
            i += 1
    parts.append("$")
    return re.compile("".join(parts))


COMPILED = [(glob_re(p), p, c) for pat, c in path_class for p in expand_braces(pat)]


def classify(rel: str) -> str | None:
    rel = rel.replace("\\", "/")
    for rx, _pat, cls in COMPILED:
        if rx.match(rel):
            return cls
    return None


def load_ignore(text: str) -> list[str]:
    pats: list[str] = []
    seen: set[str] = set()
    for line in text.splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        if s.startswith("/") or ".." in s.split("/"):
            die_control(".qualityignore", f"unsafe pattern {s}")
        if s in seen:
            die_control(".qualityignore", f"duplicated pattern {s}")
        seen.add(s)
        pats.append(s)
    return pats


ignore_pats = load_ignore(read_text(IGNORE))


def ignored(rel: str) -> bool:
    rel = rel.replace("\\", "/")
    for pat in ignore_pats:
        if pat.endswith("/"):
            if rel == pat[:-1] or rel.startswith(pat):
                return True
            continue
        if "/" not in pat and "*" not in pat and "?" not in pat:
            if rel == pat or rel.split("/")[-1] == pat:
                return True
            continue
        if glob_re(pat).match(rel) or glob_re("**/" + pat).match(rel):
            return True
    return False


def load_json(path: Path):
    try:
        data = json.loads(read_text(path))
    except json.JSONDecodeError:
        die_control(str(path.relative_to(ROOT)), "malformed JSON")
        raise
    if not isinstance(data, dict):
        die_control(str(path.relative_to(ROOT)), "JSON root must be an object")
    return data


baseline = load_json(BASELINE)
if "entries" not in baseline or not isinstance(baseline["entries"], list):
    die_control("quality/size-baseline.json", "missing entries list")
base_map: dict[str, dict[str, int]] = {}
for item in baseline["entries"]:
    if not isinstance(item, dict) or not all(k in item for k in ("path", "lines", "bytes", "words")):
        die_control("quality/size-baseline.json", "entry missing path/lines/bytes/words")
    p = item["path"]
    if p in base_map:
        die_control("quality/size-baseline.json", f"duplicated path {p}")
    try:
        base_map[p] = {"lines": int(item["lines"]), "bytes": int(item["bytes"]), "words": int(item["words"])}
    except (TypeError, ValueError):
        die_control("quality/size-baseline.json", f"non-integer counts for {p}")

reviews_data = load_json(REVIEWS)
if "reviews" not in reviews_data or not isinstance(reviews_data["reviews"], list):
    die_control("quality/cohesion-reviews.json", "missing reviews list")
reviews: dict[str, dict] = {}
for item in reviews_data["reviews"]:
    if not isinstance(item, dict):
        die_control("quality/cohesion-reviews.json", "review is not an object")
    for k in ("path", "class", "sha256", "concern", "rationale", "kind"):
        if k not in item:
            die_control("quality/cohesion-reviews.json", f"review missing {k}")
    p = item["path"]
    if p in reviews:
        die_control("quality/cohesion-reviews.json", f"duplicated path {p}")
    kind = item["kind"]
    if kind not in ("keep-cohesive", "legacy-fix"):
        die_control("quality/cohesion-reviews.json", f"bad kind for {p}")
    rationale = item["rationale"]
    if not isinstance(rationale, str) or not (40 <= len(rationale) <= 500):
        die_control("quality/cohesion-reviews.json", f"rationale length for {p}")
    sha = item["sha256"]
    if not isinstance(sha, str) or not re.fullmatch(r"[0-9a-f]{64}", sha):
        die_control("quality/cohesion-reviews.json", f"sha256 for {p}")
    reviews[p] = item

proc = subprocess.run(
    ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"],
    cwd=ROOT,
    capture_output=True,
    check=False,
)
if proc.returncode != 0:
    die_control("git", "cannot list staged files")
staged = [p for p in proc.stdout.decode().split("\0") if p]


def staged_meta(rel: str) -> tuple[str, bytes]:
    ls = subprocess.run(
        ["git", "ls-files", "--stage", "--", rel],
        cwd=ROOT,
        capture_output=True,
        check=False,
    )
    mode = "100644"
    if ls.returncode == 0 and ls.stdout:
        mode = ls.stdout.decode().split()[0]
    show = subprocess.run(
        ["git", "show", f":{rel}"],
        cwd=ROOT,
        capture_output=True,
        check=False,
    )
    if show.returncode != 0:
        die_control(rel, "cannot read staged blob")
        raise SystemExit(1)
    return mode, show.stdout


def counts(data: bytes) -> tuple[int, int, int]:
    nlines = data.count(b"\n")
    nbytes = len(data)
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        text = data.decode("latin-1")
    nwords = len(text.split())
    return nlines, nbytes, nwords


for rel in staged:
    rel_n = rel.replace("\\", "/")
    if ignored(rel_n):
        continue
    parts = [p for p in rel_n.split("/") if p]
    for part in parts:
        if part in forbidden:
            err(
                rel_n,
                f"generic name '{part}'",
                "no-generic-names",
                "rename to domain + role",
                "Forbidden path segments",
            )
    if "shared" in parts:
        idx = parts.index("shared")
        rest = parts[idx + 1 :]
        ok = rest[:1] == ["contracts"]
        if not ok:
            err(
                rel_n,
                "shared/ is not shared/contracts/",
                "shared-contracts-only",
                "move contracts under shared/contracts/ or rename the folder",
                "Forbidden path segments",
            )
    if "/" not in rel_n.rstrip("/"):
        if rel_n not in mandated:
            err(
                rel_n,
                "loose file at repository root",
                "no-loose-files",
                "move into a domain folder or add the name to Mandated root files",
                "Mandated root files",
            )
    cls = classify(rel_n)
    if cls is None:
        err(
            rel_n,
            "no path-class match",
            "unclassified-path",
            "add a pattern in Path class",
            "Path class",
        )
        continue
    mode, blob = staged_meta(rel_n)
    if mode == "120000":
        continue
    nlines, nbytes, nwords = counts(blob)
    sha = hashlib.sha256(blob).hexdigest()
    b = budgets[cls]
    over_hard_bits = []
    if nlines > b["hard_lines"]:
        over_hard_bits.append(f"{nlines} > {b['hard_lines']} lines")
    if nbytes > b["hard_bytes"]:
        over_hard_bits.append(f"{nbytes} > {b['hard_bytes']} bytes")
    if b["hard_words"] and nwords > b["hard_words"]:
        over_hard_bits.append(f"{nwords} > {b['hard_words']} words")
    base = base_map.get(rel_n)
    if over_hard_bits:
        grew = False
        if base:
            grew = nlines > base["lines"] or nbytes > base["bytes"] or nwords > base["words"]
        if not base or grew:
            rule = "size-baseline" if base and grew else "size-hard"
            err(
                rel_n,
                ("baseline-bound count grew; " if rule == "size-baseline" else "above hard maximum: ")
                + ", ".join(over_hard_bits),
                rule,
                "extract concerns into a folder; do not regenerate the baseline",
                "Size budgets",
            )
    elif base and (nlines > base["lines"] or nbytes > base["bytes"] or nwords > base["words"]):
        err(
            rel_n,
            "baseline-bound count grew",
            "size-baseline",
            "shrink or extract; do not regenerate the baseline",
            "Control files",
        )
    over_split = nlines >= b["split_lines"]
    if over_split:
        rev = reviews.get(rel_n)
        if not rev:
            err(
                rel_n,
                f"at split-review without a current cohesion review ({nlines} ≥ {b['split_lines']} lines)",
                "size-split",
                "add a keep-cohesive review or extract concerns into a folder",
                "Size budgets",
            )
        elif rev["sha256"] != sha:
            err(
                rel_n,
                "cohesion review hash mismatch",
                "size-split",
                "re-review the file (content changed)",
                "Control files",
            )
        elif rev["class"] != cls:
            err(
                rel_n,
                f"cohesion review class {rev['class']} != {cls}",
                "size-split",
                "fix the review class",
                "Control files",
            )
        elif rev["kind"] == "keep-cohesive" and over_hard_bits:
            err(
                rel_n,
                "keep-cohesive review on a file above hard maximum",
                "size-hard",
                "extract concerns into a folder",
                "Size budgets",
            )

flush(0)
PY
