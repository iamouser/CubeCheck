"""Extract the latest Write/StrReplace snapshots of Rust sources from Cursor transcripts."""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOTS = [
    Path(r"C:\Users\Антон4к272\.cursor\projects\c-JumpWorld-cheacker\agent-transcripts"),
]
OUT = Path(r"C:\JumpWorld\cheacker\_recovered")
KEEP_PREFIXES = (
    "C:\\JumpWorld\\cheacker\\src\\",
    "C:\\JumpWorld\\cheacker\\crates\\",
    "C:\\JumpWorld\\cheacker\\Cargo.toml",
    "C:\\JumpWorld\\cheacker\\build.rs",
    "C:\\JumpWorld\\cheacker\\src\\",
)
KEEP_NAMES = {
    "Cargo.toml",
    "build.rs",
}

writes: dict[str, tuple[int, str]] = {}
seq = 0


def norm(p: str) -> str:
    return p.replace("/", "\\")


def keep(p: str) -> bool:
    n = norm(p)
    if n.endswith(".rs") or n.endswith("Cargo.toml") or n.endswith("build.rs") or n.endswith(".json"):
        if "\\src\\" in n or "\\crates\\" in n or n.endswith("\\Cargo.toml") or n.endswith("\\build.rs"):
            if n.startswith("C:\\JumpWorld\\cheacker"):
                return True
    return False


def walk(obj, path_hint: str | None = None):
    global seq
    if isinstance(obj, dict):
        name = obj.get("name") or obj.get("toolName")
        inp = obj.get("input") or obj.get("arguments") or {}
        if name in ("Write", "write") and isinstance(inp, dict):
            p = inp.get("path")
            c = inp.get("contents")
            if p and isinstance(c, str) and keep(p):
                seq += 1
                writes[norm(p)] = (seq, c)
        if name in ("StrReplace", "strreplace") and isinstance(inp, dict):
            p = inp.get("path")
            old = inp.get("old_string")
            new = inp.get("new_string")
            if p and isinstance(old, str) and isinstance(new, str) and keep(p):
                cur = writes.get(norm(p))
                if cur and old in cur[1]:
                    seq += 1
                    writes[norm(p)] = (seq, cur[1].replace(old, new, 1))
        for v in obj.values():
            walk(v)
    elif isinstance(obj, list):
        for v in obj:
            walk(v)


def main():
    files = []
    for root in ROOTS:
        if not root.exists():
            continue
        files.extend(root.rglob("*.jsonl"))
    files.sort()
    print(f"transcripts: {len(files)}")
    for f in files:
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for line in text.splitlines():
            if '"Write"' not in line and '"StrReplace"' not in line:
                continue
            try:
                walk(json.loads(line))
            except json.JSONDecodeError:
                continue
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"files recovered: {len(writes)}")
    for p, (s, c) in sorted(writes.items()):
        rel = p.replace("C:\\JumpWorld\\cheacker\\", "")
        dest = OUT / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(c, encoding="utf-8")
        print(f"  {rel}  ({len(c)} chars, seq={s})")


if __name__ == "__main__":
    main()
