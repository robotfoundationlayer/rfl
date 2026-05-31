# Design: `surface-scan` worked example and the Σ sweep generators

Status: approved design, pre-implementation (2026-05-31)

This is the second reference-implementation increment. It adds a structurally
different worked example, a surface scan, whose retargeting exercises the
**generative** path of the Translation Layer: `reach.scan` compiles a region into
an ordered set of sensor poses (the sweep set Σ, `spec/02` Appendix A) rather than
a single goal pose. It is the first increment that emits **numeric** poses, so it
also establishes deterministic floating-point serialization. The specification
under `spec/` is authoritative; this document describes how the reference
implementation realizes it.

## 1. Why this increment

The cable-insertion increment (v0) lowered every primitive as `params -> one
canonical action` and carried all quantities as strings, so it never emitted a
float. `reach.scan` is qualitatively different: its Σ is a deterministic function
`(region, pattern, standoff, coverage_overlap, fov) -> [sensor pose...]` that
**generates** a path. Demonstrating it proves the engine is not specific to one
skill shape, and it forces two new pieces:

1. the Σ geometry (transcribed from `spec/02` Appendix A), and
2. **deterministic floating-point serialization** (the numeric counterpart of the
   string-quantity determinism lever from v0).

The same scan skill retargets onto the three existing hands, and because their
sensors declare different fields of view (allegro `palm_cam` 60x45, leap
`wrist_cam` 70x55, pneumatic `head_cam` 65x50), each produces a **different Σ**
(footprint, pass spacing, pose count). That divergence, from one skill and three
descriptors, is the generality demonstration.

## 2. Scope

In scope:

- A new worked example `examples/02-surface-scan/`: a static planar surface region
  scanned by `reach.scan` (raster), then `sense.inspect`.
- Two Σ patterns: **raster** (serpentine surface coverage, the default and the
  meaningful generative case) and **waypoints** (the degenerate passthrough).
- Numeric Σ pose generation with deterministic float serialization.
- Lowering for `reach.scan` and `sense.inspect`; conformance test class 2 extended
  with the scan example.

Deferred (consistent with v0 deferrals and YAGNI):

- The `spiral` and `arc` Σ patterns (Archimedean spiral, swept arc): same Σ
  framework, a clean follow-up once raster and waypoints are proven.
- `path` and `volume` `ScanRegion` kinds (`spec/02` Appendix A reduces both to
  repeated surface coverage): v0 does the `surface` kind only.
- `coverage_overlap: auto` derivation from FOV: v0 requires an explicit overlap in
  the example (the `auto` rule is a later increment).
- Everything still deferred from v0 (mass-dependent grasp-force, routing,
  time-scaling, multi-embodiment, conformance class 3/4).

## 3. Architecture

New and extended units in `rfl-core` (the existing module layout is extended, not
restructured):

```
rfl-core
  region.rs       [new]  ScanRegion input model:
                         Surface { frame, size_u, size_v }  (the v0 surface rect)
                         | Waypoints { frame, poses: [Pose6D] }
  sigma.rs        [new]  the Σ generators (pure functions, nalgebra geometry,
                         transcribed from spec/02 Appendix A):
                         raster(surface, standoff, coverage_overlap, fov) -> [Pose6D]
                         waypoints(wps) -> [Pose6D]
  canonical.rs    [extend]  PoseExpr::SweepPath { pattern, poses: [Pose6D] }
                            + deterministic float serialization (6-decimal round)
  skill_isa.rs    [extend]  ReachScan + SenseInspect param structs (v0 subset)
  embodiment.rs   [extend]  typed sensor descriptor: sensors[frame].fov{h_angle,v_angle}
  translation.rs  [extend]  lower_reach_scan (calls sigma) + lower_sense_inspect;
                            gate: reach.scan = baseline (no key), sense.inspect = "sense.inspect"
```

New example and tests:

```
examples/02-surface-scan/
  skill.yaml                 reach.scan(static surface region, raster) -> sense.inspect
  embodiments/{allegro,leap,pneumatic-6f}.yaml   self-contained copies + sense.inspect capability (decision B1)
  run.py                     same entry shape as example 01
crates/rfl-conformance/tests/
  surface_scan.rs            3 insta golden + generate-twice + per-line execute-schema validation
```

### Decision A: `reach.scan` emits one action carrying Σ

`reach.scan` lowers to a **single** canonical action whose `target_pose` is the new
`PoseExpr::SweepPath { pattern, poses }`. `spec/02` calls Σ "the scan path", so the
sweep is one logical action the driver executes as a unit. This extends the
existing `PoseExpr` enum and keeps the `CanonicalAction` tuple shape unchanged. The
alternative (one canonical action per Σ station) was rejected: it bloats the stream
and loses the scan grouping.

### Decision B: self-contained example descriptors (B1)

`examples/02-surface-scan/embodiments/` holds its own copies of the three hand
descriptors, each with `sense.inspect` added to `capabilities.skills` and reusing
the existing camera sensor blocks. This keeps the example self-contained and does
not touch example 01 or its committed goldens. The cost is descriptor duplication;
a shared `examples/embodiments/` directory is noted as a future cleanup but is out
of scope here.

### Decision C: deterministic float serialization

Σ produces `f64` positions and orientations. For byte-identical generation (RD1c),
each coordinate is **rounded to 6 decimals** (`(x * 1e6).round() / 1e6`) before
serialization, then emitted as a JSON number (serde_json's shortest round-trip
repr of the rounded value is deterministic). Six decimals is sub-micron in metres,
ample for poses. On one platform (the golden's CI) generation is exactly
reproducible; the rounding absorbs the last-ULP differences a transcendental
(`tan`, the orientation quaternion) can produce across platforms.

## 4. The Σ surface raster (transcribed from `spec/02` Appendix A)

For a `Surface { frame, size_u = U, size_v = V }` region, sensor `fov = {h_angle,
v_angle}`, and `standoff`:

- footprint `f_u = 2 * standoff * tan(h_angle / 2)`, `f_v = 2 * standoff * tan(v_angle / 2)`;
- pass spacing `s_u = f_u * (1 - coverage_overlap)`, `s_v = f_v * (1 - coverage_overlap)`;
- passes `n_v = ceil(V / s_v)`, `v_k = (k + 0.5) * V / n_v` for `k = 0 .. n_v - 1`;
- stations `n_u = ceil(U / s_u)`, `u_j = (j + 0.5) * U / n_u` for `j = 0 .. n_u - 1`;
- `Σ` is the serpentine (boustrophedon) order: `j` ascending on even `k`, descending
  on odd `k`, so the path is continuous.

A station `(u, v)` maps to a sensor pose expressed in the region frame: position
`= (u, v, standoff)` (the surface is the frame's xy-plane, the outward normal is
`+z`, the sensor stands off along `+z`); orientation is the fixed quaternion that
points the sensor `bore_axis` anti-parallel to the normal with its secondary axis
along `+u`. For an axis-aligned surface the orientation is constant across
stations; only the position varies. `waypoints` ignores `standoff`/`overlap`/`fov`
and returns the caller's poses verbatim.

## 5. Lowering

- `reach.scan`: `reach.*` is the unkeyed baseline, so there is no capability gate;
  the lowering reads `fov` from `embodiment.sensors[sensor_frame]` (default
  `role_defaults.sensor`), `standoff`/`region`/`pattern`/`coverage_overlap` from the
  skill, calls the Σ generator, and emits one `CanonicalAction` with
  `target_frame = sensor_frame` and `target_pose = SweepPath`. Motion bounds are
  clamped by the existing `base_envelope`.
- `sense.inspect`: gated by the `sense.inspect` capability key; lowers to a
  perception action (like `sense.locate`), binding any `let` variable, with the
  target as a symbolic reference.

## 6. Determinism and testing

- `sigma` unit tests: a known `(surface, fov, standoff, overlap)` yields a known
  pose count and known rounded coordinates, hand-checked against the Appendix A
  formulas; waypoints round-trips its input.
- `canonical` float-emit test: a `SweepPath` with crafted coordinates serializes to
  the expected rounded numbers, twice-identically.
- `rfl-conformance` `surface_scan.rs`: three insta golden snapshots (one per hand,
  showing the per-FOV Σ divergence), a generate-twice byte-equality check, and
  per-line validation against the driver-interface `execute` schema (the
  `SweepPath` canonical action passes the open `canonical_action` floor).
- The example 01 cable goldens and the Python `validate.py` (C1 through C7) stay
  unchanged and green.

## 7. Sections to transcribe during implementation

- `spec/02` Appendix A (the per-pattern Σ construction) and `spec/02` § Trajectory
  generation and timing (TG1c).
- `schemas/skill-isa.schema.json` `$defs/ReachScanParams` and `$defs/SenseInspectParams`
  (the precise parameter sets; already read for this design).
- `spec/03` § Sensor descriptor (the `fov` `{h_angle, v_angle}` shape and the
  `bore_axis` / `working_range` fields).
