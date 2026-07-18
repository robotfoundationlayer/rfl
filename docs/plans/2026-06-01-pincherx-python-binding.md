# `bindings/python/` — minimal RFL retarget binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **LOCAL-ONLY plan file** — never `git add` this doc (it lives under `docs/plans/`, which is not committed). The design doc `docs/design/2026-05-31-pincherx-existence-proof-design.md` is the committed authority.

**Goal:** Add a minimal PyO3 binding to the `rfl` repo exposing one function, `rfl.retarget(skill_yaml: str, descriptor_yaml: str) -> str` (canonical-action JSONL), so the separate `milchick` driver can call the RFL retarget engine in-process.

**Architecture:** A new, workspace-*excluded* cdylib crate at `bindings/python/`, path-dependent on `crates/rfl-core`, built with `maturin`. Its single function transcribes the exact 4-call pipeline of `rfl-conformance::retarget_example_to_jsonl` (and `rfl-cli retarget`), but string-in / string-out instead of path-in. Being string-based and format-agnostic it does not churn as primitive coverage or the canonical JSON format evolve. Errors become a Python exception `rfl.RetargetError`.

**Tech Stack:** Rust (edition 2024, rustc 1.96), PyO3 0.28 (`abi3-py39` + `extension-module`), maturin ≥1.0, uv-managed venv, pytest. The existing crates/spec/conformance/examples/schemas are **not touched** (in-process incrementing is complete); the only change inside the existing tree is one line added to the root `Cargo.toml` workspace table (`exclude`).

---

## Source of truth (transcribed 2026-06-01 from HEAD `274f81f`, sync `0 0`)

The binding must call exactly these rfl-core APIs. Do **not** write them from memory — they are transcribed here verbatim:

- `crates/rfl-core/src/skill_isa.rs:507` — `pub fn parse_yaml(text: &str) -> crate::Result<Self>` on `pub struct Skill` (`skill_isa.rs:49`), which has field `pub skill: String` (`skill_isa.rs:51`). Access the name as `skill.skill`.
- `crates/rfl-core/src/embodiment.rs:135` — `pub fn parse_yaml(text: &str) -> crate::Result<Self>` on `pub struct Embodiment` (`embodiment.rs:23`), which has field `pub id: String` (`embodiment.rs:25`). Access as `emb.id`.
- `crates/rfl-core/src/translation.rs:82` — `pub fn retarget(skill: &Skill, embodiment: &Embodiment) -> crate::Result<RetargetOutput>`. `RetargetOutput` (`translation.rs:30`) has `pub actions: Vec<CanonicalAction>` (`:32`) and `pub suffixes: Vec<&'static str>` (`:34`).
- `crates/rfl-core/src/canonical.rs:259` — `pub fn to_jsonl(skill: &str, embodiment_id: &str, actions: &[CanonicalAction], suffixes: &[&str]) -> String`.
- `crate::Result<T>` = `Result<T, rfl_core::Error>` (`crates/rfl-core/src/lib.rs:62`); `rfl_core::Error` (`lib.rs:39`) implements `std::error::Error` + `Display`.

Reference implementations that already chain these four calls (copy the call shape, not the I/O):
- `crates/rfl-conformance/src/lib.rs:26-35` — `retarget_example_to_jsonl(skill_path, embodiment_path)` (path-in).
- `crates/rfl-cli/src/main.rs:55-64` — the `Retarget { skill, embodiment }` arm; prints `to_jsonl(...)` via `print!("{jsonl}")` (no extra trailing newline). **This is the test oracle.**

Test fixture pair (real, committed, parses cleanly):
- skill: `examples/01-cable-insertion/skill.yaml` → `skill: cable-insertion`
- embodiment: `examples/01-cable-insertion/embodiments/allegro.yaml` → `embodiment.id: wonik-allegro-v4`
- ⇒ `to_jsonl` ids are `cable-insertion/wonik-allegro-v4/0001-<suffix>`, one per line.

Environment facts: maturin **not yet installed**; `uv` 0.8.8 present; rustc 1.96.0. Root `.gitignore` already covers `target/`, `*.so`, `.venv/`, `__pycache__/`, `.pytest_cache/`, `dist/`, `build/` — so build artifacts and the venv are auto-ignored; **no local `.gitignore` needed**. The repo intentionally commits `Cargo.lock`, so the binding's own `bindings/python/Cargo.lock` is committed too.

README marker to update (final task): `README.md:69`.

---

## File structure

- Create `bindings/python/Cargo.toml` — the cdylib crate manifest (pyo3 + path-dep on rfl-core).
- Create `bindings/python/pyproject.toml` — maturin build backend + project metadata.
- Create `bindings/python/src/lib.rs` — the one `#[pyfunction] retarget` + `RetargetError` + `#[pymodule] rfl`.
- Create `bindings/python/tests/test_binding.py` — pytest: import/smoke, CLI-oracle equivalence, error.
- Modify `Cargo.toml` (root) — add `exclude = ["bindings/python"]` to `[workspace]`.
- Modify `README.md:69` — bindings layout marker `⏳ → 🚧`.
- Generated (committed): `bindings/python/Cargo.lock`. Generated (ignored): `bindings/python/target/`, `bindings/python/.venv/`.

---

## Task 0: Tooling prerequisites (environment, not TDD)

**Files:** none (installs maturin + creates the test venv).

- [ ] **Step 1: Create the test venv and install build tooling**

Run (single shell invocation — venv activation does not persist across calls):
```bash
cd ~/Documents/GitHub/rfl/bindings/python 2>/dev/null || (mkdir -p ~/Documents/GitHub/rfl/bindings/python && cd ~/Documents/GitHub/rfl/bindings/python)
cd ~/Documents/GitHub/rfl/bindings/python && uv venv && source .venv/bin/activate && uv pip install "maturin>=1.0,<2.0" pytest && maturin --version && python --version
```
Expected: prints e.g. `maturin 1.x.y` and `Python 3.1x.z`. (With abi3 the exact Python version does not matter.)

> Note: every later build/test step re-runs `source .venv/bin/activate` in its own command because the harness shell state does not persist between tool calls.

---

## Task 1: Scaffold the crate, implement `retarget`, and prove it imports + runs

**Files:**
- Modify: `Cargo.toml` (root) — `[workspace] exclude`
- Create: `bindings/python/Cargo.toml`
- Create: `bindings/python/pyproject.toml`
- Create: `bindings/python/src/lib.rs`
- Test: `bindings/python/tests/test_binding.py`

- [ ] **Step 1: Exclude the binding from the workspace (required before maturin runs cargo)**

In root `Cargo.toml`, the `[workspace]` table currently reads:
```toml
[workspace]
resolver = "2"
members = [
    "crates/rfl-core",
    "crates/rfl-cli",
    "crates/rfl-conformance",
]
```
Change it to add `exclude` (members unchanged):
```toml
[workspace]
resolver = "2"
members = [
    "crates/rfl-core",
    "crates/rfl-cli",
    "crates/rfl-conformance",
]
# bindings/python is a PyO3 cdylib built only via maturin; excluding it keeps
# workspace-wide `cargo test`/`cargo build` (and the green conformance suite)
# byte-identical and lets maturin invoke cargo inside it without a member error.
exclude = ["bindings/python"]
```

- [ ] **Step 2: Create `bindings/python/Cargo.toml`**

```toml
[package]
name = "rfl"
description = "Robot Foundation Layer — minimal Python binding (retarget engine)"
version = "0.0.1"
edition = "2024"
rust-version = "1.85"
license = "Apache-2.0"
repository = "https://github.com/robotfoundationlayer/rfl"
homepage = "https://github.com/robotfoundationlayer"
authors = ["Yoichiro Hara <yo@vox.delivery>"]

[lib]
# Python import name. Must match the `#[pymodule]` name in src/lib.rs.
name = "rfl"
crate-type = ["cdylib"]

[dependencies]
# abi3-py39: one extension that imports on any CPython >= 3.9 (covers ROS 2
# Humble=3.10 / Jazzy=3.12). extension-module is added by maturin at build time
# (see pyproject [tool.maturin].features) so it is not a default feature here.
pyo3 = { version = "0.28", features = ["abi3-py39"] }
rfl-core = { path = "../../crates/rfl-core" }
```

- [ ] **Step 3: Create `bindings/python/pyproject.toml`**

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "rfl"
dynamic = ["version"]
description = "Robot Foundation Layer — minimal Python binding (retarget engine)"
requires-python = ">=3.9"
license = { text = "Apache-2.0" }

[tool.maturin]
# Adds the pyo3 extension-module feature only for maturin builds, so the produced
# cdylib does not link libpython (symbols resolve at import time).
features = ["pyo3/extension-module"]
```

- [ ] **Step 4: Create `bindings/python/src/lib.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Minimal Python binding for the RFL retarget engine.
//!
//! Exposes one function, [`retarget`], wrapping the stable rfl-core pipeline
//! (`Skill::parse_yaml` -> `Embodiment::parse_yaml` -> `translation::retarget` ->
//! `canonical::to_jsonl`) — identical to `rfl-conformance::retarget_example_to_jsonl`
//! and `rfl-cli`'s `retarget` subcommand, but string-in / string-out. Because it
//! exchanges only strings it does not churn as primitive coverage or the canonical
//! JSON format evolve.

use pyo3::prelude::*;

use rfl_core::canonical;
use rfl_core::embodiment::Embodiment;
use rfl_core::skill_isa::Skill;
use rfl_core::translation;

pyo3::create_exception!(rfl, RetargetError, pyo3::exceptions::PyException);

/// Map an rfl-core error to the Python `rfl.RetargetError`.
fn to_py_err(e: rfl_core::Error) -> PyErr {
    RetargetError::new_err(e.to_string())
}

/// Retarget a Skill ISA composition onto an embodiment descriptor.
///
/// Both inputs are YAML text; the return value is the canonical-action JSONL
/// stream (one `execute` message per line). Parse or retarget failures raise
/// `rfl.RetargetError`.
#[pyfunction]
fn retarget(skill_yaml: &str, descriptor_yaml: &str) -> PyResult<String> {
    let skill = Skill::parse_yaml(skill_yaml).map_err(to_py_err)?;
    let emb = Embodiment::parse_yaml(descriptor_yaml).map_err(to_py_err)?;
    let out = translation::retarget(&skill, &emb).map_err(to_py_err)?;
    Ok(canonical::to_jsonl(
        &skill.skill,
        &emb.id,
        &out.actions,
        &out.suffixes,
    ))
}

/// The `rfl` Python module.
#[pymodule]
fn rfl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(retarget, m)?)?;
    m.add("RetargetError", m.py().get_type::<RetargetError>())?;
    Ok(())
}
```

- [ ] **Step 5: Write the failing smoke test `bindings/python/tests/test_binding.py`**

```python
"""Tests for the minimal rfl Python binding (rfl.retarget)."""

import json
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
        assert "cable-insertion/wonik-allegro-v4/" in json.dumps(msg)
```

- [ ] **Step 6: Run the test to verify it fails for the right reason**

```bash
cd ~/Documents/GitHub/rfl/bindings/python && source .venv/bin/activate && python -m pytest -v
```
Expected: FAIL — `ModuleNotFoundError: No module named 'rfl'` (the extension is not built yet).

- [ ] **Step 7: Build and install the extension into the venv**

```bash
cd ~/Documents/GitHub/rfl/bindings/python && source .venv/bin/activate && maturin develop
```
Expected: `🔗 Found pyo3 bindings with abi3 support` … `📦 Installed rfl-0.0.1`. If it errors with a workspace-member message, Step 1 was skipped — add the `exclude` and rerun.

- [ ] **Step 8: Run the test to verify it passes**

```bash
cd ~/Documents/GitHub/rfl/bindings/python && source .venv/bin/activate && python -m pytest -v
```
Expected: PASS (2 passed).

- [ ] **Step 9 (separate verification batch): confirm the existing suite is unaffected**

```bash
cd ~/Documents/GitHub/rfl && cargo test 2>&1 | tail -20 && echo "---SCHEMA C1-C7---" && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -5
```
Expected: existing crates' tests all green (unchanged count) and `validate.py` C1–C7 PASS. **Visually confirm green before staging anything.**

- [ ] **Step 10: Commit (explicit paths only; no `-A`)**

```bash
cd ~/Documents/GitHub/rfl && git add Cargo.toml bindings/python/Cargo.toml bindings/python/pyproject.toml bindings/python/src/lib.rs bindings/python/tests/test_binding.py bindings/python/Cargo.lock && git status --short
```
Confirm the staged list contains only those files (no `target/`, no `.venv/`, no `.so`), then:
```bash
cd ~/Documents/GitHub/rfl && git commit -m "$(cat <<'EOF'
feat(bindings): add minimal PyO3 Python binding (rfl.retarget)

Expose one string-in/out function wrapping the stable rfl-core pipeline
(Skill::parse_yaml -> Embodiment::parse_yaml -> translation::retarget ->
canonical::to_jsonl), identical to rfl-conformance's retarget_example_to_jsonl
and rfl-cli's retarget subcommand. The separate `milchick` PincherX driver
imports it as its retarget bridge. Built with maturin; abi3-py39 for ROS 2
Python portability. The crate is excluded from the workspace so the existing
conformance suite is unaffected.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Equivalence to the canonical path + error handling

**Files:**
- Test: `bindings/python/tests/test_binding.py` (append two tests)

- [ ] **Step 1: Append the CLI-oracle equivalence test and the error test**

Append to `bindings/python/tests/test_binding.py`:
```python
import shutil
import subprocess


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
```

- [ ] **Step 2: Rebuild (no lib.rs change, but ensures latest) and run all tests**

```bash
cd ~/Documents/GitHub/rfl/bindings/python && source .venv/bin/activate && maturin develop && python -m pytest -v
```
Expected: PASS (4 passed). The oracle test builds `rfl-cli` once (fast; already built) then compares stdout.

If `test_invalid_yaml_raises_retarget_error` instead raises a different exception type, the input chosen does not reach `parse_yaml`'s error path — replace the bad YAML with a value that parses as YAML but fails Skill validation, e.g. `rfl.retarget("skill: 123\n", EMBODIMENT.read_text())`, and rerun. The contract is only that *some* `rfl.RetargetError` is raised on bad input.

- [ ] **Step 3: Commit**

```bash
cd ~/Documents/GitHub/rfl && git add bindings/python/tests/test_binding.py && git commit -m "$(cat <<'EOF'
test(bindings): assert byte-equivalence to rfl-cli retarget + error path

Prove rfl.retarget delegates faithfully by comparing against the rfl-cli
retarget oracle (format-evolution-proof: both sides share the pipeline) and
that bad input raises rfl.RetargetError.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Update the README layout marker + memory

**Files:**
- Modify: `README.md:69`

- [ ] **Step 1: Update the bindings layout marker**

In `README.md`, replace the line (currently `:69`):
```
└── bindings/             # ⏳ Python (PyO3) + C (cbindgen) bindings (planned for the v1.0 stabilization milestone, 2027 Q4)
```
with:
```
└── bindings/             # 🚧 Python (PyO3): minimal retarget binding present (`rfl.retarget`, string-in/out) · C (cbindgen) + full typed Python binding ⏳ (v1.0, 2027 Q4)
```
(Both the `🚧` "scaffold/minimal present" and `⏳` "planned" legend markers are used truthfully; the full typed + C bindings remain v1.0 deferrals per design doc §10.)

- [ ] **Step 2: Commit**

```bash
cd ~/Documents/GitHub/rfl && git add README.md && git commit -m "$(cat <<'EOF'
docs(readme): bindings layout marker ⏳ -> 🚧 (minimal Python binding present)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 3: Update memory with the real commit hashes**

```bash
cd ~/Documents/GitHub/rfl && git log --oneline -3
```
Capture the actual short hashes, then update `~/.claude/projects/-Users-yoichirohara-Library-Mobile-Documents-iCloud-md-obsidian-Documents-hacci/memory/project_rfl.md` (and its `MEMORY.md` index line): note that `bindings/python/` now provides `rfl.retarget(skill_yaml, descriptor_yaml) -> jsonl` (PyO3 0.28, abi3-py39, maturin; workspace-excluded; verified against the rfl-cli oracle), with the real HEAD hash. Do **not** invent hashes — read them from `git log`.

---

## Pre-push verification (gated on user go-ahead — push is outward-facing)

```bash
cd ~/Documents/GitHub/rfl && git merge-base --is-ancestor origin/main HEAD && echo "FF-OK (origin/main is an ancestor of HEAD)"
```
Expected: `FF-OK`. Then push (only after the user confirms), and verify sync:
```bash
cd ~/Documents/GitHub/rfl && git push origin main && git fetch origin && git rev-list --left-right --count origin/main...HEAD
```
Expected after push: `0	0`.

---

## Out of scope for THIS plan (later phases — design doc §4 / §10)

- The `milchick` repo (descriptor / skills / `rfl_interbotix_driver.py` / `measure.py`) — starts only after this binding imports cleanly.
- Gazebo / physical-arm runs — after milchick is populated and the user's Interbotix bringup is done.
- The C (cbindgen) binding and the full typed Python binding — v1.0 deferrals.
- Making `milchick` public + Apache 2.0 + the rfl README "reference driver" link — only once the existence proof runs (milchick is Private now).

---

## Self-review

- **Spec coverage (design doc §4 / §11):** ✅ one `rfl.retarget(skill_yaml, descriptor_yaml) -> str` (Task 1 Step 4); ✅ wraps the exact conformance/CLI pipeline (Source-of-truth section + Task 1 code); ✅ PyO3 cdylib, path-dep on rfl-core, maturin build (Task 1 Steps 2–3); ✅ errors are a Python exception `rfl.RetargetError` (Task 1 Step 4, Task 2 error test); ✅ module imports in a Python env — verified by the import test and made portable across ROS 2 Pythons via abi3 (Task 1 Steps 6–8); ✅ string-in/out / format-independence asserted by the CLI-oracle test (Task 2). §11's "confirm PyO3 + maturin setup and that the module imports in the driver's Python env" is the explicit Task 0 + Task 1 Steps 6–8.
- **Placeholder scan:** none — every code/command step shows full content and an expected result.
- **Type consistency:** `Skill.skill`, `Embodiment.id`, `RetargetOutput.actions`/`.suffixes`, and `to_jsonl(&str, &str, &[CanonicalAction], &[&str])` all match the transcribed signatures; `to_py_err` is defined once and used three times; the `#[pymodule] fn rfl` name matches `[lib] name = "rfl"` and the `import rfl` in the tests.
- **Discipline:** Conventional Commits + `Co-Authored-By`; explicit-path `git add` (no `-A`); verification batch (Step 9 / pre-push) separated from staging; no secrets staged; the only edit inside the existing tree is the root `Cargo.toml` `exclude` line (crates/spec/conformance/examples/schemas untouched).
