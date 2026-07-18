# Spiral Sweep-Pattern Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the normative `spiral` sweep-set Σ generator (`spec/02` Appendix A) so `reach.scan` with `pattern: spiral` over a surface region emits an Archimedean-spiral sweep set instead of a raster.

**Architecture:** A pure `sigma::spiral` generator (arc-length-stepped Archimedean spiral from the surface centroid, bracketed bisection for the arc-length inversion, fixed surface station orientation reused), wired into `translation::lower_reach_scan`'s Surface arm by a `pattern` dispatch, and demonstrated by a sibling `skill-spiral.yaml` in example 02 with per-hand conformance goldens.

**Tech Stack:** Rust (rfl-core / rfl-conformance), nalgebra (Pose6D), insta (goldens), boon (JSON-Schema), uv + jsonschema (Class-1 schema check).

**Process discipline (this repo):**
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"` (shell state does not persist between Bash calls).
- Run validation (`cargo test` / `validate.py`) and read the result in a batch **physically separate** from the `git commit` — stage only after eyeballing green.
- `git add` explicit file paths only (never `-A`); never stage `docs/plans/`. The parallel session shares this working tree — `git status` immediately before each stage.
- Per commit: branch == `main`; before push `git merge-base --is-ancestor origin/main HEAD`; after push `git rev-list --left-right --count origin/main...HEAD` == `0 0`.
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` trailer. Use the real `git log` hash in notes, never a predicted one.

**Analytic expectations (derived from Appendix A, used as test pins):** for the 200×150 mm surface, standoff 100 mm, overlap 0.2, bounding radius `R = √(0.10²+0.075²) = 0.125 m`:
- allegro (FOV 60×45) → `s = min(s_u,s_v) = 0.0662742 m`, stations `i=0..11` → **12 poses** (raster was 9).
- leap (FOV 70×55) → `s = 0.0832915 m` → **8 poses** (raster was 4).
- pneumatic-6f (FOV 65×50) → `s = 0.0746092 m` → **10 poses** (raster was 6).
- Smaller FOV → smaller footprint → smaller `s` → more stations; allegro(12) > pneumatic(10) > leap(8). A unit-test count mismatch means **recheck the arc-length math**, not a blind update.

---

## Task 1: `sigma::spiral` generator + unit tests

**Files:**
- Modify: `crates/rfl-core/src/sigma.rs` (add `spiral`, `spiral_arc_length`, `solve_theta_for_arc_length`, and four unit tests)

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests` block in `crates/rfl-core/src/sigma.rs` (it already has `use super::*;` and `use crate::canonical::round6;`):

```rust
    #[test]
    fn spiral_first_station_is_the_centroid() {
        // i=0 sits at theta=0, r=0 => the surface centroid (U/2, V/2), z=standoff.
        let poses = spiral(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert_eq!(round6(poses[0].position[0]), 0.10);
        assert_eq!(round6(poses[0].position[1]), 0.075);
        assert_eq!(round6(poses[0].position[2]), 0.10);
    }

    #[test]
    fn allegro_spiral_has_twelve_poses() {
        // 60x45 deg FOV, standoff 0.10, overlap 0.2 -> s=0.0662742; stations to the
        // half-diagonal R=0.125 give i=0..11. Analytic value; a mismatch means
        // recheck the arc-length math, not a blind update.
        let poses = spiral(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert_eq!(poses.len(), 12);
    }

    #[test]
    fn spiral_radius_is_non_decreasing() {
        let poses = spiral(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        let (cu, cv) = (0.10_f64, 0.075_f64);
        let mut prev = -1.0_f64;
        for p in &poses {
            let r = ((p.position[0] - cu).powi(2) + (p.position[1] - cv).powi(2)).sqrt();
            assert!(r >= prev - 1e-9, "radius decreased: {r} < {prev}");
            prev = r;
        }
    }

    #[test]
    fn spiral_count_varies_per_fov() {
        let allegro = spiral(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        let leap = spiral(0.20, 0.15, 0.10, 0.2, 70_f64.to_radians(), 55_f64.to_radians());
        let pneu = spiral(0.20, 0.15, 0.10, 0.2, 65_f64.to_radians(), 50_f64.to_radians());
        // Smaller FOV -> smaller footprint -> smaller spacing -> more stations.
        assert_eq!((allegro.len(), leap.len(), pneu.len()), (12, 8, 10));
        assert_ne!(allegro.len(), 9); // differs from the raster count
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core spiral`
Expected: FAIL — `cannot find function spiral in this scope` (compile error).

- [ ] **Step 3: Implement the generator**

In `crates/rfl-core/src/sigma.rs`, after the `waypoints` function (before `#[cfg(test)]`), add:

```rust
/// Archimedean-spiral arc length from the centre to polar angle `theta`, for the
/// spiral `r = a·θ`: `L(θ) = (a/2)·[θ·√(1+θ²) + asinh(θ)]` (from the polar
/// arc-length element `ds = a·√(θ²+1) dθ`).
fn spiral_arc_length(theta: f64, a: f64) -> f64 {
    0.5 * a * (theta * (1.0 + theta * theta).sqrt() + theta.asinh())
}

/// Solve `L(θ) = target` for the unique `θ ≥ theta_lo` by bisection. `L` is
/// strictly increasing; for a one-arc-length-step advance (`target − L(theta_lo) =
/// s = 2π·a`) the bracket `[theta_lo, theta_lo + 2π]` always contains the root
/// because `dL/dθ = a·√(θ²+1) ≥ a`. Bisection is chosen over Newton for
/// byte-determinism: pure comparison + midpoint, platform-invariant control flow.
fn solve_theta_for_arc_length(target: f64, a: f64, theta_lo: f64) -> f64 {
    let mut lo = theta_lo;
    let mut hi = theta_lo + 2.0 * std::f64::consts::PI;
    // 60 halvings drive the bracket below 2π/2^60 ≈ 5e-18, far under round6 (1e-6).
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if spiral_arc_length(mid, a) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Generate the spiral sweep set Σ for a planar surface (`spec/02` Appendix A,
/// spiral). An Archimedean spiral outward from the surface centroid at constant
/// arc-length step Δℓ = s = min(s_u, s_v), CCW, bounded by the region half-diagonal
/// (the v0 reading of "bounding radius"). Same units / FOV inputs as `raster`; the
/// fixed surface `station_orientation` is reused (general orientation is deferred).
#[must_use]
pub fn spiral(
    size_u: f64,
    size_v: f64,
    standoff: f64,
    coverage_overlap: f64,
    h_angle_rad: f64,
    v_angle_rad: f64,
) -> Vec<Pose6D> {
    let f_u = 2.0 * standoff * (h_angle_rad / 2.0).tan();
    let f_v = 2.0 * standoff * (v_angle_rad / 2.0).tan();
    let s_u = f_u * (1.0 - coverage_overlap);
    let s_v = f_v * (1.0 - coverage_overlap);
    let s = s_u.min(s_v);
    let a = s / (2.0 * std::f64::consts::PI);
    let (cu, cv) = (size_u / 2.0, size_v / 2.0);
    let r_bound = (cu * cu + cv * cv).sqrt();
    let orientation = station_orientation();

    let mut poses = Vec::new();
    // Degenerate guard: zero spacing (a flat FOV) would never advance — emit only
    // the centroid station.
    if s <= 0.0 {
        poses.push(Pose6D { position: [cu, cv, standoff], orientation });
        return poses;
    }
    let mut i = 0usize;
    let mut theta_prev = 0.0_f64;
    loop {
        let theta = if i == 0 {
            0.0
        } else {
            solve_theta_for_arc_length(i as f64 * s, a, theta_prev)
        };
        let r = a * theta;
        if r > r_bound {
            break;
        }
        poses.push(Pose6D {
            position: [cu + r * theta.cos(), cv + r * theta.sin(), standoff],
            orientation,
        });
        theta_prev = theta;
        i += 1;
    }
    poses
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core spiral`
Expected: PASS — `spiral_first_station_is_the_centroid`, `allegro_spiral_has_twelve_poses`, `spiral_radius_is_non_decreasing`, `spiral_count_varies_per_fov` all green. If `allegro_spiral_has_twelve_poses` (or `spiral_count_varies_per_fov`) shows a different count, STOP and recheck the arc-length math against the analytic expectation — do not edit the assertion to match.

- [ ] **Step 5: Run the full rfl-core suite (warning-clean check), in a batch separate from the commit**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -20`
Expected: all rfl-core tests pass; no `warning:` lines from rustc default lints (dead_code etc.). Read and confirm before staging.

- [ ] **Step 6: Commit**

Verify state, then stage only `sigma.rs`:

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-core/src/sigma.rs
git commit -F - <<'EOF'
feat(core): add spiral sweep-set generator (Appendix A)

Realize the normative spiral Σ generator (spec/02 Appendix A): an Archimedean
spiral from the surface centroid at constant arc-length step s=min(s_u,s_v),
CCW, bounded by the region half-diagonal. The arc-length inversion L(θ)=i·s
uses bracketed bisection ([θ_{i-1}, θ_{i-1}+2π], dL/dθ≥a) for byte-deterministic
control flow; the fixed surface station orientation is reused. Not yet wired
into lowering (next commit).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: push succeeds; final count `0 0`. Record the real hash from `git log --oneline -1`.

---

## Task 2: wire `pattern: spiral` into `lower_reach_scan`

**Files:**
- Modify: `crates/rfl-core/src/translation.rs` (the Surface arm of `lower_reach_scan` ~line 537; add one translation test in the `mod tests` block)

- [ ] **Step 1: Write the failing translation test**

Append to the `mod tests` block in `crates/rfl-core/src/translation.rs` (it already imports `Skill`, `Embodiment`, `Path`, and `retarget`):

```rust
    #[test]
    fn scan_pattern_spiral_lowers_to_spiral_sweep() {
        let yaml = "skill: surface-scan\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: spiral\n        coverage_overlap: 0.2\n    - sense.inspect: { target: panel, observe: [defect] }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let mut allegro = Embodiment::parse_yaml(&std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/01-cable-insertion/embodiments/allegro.yaml"),
        ).unwrap()).unwrap();
        // The surface-scan skill declares sense.inspect; the 01 allegro does not.
        allegro.capabilities.skills.push("sense.inspect".to_string());
        let out = retarget(&skill, &allegro).expect("retarget");
        let crate::canonical::PoseExpr::SweepPath { poses, pattern } = &out.actions[0].target_pose
        else {
            panic!("expected SweepPath");
        };
        assert_eq!(pattern.as_str(), "spiral");
        assert_eq!(poses.len(), 12); // allegro 60x45 spiral -> 12 stations (raster was 9)
        // The first station is the centroid (U/2, V/2, standoff), round6-emitted.
        assert_eq!(poses[0].position, [0.1, 0.075, 0.1]);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core scan_pattern_spiral_lowers_to_spiral_sweep`
Expected: FAIL — `assertion failed: poses.len() == 12` shows `9` (the Surface arm still always calls `raster`), and/or the centroid assert fails.

- [ ] **Step 3: Dispatch on `pattern` in the Surface arm**

In `crates/rfl-core/src/translation.rs`, replace the Surface arm of the `match &p.region` in `lower_reach_scan` (currently):

```rust
        // Surface region: raster (spiral/arc fall back to raster in v0).
        crate::region::ScanRegion::Surface { size_u, size_v, .. } => {
            let (h, v) = e
                .sensor_fov(&sensor_frame)
                .map(|f| (angle_rad(&f.h_angle), angle_rad(&f.v_angle)))
                .unwrap_or((0.0, 0.0));
            crate::sigma::raster(length_m(size_u), length_m(size_v), standoff, overlap, h, v)
        }
```

with:

```rust
        // Surface region: the pattern selects the generator. arc / waypoints-on-a-
        // surface still fall back to raster in v0 (arc needs a pivot + variable
        // orientation; the waypoints pattern is driven by a Waypoints region).
        crate::region::ScanRegion::Surface { size_u, size_v, .. } => {
            let (h, v) = e
                .sensor_fov(&sensor_frame)
                .map(|f| (angle_rad(&f.h_angle), angle_rad(&f.v_angle)))
                .unwrap_or((0.0, 0.0));
            let (u, vv) = (length_m(size_u), length_m(size_v));
            match pattern {
                ScanPattern::Spiral => crate::sigma::spiral(u, vv, standoff, overlap, h, v),
                _ => crate::sigma::raster(u, vv, standoff, overlap, h, v),
            }
        }
```

(`ScanPattern` is already imported in translation.rs; the `_` arm covers Raster/Arc/Waypoints so no exhaustive-match obligation. `pattern` is `Copy`.)

- [ ] **Step 4: Run the test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core scan_pattern_spiral_lowers_to_spiral_sweep`
Expected: PASS.

- [ ] **Step 5: Run the full rfl-core suite — confirm the raster path is unchanged**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -20`
Expected: all pass, including `scan_lowers_to_sweep_path_per_fov` (still 9 for raster — the `_` arm preserves raster behavior). Warning-clean. Read before staging.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add crates/rfl-core/src/translation.rs
git commit -F - <<'EOF'
feat(core): lower reach.scan pattern: spiral to the spiral generator

Dispatch the lower_reach_scan Surface arm on the resolved pattern so
pattern: spiral compiles to the Archimedean-spiral Σ; raster (and the arc /
waypoints-on-surface v0 fallbacks) are unchanged. The emitted SweepPath.pattern
name is now truthful for spiral.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: push succeeds; final count `0 0`. Record the real hash.

---

## Task 3: spiral worked-example + conformance goldens

**Files:**
- Create: `examples/02-surface-scan/skill-spiral.yaml`
- Create: `crates/rfl-conformance/tests/surface_scan_spiral.rs`
- Create (generated): `crates/rfl-conformance/tests/snapshots/surface_scan_spiral__spiral_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Create the spiral skill file**

Create `examples/02-surface-scan/skill-spiral.yaml`:

```yaml
# Example 02 — Surface scan (spiral variant)
# The same planar surface as skill.yaml, swept with pattern: spiral instead of
# raster. Demonstrates that the pattern parameter selects the Σ generator: an
# Archimedean spiral from the surface centroid whose station count varies with each
# embodiment's sensor field of view (Principle 1), distinct from the raster Σ.
skill: surface-scan
description: >
  Spiral-scan a rectangular surface region at a fixed standoff, then inspect the
  covered surface for defects.

body:
  sequence:
    - reach.scan:
        region:
          kind: surface
          frame: panel
          size_u: 200 mm
          size_v: 150 mm
        standoff: 100 mm
        pattern: spiral
        coverage_overlap: 0.2

    - sense.inspect:
        target: panel
        observe: [defect]
```

- [ ] **Step 2: Class-1 validate the new skill file against skill-isa (validate.py is ex01-only)**

Run:

```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python -c '
import json, yaml
from jsonschema import Draft202012Validator
schema = json.load(open("schemas/skill-isa.schema.json"))
doc = yaml.safe_load(open("examples/02-surface-scan/skill-spiral.yaml"))
errs = list(Draft202012Validator(schema).iter_errors(doc))
print("skill-spiral.yaml vs skill-isa:", "OK" if not errs else errs[0].message)
'
```

Expected: `skill-spiral.yaml vs skill-isa: OK` (it differs from skill.yaml only in the `pattern` enum value, already `spiral` in the schema).

- [ ] **Step 3: Write the conformance test (snapshots not yet generated → will fail)**

Create `crates/rfl-conformance/tests/surface_scan_spiral.rs` (mirrors `surface_scan.rs`):

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the surface-scan spiral variant: pattern: spiral
//! compiles the same surface region into an Archimedean-spiral Σ that is
//! byte-deterministic, matches a committed golden, and whose every line is a valid
//! driver-interface execute message.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-spiral.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_spiral_allegro() {
    insta::assert_snapshot!("spiral_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_spiral_leap() {
    insta::assert_snapshot!("spiral_leap", jsonl_for("leap"));
}

#[test]
fn golden_spiral_pneumatic() {
    insta::assert_snapshot!("spiral_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn spiral_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn spiral_every_line_is_a_valid_execute_message() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("driver-interface.schema.json", schema).unwrap();
    let idx = compiler.compile("driver-interface.schema.json", &mut schemas).unwrap();

    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas.validate(&v, idx).unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
```

- [ ] **Step 4: Run to confirm the goldens are missing (expected fail)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test surface_scan_spiral`
Expected: the three `golden_*` tests FAIL (no stored snapshot); `spiral_generation_is_byte_identical` and `spiral_every_line_is_a_valid_execute_message` PASS.

- [ ] **Step 5: Generate and review the goldens**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test surface_scan_spiral
rm -f ~/Documents/GitHub/rfl/crates/rfl-conformance/tests/snapshots/*.snap.new
```

Then eyeball the three new `.snap` files:

```bash
cd ~/Documents/GitHub/rfl && for h in allegro leap pneumatic; do echo "== $h =="; grep -c '"message":"execute"' crates/rfl-conformance/tests/snapshots/surface_scan_spiral__spiral_$h.snap; done
```

Expected line counts (one execute message per sweep pose + one inspect action = scan poses + 1):
- `spiral_allegro` → 13 lines (12 spiral + 1 inspect)
- `spiral_leap` → 9 lines (8 spiral + 1 inspect)
- `spiral_pneumatic` → 11 lines (10 spiral + 1 inspect)

Confirm each scan line carries `"pattern":"spiral"` and the first scan pose is the centroid `[0.1,0.075,0.1]`. If counts differ from 13/9/11, STOP and reconcile with the analytic expectation before accepting (do not accept a golden you cannot explain).

- [ ] **Step 6: Run the test again to confirm goldens pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-conformance --test surface_scan_spiral`
Expected: all five PASS.

- [ ] **Step 7: Full validation batch (separate from the commit)**

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core 2>&1 | tail -5 && cargo test -p rfl-conformance 2>&1 | tail -30 && cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -15
```

Expected: rfl-core all pass; rfl-conformance all suites pass (lib + driver_protocol + envelope_conformance + retarget_determinism + surface_scan + screw_fasten + **surface_scan_spiral**); the existing `surface_scan` (raster) goldens are UNCHANGED; validate.py C1–C7 all PASS / EXIT 0. Read and confirm before staging. Also confirm no stale pending snapshots:
`ls crates/rfl-conformance/tests/snapshots/*.snap.new 2>/dev/null && echo "STALE PENDING — remove before commit" || echo "no pending snapshots"`

- [ ] **Step 8: Commit**

```bash
cd ~/Documents/GitHub/rfl && git fetch origin main --quiet && git rev-parse --abbrev-ref HEAD && git status --short
git add examples/02-surface-scan/skill-spiral.yaml crates/rfl-conformance/tests/surface_scan_spiral.rs crates/rfl-conformance/tests/snapshots/surface_scan_spiral__spiral_allegro.snap crates/rfl-conformance/tests/snapshots/surface_scan_spiral__spiral_leap.snap crates/rfl-conformance/tests/snapshots/surface_scan_spiral__spiral_pneumatic.snap
git commit -F - <<'EOF'
test(conformance): spiral worked example goldens + per-hand counts

Add the surface-scan spiral variant (examples/02-surface-scan/skill-spiral.yaml,
the same surface region with pattern: spiral) and a surface_scan_spiral
conformance test: three per-hand Class-2 goldens (allegro 12 / leap 8 /
pneumatic 10 spiral stations), generate-twice byte-equality, and per-line
driver-interface schema validation. The raster surface_scan goldens are
untouched.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
git show --stat --oneline HEAD | head -10
git merge-base --is-ancestor origin/main HEAD && git push origin main && git rev-list --left-right --count origin/main...HEAD
```

Expected: `git show --stat` lists exactly the 5 files (skill + test + 3 snapshots), no `docs/plans/`; push succeeds; final count `0 0`. Record the real hash.

---

## Final verification gate

- [ ] All three commits on `main`, each pushed with post-push `0 0`.
- [ ] `cargo test -p rfl-core` green; `cargo test -p rfl-conformance` green (7 suites incl. surface_scan_spiral); `validate.py` C1–C7 EXIT 0.
- [ ] `surface_scan` (raster) goldens unchanged (`git log --oneline -- crates/rfl-conformance/tests/snapshots/surface_scan__*.snap` shows no new commit from this increment).
- [ ] No `schemas/` change (validate.py untouched); no new `Primitive`/`ScanPattern` enum variant.
- [ ] Update README / `project_rfl.md` Implementation track / `MEMORY.md` with the three real hashes (session log NOT written to the public repo).

## Self-review (run after writing, fix inline)

**Spec coverage** (design §2 in-scope):
- `sigma::spiral` transcribing Appendix A § spiral → Task 1. ✓
- Surface-arm `pattern` dispatch (Spiral→spiral; Raster/Arc/Waypoints→raster) → Task 2. ✓
- Worked example `skill-spiral.yaml` + `surface_scan_spiral.rs` (goldens + generate-twice + boon) + Class-1 check → Task 3. ✓
- Deferred items (arc/path/volume/coverage_overlap:auto/general orientation) → no task, intentionally out of scope per design §2. ✓

**Placeholder scan:** every code step shows full code; every run step gives an exact command + expected output; counts are concrete (12/8/10, lines 13/9/11). No TBD/TODO. ✓

**Type consistency:** `spiral(size_u,size_v,standoff,coverage_overlap,h_angle_rad,v_angle_rad)` signature identical in Task 1 definition and Task 2 call; helpers `spiral_arc_length`/`solve_theta_for_arc_length` defined and used in Task 1 only; `ScanPattern::Spiral`, `PoseExpr::SweepPath { poses, pattern }`, `SweepPose.position` match the existing code read from the tree. ✓

**Determinism:** bisection (Task 1) + round6 at `SweepPose::from_pose` (existing) + generate-twice (Task 3) — consistent with the design §3.1. ✓
