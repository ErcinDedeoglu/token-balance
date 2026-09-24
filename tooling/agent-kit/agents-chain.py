#!/usr/bin/env python3
"""AGENTS.md chain gate. Fail closed. No network. No opt-out."""

import pathlib
import sys

here = pathlib.Path(__file__).resolve().parent
ns = {"__name__": "agents_chain", "__file__": str(here / "chain_paths.py")}
for name in ("chain_paths.py", "chain_links.py", "chain_main.py"):
    path = here / name
    exec(compile(path.read_text(encoding="utf-8"), str(path), "exec"), ns)
sys.exit(ns["main"](sys.argv[1:]))
