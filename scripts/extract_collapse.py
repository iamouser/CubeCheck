"""Extract the last collapsing sidebar implementation from transcripts."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(r"C:\Users\Антон4к272\.cursor\projects\c-JumpWorld-cheacker\agent-transcripts")
OUT = Path(r"C:\JumpWorld\cheacker\_extract\collapse.rs")


def walk(obj, hits):
    if isinstance(obj, dict):
        name = obj.get("name") or obj.get("toolName")
        inp = obj.get("input") or obj.get("arguments") or {}
        if name in ("StrReplace", "Write", "strreplace", "write") and isinstance(inp, dict):
            p = str(inp.get("path") or "")
            if p.replace("/", "\\").endswith("sidebar.rs"):
                new = inp.get("new_string") or inp.get("contents") or ""
                if "CollapsingState" in new or "paint_default_icon" in new:
                    hits.append(new)
        for v in obj.values():
            walk(v, hits)
    elif isinstance(obj, list):
        for v in obj:
            walk(v, hits)


def main():
    hits = []
    for f in ROOT.rglob("*.jsonl"):
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            if "CollapsingState" not in line and "paint_default_icon" not in line:
                continue
            try:
                walk(json.loads(line), hits)
            except json.JSONDecodeError:
                pass
    print("hits", len(hits))
    if hits:
        OUT.write_text(hits[-1], encoding="utf-8")
        print("saved last", len(hits[-1]), "chars")


if __name__ == "__main__":
    main()
