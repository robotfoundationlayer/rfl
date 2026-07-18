# Σ path/volume sweep generators — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Steps use `- [ ]` for tracking.
>
> **LOCAL-ONLY.** This file lives in `docs/plans/` — NEVER `git add` it.

**Goal:** Implement the two deferred Σ sweep generators — `path` (1-D raster over a polyline) and `volume` (boustrophedon stack of surface layers) — per spec/02 Appendix A, mirroring the `arc` precedent (`f79aa6d`).

**Architecture:** Each region kind = a `ScanRegion` variant (`region.rs`, Deserialize-only input model) → a pure `sigma::` generator fn (`sigma.rs`) → a `lower_reach_scan` dispatch arm (`translation.rs`, exhaustive match) → an example skill + per-embodiment goldens. `pattern` stays orthogonal (`pattern: raster` for both); no new `ScanPattern` variant; no schema change.

**Tech Stack:** Rust (rfl-core, rfl-conformance), nalgebra, insta goldens, boon JSON-schema validation.

**Gates (every commit, read actual exit codes — no `| tail`):**
```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --all-targets 2>&1 | grep -c "test result: FAILED"   # expect 0
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
uv run --with jsonschema --with pyyaml python schemas/validate.py
python3 scripts/md_lint.py        # only if md changed
```
Per-commit guard before staging:
```
git diff --cached --name-only | grep -E '^whitepaper/|^spec/02|bindings/python|friend-review' && echo ABORT || echo ok
```
Push ff-safe (`git fetch origin main` → `HEAD~1 == origin/main`), then `gh run watch` all 8 jobs green.

---

## Task 1: `path` generator (Increment 1, one commit)

**Files:**
- Modify: `crates/rfl-core/src/sigma.rs` (add `pub fn path` + unit tests)
- Modify: `crates/rfl-core/src/region.rs` (add `ScanRegion::Path` variant + parse test)
- Modify: `crates/rfl-core/src/translation.rs` (add `Path` dispatch arm in `lower_reach_scan`, ~line 2135)
- Create: `examples/02-surface-scan/skill-path.yaml`
- Create: `crates/rfl-conformance/tests/surface_scan_path.rs`
- Create (generated): `crates/rfl-conformance/tests/snapshots/surface_scan_path__path_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the failing sigma unit tests**

Append to the `tests` module in `crates/rfl-core/src/sigma.rs`:

```rust
    #[test]
    fn path_resamples_polyline_at_s_u_spacing() {
        // L-shape: (0,0,0)->(0.2,0,0)->(0.2,0.15,0), total length L=0.35.
        // allegro fov_h=60deg, standoff=0.10, overlap=0.2 => f_u=0.115470,
        // s_u=0.092376; n=ceil(0.35/0.092376)=ceil(3.789)=4. Analytic.
        let pts = [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.2, 0.15, 0.0]];
        let poses = path(&pts, 0.10, 0.2, 60_f64.to_radians());
        assert_eq!(poses.len(), 4);
        // First centred station at ell = 0.5*L/4 = 0.04375, on segment 1 (x-axis):
        // position = point + standoff*z_hat.
        assert_eq!(round6(poses[0].position[0]), 0.04375);
        assert_eq!(round6(poses[0].position[1]), 0.0);
        assert_eq!(round6(poses[0].position[2]), 0.10);
        // Orientation is the flat-normal surface convention (bore anti-parallel +z).
        let q = poses[0].orientation.coords;
        assert!((round6(q[0]) - 1.0).abs() < 1e-9);
        assert!(round6(q[3]).abs() < 1e-9);
    }

    #[test]
    fn path_stations_lie_on_the_polyline_at_standoff() {
        let pts = [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.2, 0.15, 0.0]];
        let poses = path(&pts, 0.10, 0.2, 60_f64.to_radians());
        for p in &poses {
            assert_eq!(round6(p.position[2]), 0.10); // every station standoff above z=0
            // (x,y) is on the L: either y==0 with x in [0,0.2], or x==0.2 with y in [0,0.15].
            let on_seg1 = round6(p.position[1]) == 0.0
                && p.position[0] >= -1e-9
                && p.position[0] <= 0.2 + 1e-9;
            let on_seg2 = (round6(p.position[0]) - 0.2).abs() < 1e-9
                && p.position[1] >= -1e-9
                && p.position[1] <= 0.15 + 1e-9;
            assert!(on_seg1 || on_seg2, "station off the polyline: {:?}", p.position);
        }
    }

    #[test]
    fn path_count_varies_per_fov_and_degenerates_safely() {
        let pts = [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]]; // L=0.5
        let mk = |h: f64| path(&pts, 0.10, 0.2, h.to_radians()).len();
        // Smaller FOV -> smaller footprint -> finer spacing -> more stations.
        assert!(mk(40.0) > mk(60.0));
        // A flat (zero) FOV degenerates to a single midpoint station, never diverges.
        assert_eq!(mk(0.0), 1);
        let mid = path(&pts, 0.10, 0.2, 0.0_f64);
        assert_eq!(round6(mid[0].position[0]), 0.25); // midpoint of the segment
    }

    #[test]
    fn path_single_point_yields_one_station() {
        let pts = [[0.1, 0.05, 0.0]];
        let poses = path(&pts, 0.10, 0.2, 60_f64.to_radians());
        assert_eq!(poses.len(), 1);
        assert_eq!(round6(poses[0].position[0]), 0.1);
        assert_eq!(round6(poses[0].position[2]), 0.10);
    }
```

- [ ] **Step 2: Run to verify it fails (does not compile — `path` undefined)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core sigma::tests::path 2>&1 | grep -E "error\[|cannot find"`
Expected: compile error `cannot find function path in this scope`.

- [ ] **Step 3: Implement `sigma::path`**

Insert into `crates/rfl-core/src/sigma.rs` after the `arc` fn (before `#[cfg(test)]`):

```rust
/// Generate the `path` sweep set Σ (`spec/02` Appendix A, Region kinds beyond
/// surface): stations spaced `s_u` along a caller-supplied polyline of surface
/// points — a one-dimensional `raster`. Stations are placed at centred
/// arc-length positions `(j + 0.5)·L/n` along the polyline; each pose sits a
/// `standoff` above its point along `+z` with the flat-normal surface
/// orientation (general per-point normals are deferred). `points` are in the
/// region frame (metres); only `h_angle_rad` is used (single pass).
#[must_use]
pub fn path(
    points: &[[f64; 3]],
    standoff: f64,
    coverage_overlap: f64,
    h_angle_rad: f64,
) -> Vec<Pose6D> {
    let orientation = station_orientation();
    if points.is_empty() {
        return Vec::new();
    }
    let seg_len = |a: &[f64; 3], b: &[f64; 3]| {
        let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    let total: f64 = points.windows(2).map(|w| seg_len(&w[0], &w[1])).sum();
    let f_u = 2.0 * standoff * (h_angle_rad / 2.0).tan();
    let s_u = f_u * (1.0 - coverage_overlap);

    // Degenerate guard (mirrors arc/spiral): a flat FOV or a zero-length path
    // gives no advance — emit a single station at the path midpoint.
    let parametric = s_u > 0.0 && total > 0.0;
    let n = if parametric {
        (total / s_u).ceil().max(1.0) as usize
    } else {
        1
    };

    // The surface point at arc length `ell` along the polyline (clamped).
    let point_at = |ell: f64| -> [f64; 3] {
        if total <= 0.0 {
            return points[0];
        }
        let mut acc = 0.0;
        for w in points.windows(2) {
            let l = seg_len(&w[0], &w[1]);
            if l == 0.0 {
                continue;
            }
            if acc + l >= ell {
                let t = (ell - acc) / l;
                return [
                    w[0][0] + t * (w[1][0] - w[0][0]),
                    w[0][1] + t * (w[1][1] - w[0][1]),
                    w[0][2] + t * (w[1][2] - w[0][2]),
                ];
            }
            acc += l;
        }
        *points.last().unwrap()
    };

    let mut poses = Vec::with_capacity(n);
    for j in 0..n {
        let ell = if parametric {
            (j as f64 + 0.5) * total / n as f64
        } else {
            total / 2.0
        };
        let p = point_at(ell);
        poses.push(Pose6D {
            position: [p[0], p[1], p[2] + standoff],
            orientation,
        });
    }
    poses
}
```

- [ ] **Step 4: Run to verify the unit tests pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core sigma::tests::path 2>&1 | grep "test result"`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: Add the `ScanRegion::Path` variant + parse test**

In `crates/rfl-core/src/region.rs`, add to the `ScanRegion` enum (after `Arc { .. }`, before `Waypoints`):

```rust
    /// A polyline of surface points in `frame` swept as a one-dimensional raster:
    /// stations spaced `s_u` along the path (`spec/02` Appendix A, Region kinds
    /// beyond surface). Points are in the region frame (metres); v0 uses the
    /// flat-normal surface orientation.
    Path {
        /// The region reference frame.
        frame: FrameRef,
        /// The ordered surface points of the polyline (metres).
        points: Vec<[f64; 3]>,
    },
```

Add to the `tests` module in `region.rs`:

```rust
    #[test]
    fn parses_path_region() {
        let yaml = "kind: path\nframe: panel\npoints:\n  - [0.0, 0.0, 0.0]\n  - [0.2, 0.0, 0.0]\n  - [0.2, 0.15, 0.0]\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Path { frame, points } = r else {
            panic!("expected path")
        };
        assert_eq!(frame, "panel");
        assert_eq!(points.len(), 3);
        assert_eq!(points[2], [0.2, 0.15, 0.0]);
    }
```

- [ ] **Step 6: Add the `Path` dispatch arm**

In `crates/rfl-core/src/translation.rs` `lower_reach_scan`, inside `match &p.region`, add (after the `Arc` arm, before the closing `}` of the match at ~line 2174):

```rust
        // Path region: stations spaced s_u along the caller's polyline, a 1-D
        // raster (spec/02 Appendix A). FOV's h_angle sets the spacing.
        crate::region::ScanRegion::Path { points, .. } => {
            let (h, _v) = e.sensor_fov(&sensor_frame).map_or((0.0, 0.0), |f| {
                (angle_rad(&f.h_angle), angle_rad(&f.v_angle))
            });
            crate::sigma::path(points, standoff, overlap, h)
        }
```

- [ ] **Step 7: Verify the workspace compiles + all existing tests pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace --all-targets 2>&1 | grep -c "test result: FAILED"`
Expected: `0`.

- [ ] **Step 8: Create the example skill `examples/02-surface-scan/skill-path.yaml`**

```yaml
# Example 02 — Surface scan (path variant)
# Follow a caller-supplied polyline across the surface (e.g. a weld seam or an
# L-shaped feature) rather than filling a rectangle. Demonstrates the path
# generator (spec/02 Appendix A, Region kinds beyond surface): stations are
# spaced s_u along the path — a one-dimensional raster — so the station count
# follows each embodiment's sensor field of view (Principle 1).
skill: surface-scan
description: >
  Path-scan an L-shaped surface feature by sweeping the sensor along a polyline
  at a fixed standoff, then inspect the covered surface for defects.

body:
  sequence:
    - reach.scan:
        region:
          kind: path
          frame: panel
          points:
            - [0.0, 0.0, 0.0]
            - [0.2, 0.0, 0.0]
            - [0.2, 0.15, 0.0]
        standoff: 100 mm
        pattern: raster
        coverage_overlap: 0.2

    - sense.inspect:
        target: panel
        observe: [defect]
```

- [ ] **Step 9: Create the golden test `crates/rfl-conformance/tests/surface_scan_path.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the surface-scan path variant: a `path` region
//! compiles into a 1-D raster Σ — stations spaced s_u along the polyline
//! (spec/02 Appendix A, Region kinds beyond surface). Byte-deterministic,
//! matches a committed golden, and every line is a valid driver-interface
//! execute message.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-path.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_path_allegro() {
    insta::assert_snapshot!("path_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_path_leap() {
    insta::assert_snapshot!("path_leap", jsonl_for("leap"));
}

#[test]
fn golden_path_pneumatic() {
    insta::assert_snapshot!("path_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn path_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(
            jsonl_for(stem),
            jsonl_for(stem),
            "non-deterministic for {stem}"
        );
    }
}

#[test]
fn path_every_line_is_a_valid_execute_message() {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("driver-interface.schema.json", schema)
        .unwrap();
    let idx = compiler
        .compile("driver-interface.schema.json", &mut schemas)
        .unwrap();

    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas
                .validate(&v, idx)
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
```

- [ ] **Step 10: Generate the goldens + verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test surface_scan_path 2>&1 | grep "test result"`
Then delete any stray `.snap.new`: `find crates/rfl-conformance/tests/snapshots -name '*.snap.new' -delete`
Re-run WITHOUT update to confirm the goldens pass: `cargo test -p rfl-conformance --test surface_scan_path 2>&1 | grep "test result"`
Expected: `test result: ok. 5 passed`.
Eyeball the 3 `.snap` files: each is a 2-line JSONL (the scan execute message + the sense.inspect), the scan line's `poses` array length matching the analytic count.

- [ ] **Step 11: Run the full gate suite**

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace --all-targets 2>&1 | grep -c "test result: FAILED"   # 0
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
uv run --with jsonschema --with pyyaml python schemas/validate.py
```
Expected: 0 failures, fmt clean, clippy clean, validate.py all checks pass.

- [ ] **Step 12: Commit (one commit)**

```bash
git diff --cached --name-only | grep -E '^whitepaper/|^spec/02|bindings/python|friend-review' && echo ABORT || echo ok
git add crates/rfl-core/src/sigma.rs crates/rfl-core/src/region.rs crates/rfl-core/src/translation.rs \
  examples/02-surface-scan/skill-path.yaml crates/rfl-conformance/tests/surface_scan_path.rs \
  crates/rfl-conformance/tests/snapshots/surface_scan_path__path_allegro.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_path__path_leap.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_path__path_pneumatic.snap
git commit -m "$(cat <<'EOF'
feat(sigma): implement the Σ path sweep-pattern generator (spec/02 App A)

A `path` region sweeps stations spaced s_u along a caller-supplied polyline of
surface points — a one-dimensional raster (spec/02 Appendix A, Region kinds
beyond surface):

- ScanRegion::Path { frame, points } (points in the region frame, metres).
- sigma::path: centred arc-length placement (j+0.5)·L/n at spacing s_u, each
  pose a standoff above its point with the flat-normal surface orientation;
  degenerate flat-FOV / zero-length path falls back to a single midpoint
  station, mirroring arc/spiral.
- lower_reach_scan dispatches a Path region to sigma::path.
- examples/02-surface-scan/skill-path.yaml (L-shaped feature) + per-embodiment
  goldens (surface_scan_path) + sigma/region unit tests pinning the count, the
  on-polyline property, centred placement, and the single-point/flat-FOV guards.

The volume region kind remains; orthogonal pattern field stays raster.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 13: Push ff-safe + watch CI**

```bash
git fetch origin main
git rev-parse HEAD~1 && git rev-parse origin/main   # must be equal
git push origin main
gh run watch "$(gh run list -L1 --json databaseId -q '.[0].databaseId')" --exit-status
```
Expected: all 8 jobs green.

---

## Task 2: `volume` generator (Increment 2, one commit)

**Files:**
- Modify: `crates/rfl-core/src/sigma.rs` (add `pub fn volume` + unit tests)
- Modify: `crates/rfl-core/src/region.rs` (add `ScanRegion::Volume` variant + parse test)
- Modify: `crates/rfl-core/src/translation.rs` (add `Volume` dispatch arm)
- Create: `examples/02-surface-scan/skill-volume.yaml`
- Create: `crates/rfl-conformance/tests/surface_scan_volume.rs`
- Create (generated): `crates/rfl-conformance/tests/snapshots/surface_scan_volume__volume_{allegro,leap,pneumatic}.snap`

- [ ] **Step 1: Write the failing sigma unit tests**

Append to the `tests` module in `crates/rfl-core/src/sigma.rs`:

```rust
    #[test]
    fn volume_stacks_surface_layers_along_z() {
        // U=0.20, V=0.15, W=0.20, standoff=0.10, overlap=0.2, allegro fov 60x45.
        // In-plane raster: n_u=ceil(0.2/0.092376)=3, n_v=ceil(0.15/0.066274)=3 => 9/layer.
        // Layers: s_v=0.066274, n_w=ceil(0.20/0.066274)=ceil(3.018)=4 => 4 layers.
        // Total = 4*9 = 36. Analytic.
        let poses = volume(
            0.20,
            0.15,
            0.20,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        assert_eq!(poses.len(), 36);
        // Every pose's (x,y) is inside the box footprint and z = w_k + standoff
        // for one of the 4 centred layer depths.
        let depths: Vec<f64> = (0..4).map(|k| (k as f64 + 0.5) * 0.20 / 4.0 + 0.10).collect();
        for p in &poses {
            assert!(p.position[0] >= 0.0 && p.position[0] <= 0.20 + 1e-9);
            assert!(p.position[1] >= 0.0 && p.position[1] <= 0.15 + 1e-9);
            assert!(
                depths.iter().any(|d| (p.position[2] - d).abs() < 1e-9),
                "z {} not a layer depth",
                p.position[2]
            );
        }
    }

    #[test]
    fn volume_layers_are_boustrophedon_continuous() {
        // 9 poses per layer; the last pose of layer 0 and the first pose of the
        // reversed layer 1 share (x,y) (vertical step only) => a continuous Σ.
        let poses = volume(
            0.20,
            0.15,
            0.20,
            0.10,
            0.2,
            60_f64.to_radians(),
            45_f64.to_radians(),
        );
        let per_layer = 9;
        let last0 = &poses[per_layer - 1];
        let first1 = &poses[per_layer];
        assert_eq!(round6(last0.position[0]), round6(first1.position[0]));
        assert_eq!(round6(last0.position[1]), round6(first1.position[1]));
        assert!(first1.position[2] > last0.position[2]); // next layer is deeper
    }

    #[test]
    fn volume_count_varies_per_fov_and_degenerates_safely() {
        let mk = |h: f64, v: f64| {
            volume(0.20, 0.15, 0.20, 0.10, 0.2, h.to_radians(), v.to_radians()).len()
        };
        // leap (70x55) and pneumatic (65x50) cover with different layer/grid counts.
        assert_eq!(mk(70.0, 55.0), 12); // n_w=3, 2x2 per layer
        assert_eq!(mk(65.0, 50.0), 18); // n_w=3, 2x3 per layer
        // A flat v-FOV degenerates the stacking to a single layer (raster handles u/v).
        let one = volume(0.20, 0.15, 0.20, 0.10, 0.2, 60_f64.to_radians(), 0.0);
        assert_eq!(one.len(), 9); // 1 layer * 9
    }
```

- [ ] **Step 2: Run to verify it fails (does not compile — `volume` undefined)**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core sigma::tests::volume 2>&1 | grep -E "error\[|cannot find"`
Expected: `cannot find function volume in this scope`.

- [ ] **Step 3: Implement `sigma::volume`**

Insert into `crates/rfl-core/src/sigma.rs` after the `path` fn (before `#[cfg(test)]`):

```rust
/// Generate the `volume` sweep set Σ (`spec/02` Appendix A, Region kinds beyond
/// surface): an axis-aligned box `[0,U]×[0,V]×[0,W]` covered as a deterministic
/// stack of surface layers at depth intervals `s_v` along the region frame `+z`
/// (the v0 principal axis). Each layer is a `raster` over `[0,U]×[0,V]` shifted
/// to its depth; odd layers are reversed so Σ stays a continuous path
/// (boustrophedon stacking). Same units / FOV inputs as `raster`.
#[must_use]
pub fn volume(
    size_u: f64,
    size_v: f64,
    size_w: f64,
    standoff: f64,
    coverage_overlap: f64,
    h_angle_rad: f64,
    v_angle_rad: f64,
) -> Vec<Pose6D> {
    let f_v = 2.0 * standoff * (v_angle_rad / 2.0).tan();
    let s_v = f_v * (1.0 - coverage_overlap);
    // Degenerate guard: a flat v-FOV gives no depth advance — a single layer.
    let n_w = if s_v > 0.0 {
        (size_w / s_v).ceil().max(1.0) as usize
    } else {
        1
    };

    let mut poses = Vec::new();
    for k in 0..n_w {
        let w_k = (k as f64 + 0.5) * size_w / n_w as f64;
        let mut layer = raster(size_u, size_v, standoff, coverage_overlap, h_angle_rad, v_angle_rad);
        for p in &mut layer {
            p.position[2] += w_k; // surface at depth w_k, sensor standoff above it
        }
        if k % 2 == 1 {
            layer.reverse(); // boustrophedon: continuous Σ across layers
        }
        poses.extend(layer);
    }
    poses
}
```

- [ ] **Step 4: Run to verify the unit tests pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p rfl-core sigma::tests::volume 2>&1 | grep "test result"`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Add the `ScanRegion::Volume` variant + parse test**

In `crates/rfl-core/src/region.rs`, add to the `ScanRegion` enum (after `Path { .. }`, before `Waypoints`):

```rust
    /// An axis-aligned box `[0,size_u]×[0,size_v]×[0,size_w]` in `frame`, covered
    /// as a stack of surface layers along the frame `+z` axis at depth intervals
    /// `s_v` (`spec/02` Appendix A, Region kinds beyond surface).
    Volume {
        /// The region reference frame.
        frame: FrameRef,
        /// Extent along the frame x-axis (the u direction).
        size_u: Quantity,
        /// Extent along the frame y-axis (the v direction).
        size_v: Quantity,
        /// Extent along the frame z-axis (the stacking/depth direction).
        size_w: Quantity,
    },
```

Add to the `tests` module in `region.rs`:

```rust
    #[test]
    fn parses_volume_region() {
        let yaml = "kind: volume\nframe: panel\nsize_u: 200 mm\nsize_v: 150 mm\nsize_w: 200 mm\n";
        let r: ScanRegion = serde_yaml::from_str(yaml).unwrap();
        let ScanRegion::Volume {
            frame,
            size_u,
            size_v,
            size_w,
        } = r
        else {
            panic!("expected volume")
        };
        assert_eq!(frame, "panel");
        assert_eq!(size_u.0, "200 mm");
        assert_eq!(size_v.0, "150 mm");
        assert_eq!(size_w.0, "200 mm");
    }
```

- [ ] **Step 6: Add the `Volume` dispatch arm**

In `crates/rfl-core/src/translation.rs` `lower_reach_scan` `match &p.region`, add after the `Path` arm:

```rust
        // Volume region: a boustrophedon stack of surface-raster layers at depth
        // intervals s_v along frame +z (spec/02 Appendix A). FOV sets both the
        // in-plane grid and the layer spacing.
        crate::region::ScanRegion::Volume {
            size_u,
            size_v,
            size_w,
            ..
        } => {
            let (h, v) = e.sensor_fov(&sensor_frame).map_or((0.0, 0.0), |f| {
                (angle_rad(&f.h_angle), angle_rad(&f.v_angle))
            });
            let (u, vv, ww) = (length_m(size_u), length_m(size_v), length_m(size_w));
            crate::sigma::volume(u, vv, ww, standoff, overlap, h, v)
        }
```

- [ ] **Step 7: Verify the workspace compiles + all existing tests pass**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace --all-targets 2>&1 | grep -c "test result: FAILED"`
Expected: `0`.

- [ ] **Step 8: Create `examples/02-surface-scan/skill-volume.yaml`**

```yaml
# Example 02 — Surface scan (volume variant)
# Cover a volumetric region (e.g. a transparent or layered part) as a stack of
# planar surface sweeps at successive depths. Demonstrates the volume generator
# (spec/02 Appendix A, Region kinds beyond surface): layers along the frame +z
# axis at depth intervals s_v, each an in-plane raster, so both the layer count
# and the per-layer grid follow each embodiment's sensor field of view.
skill: surface-scan
description: >
  Volume-scan a layered part by sweeping planar raster layers at successive
  depths along the part axis, then inspect the covered region for defects.

body:
  sequence:
    - reach.scan:
        region:
          kind: volume
          frame: panel
          size_u: 200 mm
          size_v: 150 mm
          size_w: 200 mm
        standoff: 100 mm
        pattern: raster
        coverage_overlap: 0.2

    - sense.inspect:
        target: panel
        observe: [defect]
```

- [ ] **Step 9: Create the golden test `crates/rfl-conformance/tests/surface_scan_volume.rs`**

Identical structure to `surface_scan_path.rs` (Task 1 Step 9), with these substitutions: skill file `skill-volume.yaml`; snapshot labels `volume_allegro` / `volume_leap` / `volume_pneumatic`; test fn names `golden_volume_*` / `volume_generation_is_byte_identical` / `volume_every_line_is_a_valid_execute_message`; the module doc references the volume variant and "a boustrophedon stack of surface layers (spec/02 Appendix A)". Full file:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Conformance test class 2 for the surface-scan volume variant: a `volume`
//! region compiles into a boustrophedon stack of surface-raster layers along
//! frame +z (spec/02 Appendix A, Region kinds beyond surface). Byte-
//! deterministic, matches a committed golden, and every line is a valid
//! driver-interface execute message.

use rfl_conformance::retarget_example_to_jsonl;
use std::path::{Path, PathBuf};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/02-surface-scan")
}

fn jsonl_for(stem: &str) -> String {
    let dir = example_dir();
    retarget_example_to_jsonl(
        &dir.join("skill-volume.yaml"),
        &dir.join(format!("embodiments/{stem}.yaml")),
    )
    .expect("retarget")
}

#[test]
fn golden_volume_allegro() {
    insta::assert_snapshot!("volume_allegro", jsonl_for("allegro"));
}

#[test]
fn golden_volume_leap() {
    insta::assert_snapshot!("volume_leap", jsonl_for("leap"));
}

#[test]
fn golden_volume_pneumatic() {
    insta::assert_snapshot!("volume_pneumatic", jsonl_for("pneumatic-6f"));
}

#[test]
fn volume_generation_is_byte_identical() {
    for stem in ["allegro", "leap", "pneumatic-6f"] {
        assert_eq!(
            jsonl_for(stem),
            jsonl_for(stem),
            "non-deterministic for {stem}"
        );
    }
}

#[test]
fn volume_every_line_is_a_valid_execute_message() {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/driver-interface.schema.json");
    let schema: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&schema_path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("driver-interface.schema.json", schema)
        .unwrap();
    let idx = compiler
        .compile("driver-interface.schema.json", &mut schemas)
        .unwrap();

    for stem in ["allegro", "leap", "pneumatic-6f"] {
        for line in jsonl_for(stem).lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            schemas
                .validate(&v, idx)
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
        }
    }
}
```

- [ ] **Step 10: Generate the goldens + verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && INSTA_UPDATE=always cargo test -p rfl-conformance --test surface_scan_volume 2>&1 | grep "test result"`
Then: `find crates/rfl-conformance/tests/snapshots -name '*.snap.new' -delete`
Re-run without update: `cargo test -p rfl-conformance --test surface_scan_volume 2>&1 | grep "test result"`
Expected: `test result: ok. 5 passed`. The allegro scan line's `poses` length is 36; leap 12; pneumatic 18.

- [ ] **Step 11: Run the full gate suite** (same commands as Task 1 Step 11). Expected: 0 failures, fmt/clippy/validate clean.

- [ ] **Step 12: Commit (one commit)**

```bash
git diff --cached --name-only | grep -E '^whitepaper/|^spec/02|bindings/python|friend-review' && echo ABORT || echo ok
git add crates/rfl-core/src/sigma.rs crates/rfl-core/src/region.rs crates/rfl-core/src/translation.rs \
  examples/02-surface-scan/skill-volume.yaml crates/rfl-conformance/tests/surface_scan_volume.rs \
  crates/rfl-conformance/tests/snapshots/surface_scan_volume__volume_allegro.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_volume__volume_leap.snap \
  crates/rfl-conformance/tests/snapshots/surface_scan_volume__volume_pneumatic.snap
git commit -m "$(cat <<'EOF'
feat(sigma): implement the Σ volume sweep-pattern generator (spec/02 App A)

A `volume` region is covered as a deterministic stack of surface layers at depth
intervals s_v along the region frame +z axis, each layer a surface sweep (spec/02
Appendix A, Region kinds beyond surface) — the last deferred Σ region kind:

- ScanRegion::Volume { frame, size_u, size_v, size_w } (an axis-aligned box).
- sigma::volume: n_w = ceil(W / s_v) centred layers, each a raster over
  [0,U]×[0,V] shifted to its depth w_k; odd layers reversed so Σ stays a
  continuous path (boustrophedon stacking); flat v-FOV degenerates to a single
  layer.
- lower_reach_scan dispatches a Volume region to sigma::volume.
- examples/02-surface-scan/skill-volume.yaml + per-embodiment goldens
  (surface_scan_volume) + sigma/region unit tests pinning the layer/grid count,
  the in-box property, boustrophedon continuity, and the flat-FOV guard.

All four normative Σ region kinds (surface / arc / waypoints implied; path /
volume) are now implemented. Orthogonal pattern field stays raster.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 13: Push ff-safe + watch CI** (same as Task 1 Step 13). Expected: all 8 jobs green.

---

## Post-completion

- [ ] Append a concise milestone to `~/.claude/projects/.../memory/project_rfl.md` `## Implementation track` (real hashes + test deltas) and refresh the MEMORY.md RFL hook if needed.
- [ ] Confirm the committed `schemas/epsilon-tolerances.yaml` is untouched (this work adds no ε data).

## Self-review notes

- **Spec coverage:** path (Task 1) + volume (Task 2) = the two deferred region kinds in Appendix A § "Region kinds beyond surface". All three design decisions (frame +z axis, flat-normal orientation, boustrophedon continuity) are realized in `sigma::volume` / `sigma::path`. ✔
- **Placeholders:** none — every step has concrete code/commands. The one prose reference (Task 2 Step 9) is immediately followed by the full file. ✔
- **Type consistency:** `path(&[[f64;3]], f64, f64, f64)`, `volume(f64×7)`, `ScanRegion::Path{frame,points}`, `ScanRegion::Volume{frame,size_u,size_v,size_w}` — used identically in sigma, region, and the translation dispatch arms. `length_m`/`angle_rad`/`e.sensor_fov` match the existing arc arm. ✔
