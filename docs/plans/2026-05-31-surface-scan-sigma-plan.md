# surface-scan + Σ generators Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a surface-scan worked example whose retargeting compiles a region into a numeric sweep set Σ (`spec/02` Appendix A), proving the generative retarget path and establishing deterministic float serialization.

**Architecture:** Extend `rfl-core` with a `ScanRegion` input model, a pure `sigma` geometry module (raster + waypoints, transcribed from Appendix A), a `PoseExpr::SweepPath` output variant with 6-decimal-rounded float serialization, and `reach.scan` / `sense.inspect` lowering. A new self-contained `examples/02-surface-scan/` and a conformance test pin the per-FOV Σ divergence across the three hands.

**Tech Stack:** Rust (edition 2024), serde + serde_yaml + serde_json, nalgebra (Σ geometry), insta (golden), boon (schema validation).

---

## Design reference

`docs/design/2026-05-31-surface-scan-sigma-design.md`. Authoritative spec: `spec/02` Appendix A (Σ construction), `schemas/skill-isa.schema.json` `$defs/ReachScanParams` + `$defs/SenseInspectParams`, `spec/03` § Sensor descriptor.

## Conventions (same as the v0 plan)

- Reference instances are the anti-fabrication oracle; copy parameter sets from the cited schema `$defs`, never from memory.
- Quantities are carried as unit-suffixed strings; the only floats emitted are the Σ pose coordinates, rounded to 6 decimals (this task's new determinism mechanism).
- Run `cargo test` and read PASS in a step **separate** from any commit. `git add` names explicit files (never `-A`, and never the local-only `docs/plans/`). Before each commit verify branch is `main`; before push `git merge-base --is-ancestor origin/main HEAD`; after push `git rev-list --left-right --count origin/main...HEAD` is `0 0`.
- Prefix every cargo command with `export PATH="$HOME/.cargo/bin:$PATH"` (shell state does not persist between calls).
- Conventional Commits + `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

## Σ raster reference values (the test oracle, hand-computed from Appendix A)

For a surface `U = 0.20 m`, `V = 0.15 m`, `standoff = 0.10 m`, `coverage_overlap = 0.2`:

| Hand | sensor FOV (h x v) | f_u, f_v (m) | s_u, s_v (m) | n_u x n_v | poses |
|---|---|---|---|---|---|
| allegro | 60 x 45 | 0.11547, 0.08284 | 0.092376, 0.066274 | 3 x 3 | **9** |
| leap | 70 x 55 | 0.14004, 0.10412 | 0.112030, 0.083297 | 2 x 2 | **4** |
| pneumatic | 65 x 50 | 0.12742, 0.09326 | 0.101937, 0.074608 | 2 x 3 | **6** |

where `f = 2*standoff*tan(angle/2)`, `s = f*(1-overlap)`, `n = ceil(size/s)`. All `n` are clear of ceil boundaries (no ULP-flip risk). For allegro the station centres are `u_j = (j+0.5)*0.20/3 in {0.033333, 0.1, 0.166667}`, `v_k = (k+0.5)*0.15/3 in {0.025, 0.075, 0.125}`, `z = 0.10`, orientation `[1,0,0,0]` (`[x,y,z,w]`, a 180-degree rotation about x: sensor bore +z points down at the +z-normal surface, secondary axis along +u).

---

## Task 1: Float serialization + `PoseExpr::SweepPath` (`canonical.rs`)

**Files:**
- Modify: `crates/rfl-core/src/canonical.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/rfl-core/src/canonical.rs`:

```rust
    #[test]
    fn round6_normalizes_and_rounds() {
        assert_eq!(round6(0.0333333333), 0.033333);
        assert_eq!(round6(0.1), 0.1);
        assert_eq!(round6(-0.0000001), 0.0); // negative zero normalized to +0.0
    }

    #[test]
    fn sweep_path_serializes_rounded_numbers() {
        let p = crate::pose::Pose6D {
            position: [0.0333333333, 0.025, 0.1],
            orientation: nalgebra::UnitQuaternion::from_axis_angle(
                &nalgebra::Vector3::x_axis(),
                std::f64::consts::PI,
            ),
        };
        let expr = PoseExpr::SweepPath {
            pattern: "raster".into(),
            poses: vec![SweepPose::from_pose(&p)],
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"pattern\":\"raster\""));
        assert!(json.contains("\"position\":[0.033333,0.025,0.1]"));
        assert!(json.contains("\"orientation\":[1.0,0.0,0.0,0.0]"));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core canonical::tests::sweep 2>&1 | tail -15`
Expected: FAIL to compile (`round6`, `SweepPose`, `PoseExpr::SweepPath` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/canonical.rs` (near the top of the item list):

```rust
/// Round a coordinate to 6 decimals for byte-deterministic serialization (RD1c,
/// numeric form). Six decimals is sub-micron in metres; the rounding absorbs the
/// last-ULP differences a transcendental can produce across platforms. Negative
/// zero is normalized to positive zero.
#[must_use]
pub fn round6(x: f64) -> f64 {
    let r = (x * 1e6).round() / 1e6;
    if r == 0.0 { 0.0 } else { r }
}

/// A fully-resolved sweep station pose, coordinates already rounded for
/// deterministic emission. `orientation` is the unit quaternion as `[x, y, z, w]`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SweepPose {
    /// Position in R^3 (metres), in the region frame.
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}

impl SweepPose {
    /// Build a rounded `SweepPose` from a `Pose6D`.
    #[must_use]
    pub fn from_pose(p: &crate::pose::Pose6D) -> Self {
        let q = p.orientation.coords; // [x, y, z, w]
        SweepPose {
            position: [round6(p.position[0]), round6(p.position[1]), round6(p.position[2])],
            orientation: [round6(q[0]), round6(q[1]), round6(q[2]), round6(q[3])],
        }
    }
}
```

Add a `SweepPath` variant to the existing `PoseExpr` enum (after `Concrete`):

```rust
    /// A generated sweep path (reach.scan): the ordered sensor poses of the sweep
    /// set Σ (spec/02 Appendix A), numeric and rounded.
    SweepPath {
        /// The sweep pattern that generated the path.
        pattern: String,
        /// The ordered sweep poses.
        poses: Vec<SweepPose>,
    },
```

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core canonical 2>&1 | tail -15`
Expected: all canonical tests PASS (the 3 existing + 2 new).

- [ ] **Step 5: Commit**

```bash
export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core canonical 2>&1 | tail -3
```
```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/canonical.rs
git commit -m "$(printf 'feat(core): add SweepPath pose and deterministic float emit\n\nAdd PoseExpr::SweepPath carrying the generated sweep poses and a 6-\ndecimal rounding (with negative-zero normalization) so numeric Sigma\noutput is byte-deterministic (RD1c, numeric form).\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 2: `ScanRegion` input model (`region.rs`)

**Files:**
- Create: `crates/rfl-core/src/region.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod region;`)

`ScanRegion` is the v0 concrete region: a planar surface rect or an explicit waypoint list. The example `skill.yaml` (Task 7) is the deserialize oracle.

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/region.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Scan-region input model (`spec/02` Appendix A region kinds). v0 models the
//! `surface` kind (a planar rectangle) and an explicit `waypoints` list; `path`
//! and `volume` reduce to repeated surface coverage and are deferred.

use crate::quantity::Quantity;
use crate::skill_isa::FrameRef;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_surface_region() {
        let yaml = "kind: surface\nframe: panel\nsize_u: 200 mm\nsize_v: 150 mm\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Surface { frame, size_u, size_v } = r else { panic!("expected surface") };
        assert_eq!(frame, "panel");
        assert_eq!(size_u.0, "200 mm");
        assert_eq!(size_v.0, "150 mm");
    }

    #[test]
    fn parses_waypoints_region() {
        let yaml = "kind: waypoints\nframe: task\nposes:\n  - position: [0.1, 0.0, 0.2]\n    orientation: [1.0, 0.0, 0.0, 0.0]\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Waypoints { frame, poses } = r else { panic!("expected waypoints") };
        assert_eq!(frame, "task");
        assert_eq!(poses.len(), 1);
        assert_eq!(poses[0].position, [0.1, 0.0, 0.2]);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core region 2>&1 | tail -15`
Expected: FAIL to compile (`ScanRegion` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/region.rs`:

```rust
/// A scan region (`spec/02` Appendix A). `serde(tag = "kind")` discriminates by the
/// `kind` field.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ScanRegion {
    /// A planar rectangular surface of `size_u` by `size_v` in `frame`'s xy-plane,
    /// outward normal `+z`.
    Surface {
        /// The region reference frame.
        frame: FrameRef,
        /// Extent along the frame x-axis (the u direction).
        size_u: Quantity,
        /// Extent along the frame y-axis (the v direction).
        size_v: Quantity,
    },
    /// An explicit ordered list of sweep poses (the `waypoints` pattern source).
    Waypoints {
        /// The reference frame the poses are expressed in.
        frame: FrameRef,
        /// The ordered poses.
        poses: Vec<WaypointPose>,
    },
}

/// A waypoint pose as authored: position + orientation quaternion `[x, y, z, w]`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WaypointPose {
    /// Position in R^3 (metres).
    pub position: [f64; 3],
    /// Orientation unit quaternion `[x, y, z, w]`.
    pub orientation: [f64; 4],
}
```

Add to `crates/rfl-core/src/lib.rs` (alphabetical, before `pub mod skill_isa;`):

```rust
pub mod region;
```

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core region 2>&1 | tail -15`
Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/region.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): add ScanRegion input model\n\nModel the v0 scan region kinds: a planar surface rectangle and an\nexplicit waypoint list. path and volume kinds reduce to surface\ncoverage and are deferred.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 3: Typed sensor descriptor with FOV (`embodiment.rs`)

**Files:**
- Modify: `crates/rfl-core/src/embodiment.rs`

The Σ generator needs the sensor field of view. The descriptor sensor block is
`<frame>: { bore_axis: +z, fov: { h_angle: 60 deg, v_angle: 45 deg }, ... }`. v0
reads `fov.h_angle` / `fov.v_angle` for the default sensor frame.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/rfl-core/src/embodiment.rs`:

```rust
    #[test]
    fn reads_sensor_fov() {
        let e = descriptor("allegro");
        let fov = e.sensor_fov("palm_cam").expect("palm_cam fov");
        assert_eq!(fov.h_angle.0, "60 deg");
        assert_eq!(fov.v_angle.0, "45 deg");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core embodiment::tests::reads_sensor_fov 2>&1 | tail -15`
Expected: FAIL to compile (`sensor_fov` not defined).

- [ ] **Step 3: Implement**

Replace the untyped `sensors` field in `crates/rfl-core/src/embodiment.rs`:

```rust
    #[serde(default)]
    pub sensors: Option<serde_yaml::Value>,
```

with a typed map:

```rust
    /// Non-contact sensor descriptors keyed by sensor frame (`spec/03` § Sensor descriptor).
    #[serde(default)]
    pub sensors: BTreeMap<String, Sensor>,
```

Add the `Sensor` / `Fov` types (after `RoleDefaults`):

```rust
/// A non-contact sensor descriptor (`spec/03` § Sensor descriptor). Only the fields
/// the Translation Layer reads are typed; the rest are ignored.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Sensor {
    /// The sensor bore axis (e.g. `+z`).
    #[serde(default)]
    pub bore_axis: Option<String>,
    /// The angular field of view about the bore axis.
    #[serde(default)]
    pub fov: Option<Fov>,
}

/// The angular field of view `{h_angle, v_angle}` about the bore axis.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Fov {
    /// Horizontal angle (the u direction).
    pub h_angle: Quantity,
    /// Vertical angle (the v direction).
    pub v_angle: Quantity,
}
```

Add a reader method in `impl Embodiment`:

```rust
    /// The FOV of a named sensor frame, if declared.
    #[must_use]
    pub fn sensor_fov(&self, frame: &str) -> Option<&Fov> {
        self.sensors.get(frame).and_then(|s| s.fov.as_ref())
    }
```

Note: `Sensor` does not set `deny_unknown_fields`, so the descriptors' `working_range`
/ `modalities` keys are ignored.

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core embodiment 2>&1 | tail -15`
Expected: the existing 2 embodiment tests plus `reads_sensor_fov` PASS.

- [ ] **Step 5: Commit**

```bash
export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core embodiment 2>&1 | tail -3
```
```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/embodiment.rs
git commit -m "$(printf 'feat(core): type the sensor descriptor FOV\n\nParse sensors[frame].fov{h_angle, v_angle} so the Translation Layer can\nread a sensor field of view for the sweep generator.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 4: Σ raster generator (`sigma.rs`)

**Files:**
- Create: `crates/rfl-core/src/sigma.rs`
- Modify: `crates/rfl-core/src/lib.rs` (add `pub mod sigma;`)

The raster generator, a pure function transcribed from `spec/02` Appendix A. It
takes already-parsed SI scalars (the lowering does the unit parsing). Orientation is
the v0 axis-aligned constant (sensor bore `+z` over a `+z`-normal surface).

- [ ] **Step 1: Write the failing test**

In `crates/rfl-core/src/sigma.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The sweep-set Σ generators (`spec/02` Appendix A, Normative sweep-pattern
//! generators). Pure functions of SI scalars so generation is byte-deterministic.

use crate::pose::Pose6D;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::round6;

    #[test]
    fn allegro_raster_has_nine_poses() {
        // U=0.20, V=0.15, standoff=0.10, overlap=0.2, fov 60x45 deg.
        let poses = raster(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert_eq!(poses.len(), 9); // n_u=3, n_v=3
        // First station (j=0, k=0): u=0.033333, v=0.025, z=standoff.
        assert_eq!(round6(poses[0].position[0]), 0.033333);
        assert_eq!(round6(poses[0].position[1]), 0.025);
        assert_eq!(round6(poses[0].position[2]), 0.10);
        // Orientation constant 180-degrees about x: [x,y,z,w]=[1,0,0,0].
        let q = poses[0].orientation.coords;
        assert!((round6(q[0]) - 1.0).abs() < 1e-9);
        assert!(round6(q[3]).abs() < 1e-9);
    }

    #[test]
    fn raster_is_serpentine() {
        // Row k=0 ascends in u, row k=1 descends. n_u=3: poses[0..3] ascending, poses[3..6] descending.
        let poses = raster(0.20, 0.15, 0.10, 0.2, 60_f64.to_radians(), 45_f64.to_radians());
        assert!(poses[0].position[0] < poses[2].position[0]); // row 0 ascending
        assert!(poses[3].position[0] > poses[5].position[0]); // row 1 descending
    }

    #[test]
    fn leap_and_pneumatic_have_different_counts() {
        let leap = raster(0.20, 0.15, 0.10, 0.2, 70_f64.to_radians(), 55_f64.to_radians());
        let pneu = raster(0.20, 0.15, 0.10, 0.2, 65_f64.to_radians(), 50_f64.to_radians());
        assert_eq!(leap.len(), 4); // 2x2
        assert_eq!(pneu.len(), 6); // 2x3
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core sigma 2>&1 | tail -15`
Expected: FAIL to compile (`raster` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/sigma.rs`:

```rust
use nalgebra::{UnitQuaternion, Vector3};

/// The fixed orientation for a v0 axis-aligned surface station: the sensor bore
/// (`+z`) points anti-parallel to the surface normal (`+z`) with the secondary axis
/// along `+u` — a 180-degree rotation about x. General bore/normal orientation is
/// deferred.
fn station_orientation() -> UnitQuaternion<f64> {
    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), std::f64::consts::PI)
}

/// Generate the raster sweep set Σ for a planar surface (`spec/02` Appendix A,
/// raster). `size_u`/`size_v`/`standoff` are metres; `coverage_overlap` is in
/// `[0, 1)`; `h_angle_rad`/`v_angle_rad` are the full FOV angles in radians. The
/// returned poses are in the region frame, in serpentine (boustrophedon) order.
#[must_use]
pub fn raster(
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
    let n_u = (size_u / s_u).ceil() as usize;
    let n_v = (size_v / s_v).ceil() as usize;
    let orientation = station_orientation();

    let mut poses = Vec::with_capacity(n_u * n_v);
    for k in 0..n_v {
        let v = (k as f64 + 0.5) * size_v / n_v as f64;
        // Serpentine: ascending u on even rows, descending on odd rows.
        let cols: Vec<usize> = if k % 2 == 0 {
            (0..n_u).collect()
        } else {
            (0..n_u).rev().collect()
        };
        for j in cols {
            let u = (j as f64 + 0.5) * size_u / n_u as f64;
            poses.push(Pose6D { position: [u, v, standoff], orientation });
        }
    }
    poses
}
```

Add to `crates/rfl-core/src/lib.rs` (alphabetical, after `pub mod region;`):

```rust
pub mod sigma;
```

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core sigma 2>&1 | tail -15`
Expected: 3 tests PASS (9 poses, serpentine, leap=4/pneu=6).

- [ ] **Step 5: Commit**

```bash
export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core sigma 2>&1 | tail -3
```
```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/sigma.rs crates/rfl-core/src/lib.rs
git commit -m "$(printf 'feat(core): add the raster Sigma generator\n\nTranscribe the spec/02 Appendix A raster construction: footprint from\nFOV and standoff, overlap-reduced pass spacing, ceil station counts,\nserpentine order. Pure and deterministic; per-FOV pose counts differ.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 5: Σ waypoints generator (`sigma.rs`)

**Files:**
- Modify: `crates/rfl-core/src/sigma.rs`

The degenerate generator: the caller's poses verbatim (`spec/02` Appendix A,
waypoints).

- [ ] **Step 1: Write the failing test**

Add to the `sigma` `tests` module:

```rust
    #[test]
    fn waypoints_passes_through() {
        let wps = vec![
            ([0.1, 0.0, 0.2], [1.0, 0.0, 0.0, 0.0]),
            ([0.2, 0.1, 0.2], [1.0, 0.0, 0.0, 0.0]),
        ];
        let poses = waypoints(&wps);
        assert_eq!(poses.len(), 2);
        assert_eq!(poses[1].position, [0.2, 0.1, 0.2]);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core sigma::tests::waypoints 2>&1 | tail -15`
Expected: FAIL to compile (`waypoints` not defined).

- [ ] **Step 3: Implement**

Add to `crates/rfl-core/src/sigma.rs`:

```rust
/// The waypoints generator (`spec/02` Appendix A, waypoints): the caller's ordered
/// poses verbatim. `standoff` / `coverage_overlap` / `fov` are ignored. Each input
/// is `(position, orientation [x, y, z, w])`.
#[must_use]
pub fn waypoints(wps: &[([f64; 3], [f64; 4])]) -> Vec<Pose6D> {
    wps.iter()
        .map(|(position, q)| Pose6D {
            position: *position,
            orientation: UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                q[3], q[0], q[1], q[2], // Quaternion::new(w, x, y, z)
            )),
        })
        .collect()
}
```

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core sigma 2>&1 | tail -15`
Expected: 4 sigma tests PASS.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/sigma.rs
git commit -m "$(printf 'feat(core): add the waypoints Sigma generator\n\nThe degenerate generator returns the caller-supplied poses verbatim\n(spec/02 Appendix A, waypoints), normalizing the quaternion ordering.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 6: Parse and lower `reach.scan` + `sense.inspect`

**Files:**
- Modify: `crates/rfl-core/src/skill_isa.rs` (enum variants + param structs)
- Modify: `crates/rfl-core/src/translation.rs` (lowering + gate)

The two new primitives are added to the `Primitive` enum **and** lowered in the same
task: the v0 `lower` / `check_capability` matches are exhaustive (no catch-all), so
adding enum variants without their match arms would not compile. `reach.scan` is the
`reach.*` baseline (no capability gate); `sense.inspect` is gated by the
`sense.inspect` key.

- [ ] **Step 1: Write the failing tests**

Add to the `parse_tests` module in `crates/rfl-core/src/skill_isa.rs`:

```rust
    #[test]
    fn parses_scan_primitives() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: raster\n        coverage_overlap: 0.2\n    - sense.inspect:\n        target: panel\n        observe: [defect]\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::ReachScan(p)) = &s.body.sequence[0] else { panic!() };
        assert_eq!(p.standoff.0, "100 mm");
        assert!(matches!(p.pattern, Some(ScanPattern::Raster)));
        assert!(matches!(&s.body.sequence[1], Statement::Primitive(Primitive::SenseInspect(_))));
    }
```

Add to the `tests` module in `crates/rfl-core/src/translation.rs`:

```rust
    #[test]
    fn scan_lowers_to_sweep_path_per_fov() {
        let yaml = "skill: surface-scan\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: raster\n        coverage_overlap: 0.2\n    - sense.inspect: { target: panel, observe: [defect] }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let allegro = Embodiment::parse_yaml(&std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/01-cable-insertion/embodiments/allegro.yaml"),
        ).unwrap()).unwrap();
        let out = retarget(&skill, &allegro).expect("retarget");
        assert_eq!(out.actions.len(), 2);
        let crate::canonical::PoseExpr::SweepPath { poses, pattern } = &out.actions[0].target_pose else {
            panic!("expected SweepPath");
        };
        assert_eq!(pattern.as_str(), "raster");
        assert_eq!(poses.len(), 9); // allegro palm_cam 60x45 -> 9 sweep poses
        assert_eq!(out.suffixes, vec!["scan", "inspect"]);
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core scan 2>&1 | tail -15`
Expected: FAIL to compile (`Primitive::ReachScan`, `ScanPattern`, lowering not defined).

- [ ] **Step 3a: Add the parser types (`skill_isa.rs`)**

Add two variants to the `Primitive` enum (after `ReachRetract`):

```rust
    /// `reach.scan`.
    #[serde(rename = "reach.scan")]
    ReachScan(ReachScan),
    /// `sense.inspect`.
    #[serde(rename = "sense.inspect")]
    SenseInspect(SenseInspect),
```

Add the param structs + pattern enum (near the other primitive structs):

```rust
/// `reach.scan` sweep pattern (`$defs/ReachScanParams.pattern`). v0 lowers raster
/// and waypoints; spiral and arc are accepted but fall back to raster (deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPattern {
    /// Serpentine surface coverage (default).
    Raster,
    /// Caller-supplied poses verbatim.
    Waypoints,
    /// Archimedean spiral (deferred; falls back to raster).
    Spiral,
    /// Swept arc (deferred; falls back to raster).
    Arc,
}

/// `reach.scan` parameters (v0 subset of `$defs/ReachScanParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachScan {
    /// The region to cover.
    pub region: crate::region::ScanRegion,
    /// Sensor-to-region distance maintained during the sweep.
    pub standoff: Quantity,
    /// Sweep pattern (default raster).
    #[serde(default)]
    pub pattern: Option<ScanPattern>,
    /// Sensor frame whose coverage matters (default the embodiment sensor frame).
    #[serde(default)]
    pub sensor_frame: Option<FrameRef>,
    /// Overlap between passes (v0 requires an explicit ratio; auto is deferred).
    #[serde(default)]
    pub coverage_overlap: Option<f64>,
}

/// `sense.inspect` parameters (v0 subset of `$defs/SenseInspectParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseInspect {
    /// What to observe (a let-reference or frame name in the reference skill).
    pub target: Ref,
    /// What to capture (interpretation is out of RFL scope).
    #[serde(default)]
    pub observe: Option<Vec<String>>,
}
```

- [ ] **Step 3b: Add the lowering + gate (`translation.rs`)**

Extend the `canonical` import to add `SweepPose`, and the `skill_isa` import to add
`ReachScan`, `ScanPattern`, `SenseInspect`:

```rust
use crate::canonical::{
    AlignSpec, CanonicalAction, Envelope, Monitor, MotionBounds, PoseExpr, ProxySpec,
    SweepPose, TactileTargetOut, TimingHints, TimingMode,
};
```
```rust
use crate::skill_isa::{
    Axes, Axis, Compliance, ForceInsertFit, GraspPinch, GraspRelease, Primitive, ReachAlign,
    ReachRetract, ReachScan, ScanPattern, SenseInspect, Skill, Statement, TactileTargetArg,
    TransportMoveToPose,
};
```

Add the unit-parse helpers and the two lowerings (near the other lowerings):

```rust
/// Parse a length quantity to metres (m / mm / cm). Returns 0.0 on a malformed value
/// (v0 scan regions are author-declared, not referenced).
fn length_m(q: &Quantity) -> f64 {
    match q.parse() {
        Some((v, "m")) => v,
        Some((v, "mm")) => v / 1000.0,
        Some((v, "cm")) => v / 100.0,
        _ => 0.0,
    }
}

/// Parse an angle quantity to radians (deg / rad).
fn angle_rad(q: &Quantity) -> f64 {
    match q.parse() {
        Some((v, "deg")) => v.to_radians(),
        Some((v, "rad")) => v,
        _ => 0.0,
    }
}

/// Lower `reach.scan`: compile the region into the sweep set Σ and emit one action
/// carrying it as a `SweepPath`. reach.* is the baseline (no capability gate); the
/// FOV comes from the embodiment sensor descriptor.
fn lower_reach_scan(p: &ReachScan, e: &Embodiment) -> CanonicalAction {
    let sensor_frame = p
        .sensor_frame
        .clone()
        .unwrap_or_else(|| e.sensor_frame().to_string());
    let pattern = p.pattern.unwrap_or(ScanPattern::Raster);
    let standoff = length_m(&p.standoff);
    let overlap = p.coverage_overlap.unwrap_or(0.0);

    let poses = match &p.region {
        crate::region::ScanRegion::Waypoints { poses, .. } => {
            let wps: Vec<([f64; 3], [f64; 4])> =
                poses.iter().map(|w| (w.position, w.orientation)).collect();
            crate::sigma::waypoints(&wps)
        }
        // Surface region: raster (spiral/arc fall back to raster in v0).
        crate::region::ScanRegion::Surface { size_u, size_v, .. } => {
            let (h, v) = e
                .sensor_fov(&sensor_frame)
                .map(|f| (angle_rad(&f.h_angle), angle_rad(&f.v_angle)))
                .unwrap_or((0.0, 0.0));
            crate::sigma::raster(length_m(size_u), length_m(size_v), standoff, overlap, h, v)
        }
    };

    let pattern_name = match pattern {
        ScanPattern::Raster => "raster",
        ScanPattern::Waypoints => "waypoints",
        ScanPattern::Spiral => "spiral",
        ScanPattern::Arc => "arc",
    };
    let sweep_poses: Vec<SweepPose> = poses.iter().map(SweepPose::from_pose).collect();
    CanonicalAction {
        target_frame: sensor_frame,
        target_pose: PoseExpr::SweepPath { pattern: pattern_name.to_string(), poses: sweep_poses },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Lower `sense.inspect`: a perception action observing the target (like sense.locate).
fn lower_sense_inspect(p: &SenseInspect, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.sensor_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}
```

Add the two arms to the `lower` match (it stays exhaustive):

```rust
        Primitive::ReachScan(p) => (lower_reach_scan(p, e), "scan"),
        Primitive::SenseInspect(p) => (lower_sense_inspect(p, e), "inspect"),
```

Add the two arms to `check_capability` (reach.scan is baseline; sense.inspect is keyed):

```rust
        Primitive::ReachScan(_) => return Ok(()),
        Primitive::SenseInspect(_) => "sense.inspect",
```

- [ ] **Step 4: Run the tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core 2>&1 | tail -15`
Expected: all rfl-core tests PASS, including `parses_scan_primitives` and
`scan_lowers_to_sweep_path_per_fov` (9 poses on allegro).

- [ ] **Step 5: Commit**

```bash
export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-core 2>&1 | tail -3
```
```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-core/src/skill_isa.rs crates/rfl-core/src/translation.rs
git commit -m "$(printf 'feat(core): parse and lower reach.scan and sense.inspect\n\nAdd the two scan-example primitives (parameters per the schema defs) and\nlower them together so the exhaustive match stays compiling: reach.scan\ncompiles its region into a SweepPath via sigma (reach.* baseline, no\ngate); sense.inspect is a perception action gated by sense.inspect.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 7: The `examples/02-surface-scan/` worked example

**Files:**
- Create: `examples/02-surface-scan/skill.yaml`
- Create: `examples/02-surface-scan/embodiments/allegro.yaml`, `leap.yaml`, `pneumatic-6f.yaml`
- Create: `examples/02-surface-scan/run.py`

- [ ] **Step 1: Create the skill**

`examples/02-surface-scan/skill.yaml`:

```yaml
# Example 02 — Surface scan
# A pure perception skill: raster-scan a planar surface, then inspect it. Demonstrates
# the generative retarget path — reach.scan compiles the region into a sweep set Σ
# whose pose count varies with each embodiment's sensor field of view (Principle 1).
skill: surface-scan
description: >
  Raster-scan a rectangular surface region at a fixed standoff, then inspect the
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
        pattern: raster
        coverage_overlap: 0.2

    - sense.inspect:
        target: panel
        observe: [defect]
```

- [ ] **Step 2: Create the descriptors (self-contained copies + `sense.inspect`)**

Copy each cable-insertion descriptor and add `sense.inspect` to its `capabilities.skills`. For `examples/02-surface-scan/embodiments/allegro.yaml`, copy `examples/01-cable-insertion/embodiments/allegro.yaml` verbatim, then change the skills line to:

```yaml
    skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, force.insert_fit, sense.locate, sense.inspect]
```

Do the same for `leap.yaml` (append `sense.inspect` to its skills) and `pneumatic-6f.yaml` (append `sense.inspect`). The sensor blocks (`palm_cam` / `wrist_cam` / `head_cam` with their FOVs) are copied unchanged — they drive the per-hand Σ divergence.

Run (verify they build): `export PATH="$HOME/.cargo/bin:$PATH"; cargo build -p rfl-cli 2>&1 | tail -2`

- [ ] **Step 3: Create `run.py`**

Copy `examples/01-cable-insertion/run.py` to `examples/02-surface-scan/run.py`, changing the docstring's task description and the `--embodiment` help text to surface-scan. The body (argparse + `cargo run -p rfl-cli -- retarget`) is identical; `HERE` resolves the new directory automatically.

- [ ] **Step 4: Run the CLI on all three hands**

Run:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
for h in allegro leap pneumatic-6f; do
  echo -n "$h sweep poses: "
  cargo run -q -p rfl-cli -- retarget examples/02-surface-scan/skill.yaml \
    --embodiment examples/02-surface-scan/embodiments/$h.yaml | head -1 \
    | python3 -c "import sys,json; print(len(json.load(sys.stdin)['canonical_action']['target_pose']['poses']))"
done
```
Expected: allegro `9`, leap `4`, pneumatic-6f `6` — the per-FOV divergence.

- [ ] **Step 5: Commit**

```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add examples/02-surface-scan/skill.yaml examples/02-surface-scan/embodiments examples/02-surface-scan/run.py
git commit -m "$(printf 'docs(example): add the surface-scan worked example\n\nA raster scan + inspect skill retargeting onto the three hands; each\nsensor FOV yields a different sweep-set size (allegro 9, leap 4,\npneumatic 6), demonstrating the generative retarget path.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 8: Conformance test class 2 for the scan example

**Files:**
- Create: `crates/rfl-conformance/tests/surface_scan.rs`
- Snapshots: `crates/rfl-conformance/tests/snapshots/` (generated)

Reuse the public `retarget_example_to_jsonl` helper in `rfl-conformance/src/lib.rs`.

- [ ] **Step 1: Write the test**

`crates/rfl-conformance/tests/surface_scan.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the surface-scan example: the generative Σ output
//! is byte-deterministic, matches a committed golden, and each emitted line is a
//! valid driver-interface execute message.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_scan_allegro() {
    insta::assert_snapshot!("scan_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_scan_leap() {
    insta::assert_snapshot!("scan_leap", jsonl_for("leap"));
}

#[test]
fn golden_scan_pneumatic() {
    insta::assert_snapshot!("scan_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn scan_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(jsonl_for(stem), jsonl_for(stem), "non-deterministic for {stem}");
    }
}

#[test]
fn scan_every_line_is_a_valid_execute_message() {
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

- [ ] **Step 2: Generate and review the snapshots**

Run:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
INSTA_UPDATE=always cargo test -p rfl-conformance 2>&1 | tail -10
```
Then inspect `crates/rfl-conformance/tests/snapshots/surface_scan__scan_*.snap`: confirm allegro has 9 sweep poses, leap 4, pneumatic 6, each pose `orientation [1.0,0.0,0.0,0.0]`, positions rounded to 6 decimals.

- [ ] **Step 3: Confirm a clean run passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH"; cargo test -p rfl-conformance 2>&1 | tail -8`
Expected: all conformance tests PASS (the 5 cable + 5 scan).

- [ ] **Step 4: Commit (test + snapshots)**

```bash
git rev-parse --abbrev-ref HEAD   # expect main
git add crates/rfl-conformance/tests/surface_scan.rs crates/rfl-conformance/tests/snapshots
git commit -m "$(printf 'test(conformance): add surface-scan determinism class 2\n\nPin the per-FOV sweep-set golden for the three hands, the generate-twice\nbyte-equality, and per-line execute-schema validation. The SweepPath\ncanonical action passes the open canonical_action floor.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 9: Closeout

**Files:** verification + memory only.

- [ ] **Step 1: Full suite + input-schema suite green**

Run:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test 2>&1 | tail -12
uv run --with jsonschema --with pyyaml python schemas/validate.py; echo "EXIT=$?"
```
Expected: all crates' tests PASS; `validate.py` `EXIT=0`. The cable goldens are unchanged.

- [ ] **Step 2: Push and verify**

```bash
git fetch origin
git merge-base --is-ancestor origin/main HEAD && echo FF_OK || echo NOT_FF
git push origin main
git rev-list --left-right --count origin/main...HEAD   # expect 0 0
```

- [ ] **Step 3: Update memory** `project_rfl.md` (the Implementation track) with the surface-scan milestone and the real pushed commit hashes (from `git log --oneline`, never predicted).

---

## Self-review notes

- **Spec coverage:** design § 3 architecture → Tasks 1 (SweepPath+float), 2 (region), 3 (sensor FOV), 4–5 (sigma), 6 (parse+lower); design § 4 Σ raster → Task 4; design § 5 lowering → Task 6; design § 6 testing → Tasks 4/5/8; the example (decision B1) → Task 7.
- **Exhaustive-match coupling:** the v0 `lower` / `check_capability` matches have no catch-all, so the enum-variant addition and its match arms are committed together in Task 6 (not split), keeping every commit compiling.
- **Type consistency:** `SweepPose` defined in Task 1 is constructed in Task 6 and serialized in Task 8; `PoseExpr::SweepPath { pattern, poses }` field names are fixed in Task 1 and matched in Tasks 6/8; `ScanRegion::{Surface, Waypoints}` field names (`frame`/`size_u`/`size_v`/`poses`) are fixed in Task 2 and read in Task 6; `sigma::raster(size_u, size_v, standoff, coverage_overlap, h_angle_rad, v_angle_rad)` and `sigma::waypoints(&[([f64;3],[f64;4])])` signatures are fixed in Tasks 4/5 and called in Task 6; `ScanPattern` variants fixed in Task 6.
- **Module cycle note:** `region` uses `skill_isa::FrameRef` and `skill_isa` uses `region::ScanRegion`; intra-crate module reference cycles compile fine in Rust.
- **Known v0 simplifications (called out, pinned by tests):** spiral/arc fall back to raster (Task 6) and are not in the example; orientation is the axis-aligned constant (Task 4); `coverage_overlap` is explicit (no auto); `length_m`/`angle_rad` cover only the units the example uses.
- **Determinism caveat:** 6-decimal rounding makes same-platform generation exactly reproducible and absorbs transcendental ULP cross-platform, except where a value sits exactly on a `ceil` boundary; the example dimensions are chosen clear of those boundaries (the reference-values table).
