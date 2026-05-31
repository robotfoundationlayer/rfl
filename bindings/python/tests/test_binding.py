"""Tests for the minimal rfl Python binding (rfl.retarget)."""

import json
import shutil
import subprocess
from pathlib import Path

import pytest

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


@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo not on PATH")
def test_retarget_matches_cli_oracle():
    """The binding must be byte-identical to `rfl-cli retarget` (same 4-call path)."""
    import rfl

    cli = subprocess.run(
        [
            "cargo", "run", "-q", "-p", "rfl-cli", "--",
            "retarget", str(SKILL), "--embodiment", str(EMBODIMENT),
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    binding_out = rfl.retarget(SKILL.read_text(), EMBODIMENT.read_text())
    assert binding_out == cli.stdout


def test_invalid_yaml_raises_retarget_error():
    import rfl

    with pytest.raises(rfl.RetargetError):
        rfl.retarget("not: [a, valid", EMBODIMENT.read_text())
