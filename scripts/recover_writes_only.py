"""Keep only the last full Write() snapshot per path (ignore StrReplace)."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(r"C:\Users\Антон4к272\.cursor\projects\c-JumpWorld-cheacker\agent-transcripts")
OUT = Path(r"C:\JumpWorld\cheacker\_recovered_writes")
writes: dict[str, tuple[int, str]] = {}
seq = 0


def keep(p: str) -> bool:
    n = p.replace("/", "\\")
    if not n.startswith("C:\\JumpWorld\\cheacker"):
        return False
    return n.endswith(".rs") or n.endswith("Cargo.toml") or n.endswith(".json")


def walk(obj):
    global seq
    if isinstance(obj, dict):
        name = obj.get("name") or obj.get("toolName")
        inp = obj.get("input") or obj.get("arguments") or {}
        if name in ("Write", "write") and isinstance(inp, dict):
            p = inp.get("path")
            c = inp.get("contents")
            if p and isinstance(c, str) and keep(p):
                seq += 1
                writes[p.replace("/", "\\")] = (seq, c)
        for v in obj.values():
            walk(v)
    elif isinstance(obj, list):
        for v in obj:
            walk(v)


def main():
    for f in sorted(ROOT.rglob("*.jsonl")):
        for line in f.read_text(encoding="utf-8", errors="replace").splitlines():
            if '"Write"' not in line:
                continue
            try:
                walk(json.loads(line))
            except json.JSONDecodeError:
                continue
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"writes: {len(writes)}")
    for p, (s, c) in sorted(writes.items()):
        rel = p.replace("C:\\JumpWorld\\cheacker\\", "")
        dest = OUT / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(c, encoding="utf-8")
        print(f"  {rel} ({len(c)} seq={s})")


if __name__ == "__main__":
    main()
