#!/usr/bin/env python3
"""Copy the Principles of Flight data out of the old web app (read-only).

Usage:  python3 scripts/import-from-web.py [path/to/pilot-prep-hub]

Writes
  data/pof.json            questions + explanations + look-alike highlights
  data/seed-progress.json  the current PoF progress from src/data/current.json
                           (only used on first start, when the phone has no
                           saved state yet)
"""
import json
import sys
from pathlib import Path

SUBJECT = "Principles of Flight"
root = Path(__file__).resolve().parent.parent
src = Path(sys.argv[1] if len(sys.argv) > 1 else root.parent / "pilot-prep-hub") / "src" / "data"

questions = [q for q in json.loads((src / "questions.json").read_text()) if q["subject"] == SUBJECT]
explanations = json.loads((src / "pof-explanations.json").read_text())
similar = json.loads((src / "pof-similar.json").read_text())


def js_slice(text: str, start: int, end: int) -> str:
    """Slice like JavaScript (UTF-16 code units)."""
    b = text.encode("utf-16-le")
    return b[2 * start:2 * end].decode("utf-16-le")


def segments(text: str, diff):
    """Question text split into plain / highlighted parts."""
    if not diff:
        return None
    n = len(text.encode("utf-16-le")) // 2
    out, pos = [], 0
    for start, end in diff:
        if start > pos:
            out.append({"t": js_slice(text, pos, start), "m": False})
        out.append({"t": js_slice(text, start, end), "m": True})
        pos = end
    if pos < n:
        out.append({"t": js_slice(text, pos, n), "m": False})
    return out


out = []
for q in questions:
    item = {
        "id": q["id"],
        "caa_id": q["caa_id"],
        "question": q["question"],
        # answers[0] is always the correct one
        "answers": [q["answer1"], q["answer2"], q["answer3"], q["answer4"]],
    }
    ex = explanations.get(str(q["id"]))
    if ex:
        item["explanation"] = {k: ex[k] for k in ("why", "tip", "twin") if ex.get(k)}
    sim = similar.get(str(q["id"]))
    if sim:
        item["similar"] = {
            "twins": sim["twins"],
            "same_text": sim["sameText"],
            "segments": segments(q["question"], sim["diff"]),
        }
    out.append(item)

(root / "data" / "pof.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))

current = json.loads((src / "current.json").read_text())
seed = {
    "subjects": {SUBJECT: current.get("subjects", {}).get(SUBJECT, {"answered": {}, "favorites": [], "currentIndex": 0})},
    "theme": current.get("theme", "dark"),
}
(root / "data" / "seed-progress.json").write_text(json.dumps(seed, ensure_ascii=False, indent=1))

sp = seed["subjects"][SUBJECT]
print(f"{len(out)} Fragen, {sum('explanation' in q for q in out)} Erklärungen, "
      f"{sum('similar' in q for q in out)} ähnliche")
print(f"Fortschritt: {len(sp.get('answered', {}))} beantwortet, {len(sp.get('favorites', []))} markiert, "
      f"{len(sp.get('ultraHard', []))} ultra")
