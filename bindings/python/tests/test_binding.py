"""Tests for the minimal rfl Python binding (rfl.retarget)."""

import json
from pathlib import Path

# bindings/python/tests/test_binding.py -> parents[3] == repo root (rfl/)
REPO_ROOT = Path(__file__).resolve().parents[3]
SKILL = REPO_ROOT / "examples" / "01-cable-insertion" / "skill.yaml"
EMBODIMENT = REPO_ROOT / "examples" / "01-cable-insertion" / "embodiments" / "allegro.yaml"


def test_module_imports_and_exposes_surface():
    import rfl

    assert hasattr(rfl, "retarget")
    assert hasattr(rfl, "RetargetError")


def test_retarget_smoke_produces_jsonl():
    import rfl

    out = rfl.retarget(SKILL.read_text(), EMBODIMENT.read_text())
    lines = out.splitlines()
    assert lines, "expected at least one canonical-action line"
    for line in lines:
        msg = json.loads(line)  # each line is valid JSON
        # to_jsonl id contract: "{skill}/{embodiment_id}/{NNNN}-{suffix}"
        assert msg["action_id"].startswith("cable-insertion/wonik-allegro-v4/")
