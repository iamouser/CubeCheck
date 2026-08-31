"""Extract last complete views.rs / widgets.rs from Cursor transcripts."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(r"C:\Users\Антон4к272\.cursor\projects\c-JumpWorld-cheacker\agent-transcripts")
OUT = Path(r"C:\JumpWorld\cheacker\_extract")
OUT.mkdir(exist_ok=True)


def walk(obj, hits, src):
    if isinstance(obj, dict):
        name = obj.get("name") or obj.get("toolName")
        inp = obj.get("input") or obj.get("arguments") or {}
        if name in ("Write", "write", "StrReplace", "strreplace") and isinstance(inp, dict):
            p = str(inp.get("path") or "")
            n = p.replace("/", "\\")
            if n.endswith("views.rs") or n.endswith("widgets.rs"):
                hits.append((src, name, n, inp))
        for v in obj.values():
            walk(v, hits, src)
    elif isinstance(obj, list):
        for v in obj:
            walk(v, hits, src)


def main():
    hits = []
    for f in sorted(ROOT.rglob("*.jsonl")):
        for i, line in enumerate(f.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            if "views.rs" not in line and "widgets.rs" not in line:
                continue
            if '"Write"' not in line and '"StrReplace"' not in line:
                continue
            try:
                walk(json.loads(line), hits, f"{f.name}:{i}")
            except json.JSONDecodeError:
                pass

    log = []
    log.append(f"hits {len(hits)}")
    for i, (src, name, p, inp) in enumerate(hits):
        kind = "W" if name.lower() == "write" else "S"
        if kind == "W":
            c = inp.get("contents") or ""
            line = (
                f"{i:03d} {kind} {src} {p} chars={len(c)} "
                f"settings={'draw_settings' in c} footer={'draw_footer' in c} "
                f"checkbox={'settings_checkbox_row' in c}"
            )
            log.append(line)
            if p.endswith("views.rs") and "draw_settings" in c:
                dest = OUT / f"views_with_settings_{i}.rs"
                dest.write_text(c, encoding="utf-8")
                log.append(f"  SAVED {dest.name}")
            if p.endswith("widgets.rs") and "settings_checkbox_row" in c:
                dest = OUT / f"widgets_with_settings_{i}.rs"
                dest.write_text(c, encoding="utf-8")
                log.append(f"  SAVED {dest.name}")
        else:
            old = inp.get("old_string") or ""
            new = inp.get("new_string") or ""
            line = (
                f"{i:03d} {kind} {src} old={len(old)} new={len(new)} "
                f"settings={'draw_settings' in new} checkbox={'settings_checkbox_row' in new}"
            )
            log.append(line)
            if "draw_settings" in new and len(new) > 400:
                dest = OUT / f"settings_new_{i}.rs"
                dest.write_text(new, encoding="utf-8")
                log.append(f"  SAVED {dest.name}")
            if "settings_checkbox_row" in new and len(new) > 400:
                dest = OUT / f"widgets_new_{i}.rs"
                dest.write_text(new, encoding="utf-8")
                log.append(f"  SAVED {dest.name}")

    (OUT / "log.txt").write_text("\n".join(log), encoding="utf-8")
    print("\n".join(log[-80:]))
    print("wrote", OUT / "log.txt")


if __name__ == "__main__":
    main()
