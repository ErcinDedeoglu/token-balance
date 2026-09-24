def ignored(root, rel):
    if not is_git(root):
        return False
    return run(["git", "-C", root, "check-ignore", "-q", "--", rel]).returncode == 0


def rel_link(from_dir, target):
    start = from_dir if from_dir else "."
    return os.path.relpath(target, start).replace("\\", "/")


def resolve_link(from_dir, link):
    link = link.strip().replace("\\", "/")
    if not link or link.startswith("/") or link.startswith("~"):
        return None
    base = from_dir.split("/") if from_dir else []
    parts = []
    for comp in link.split("/"):
        if comp in ("", "."):
            continue
        if comp == "..":
            if parts:
                parts.pop()
            elif base:
                base.pop()
            else:
                return None
        else:
            parts.append(comp)
    return "/".join(base + parts)


def parse_chain(text):
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    starts = [i for i, line in enumerate(lines) if line.strip() == "## Chain"]
    if not starts:
        return None
    if len(starts) > 1:
        return {"error": "more than one ## Chain section"}
    body = []
    for line in lines[starts[0] + 1 :]:
        if line.startswith("## "):
            break
        body.append(line.strip())
    up = None
    downs = []
    none_down = False
    illegal = []
    for line in body:
        if not line:
            continue
        if line == "- Up: (root)":
            if up is not None:
                illegal.append("duplicate Up")
            up = "(root)"
        elif line.startswith("- Up: `") and line.endswith("`") and line.count("`") == 2:
            if up is not None:
                illegal.append("duplicate Up")
            up = line[len("- Up: `") : -1]
        elif line == "- Down: (none)":
            none_down = True
        elif line.startswith("- Down: `") and line.endswith("`") and line.count("`") == 2:
            downs.append(line[len("- Down: `") : -1])
        else:
            illegal.append(line)
    return {"up": up, "downs": downs, "none": none_down, "illegal": illegal}


def read_agents(root, rel_dir):
    path = os.path.join(root, rel_dir, "AGENTS.md") if rel_dir else os.path.join(root, "AGENTS.md")
    if not os.path.isfile(path) or os.path.islink(path) and not os.path.isfile(path):
        return None
    with open(path, encoding="utf-8", errors="replace") as fh:
        return fh.read()


def agents_rel(rel_dir):
    return "AGENTS.md" if rel_dir == "" else f"{rel_dir}/AGENTS.md"


def expected_fix(from_dir, parent, children):
    if parent is None:
        up = "- Up: (root)"
    else:
        up = f"- Up: `{rel_link(from_dir, agents_rel(parent))}`"
    lines = ["## Chain", up]
    if not children:
        lines.append("- Down: (none)")
    else:
        for child in sorted(children, key=lambda c: rel_link(from_dir, agents_rel(c))):
            lines.append(f"- Down: `{rel_link(from_dir, agents_rel(child))}`")
    return lines


def parent_of(rel, required):
    if rel == "":
        return None
    parts = rel.split("/")
    for i in range(len(parts) - 1, -1, -1):
        anc = "/".join(parts[:i])
        if anc in required:
            return anc
    return ""


def build(files, subs, extra=None):
    file_set = list(files)
    if extra and extra not in file_set:
        file_set.append(extra)
    agents_dirs = set()
    direct_files = {}
    child_dirs = {}
    for rel in file_set:
        if basename(rel) == "AGENTS.md":
            agents_dirs.add(dirname(rel))
        if basename(rel) in SKIP_FILES or basename(rel).startswith("."):
            continue
        if skipped_path(rel) or under_submodule(rel, subs):
            continue
        parent = dirname(rel)
        direct_files[parent] = direct_files.get(parent, 0) + 1
        parts = rel.split("/")
        acc = ""
        for comp in parts[:-1]:
            child_dirs.setdefault(acc, set()).add(comp)
            acc = comp if not acc else f"{acc}/{comp}"
    return agents_dirs, direct_files, child_dirs


def crowd_of(rel, direct_files, child_dirs):
    return direct_files.get(rel, 0) + len(child_dirs.get(rel, ()))


def threshold_from_args(argv):
    if "--threshold" not in argv:
        return CROWD_THRESHOLD
    if os.environ.get("AGENT_KIT_CHAIN_TEST") != "1":
        fail("--threshold is not a bypass")
        return None
    i = argv.index("--threshold")
    if i + 1 >= len(argv):
        fail("--threshold needs a number")
        return None
    try:
        n = int(argv[i + 1])
    except ValueError:
        fail("--threshold needs a number")
        return None
    if n < 1:
        fail("--threshold must be ≥ 1")
        return None
    return n

