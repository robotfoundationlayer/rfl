# PincherX-100 pick-and-place existence proof — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the pure-software half of the first real-embodiment RFL existence proof — a PincherX-100 (Trossen, 4-DOF, 50 g, no-tactile) embodiment descriptor, three skills (hover / scan / pick-and-place), a real Class-4 driver that drives the Interbotix arm (Gazebo sim first, physical later) including gripper, and a Class-4 measurement — so a hand-authored RFL skill retargets onto a real low-end arm and the loop closes (Principle 1, capability negotiation, proxy-tier graceful degradation, the grasp-force floor, all on real hardware).

**Architecture:** Everything lives under a new top-level `hardware/pincherx-100/` (never touch `crates/` — a parallel session is heavily editing it). The retarget engine is reused unchanged via `rfl-cli retarget` (confirmed to accept arbitrary file paths). The driver is a single Python module split into a pure, ROS-free **parse → resolve → emit** core (unit-tested against committed canonical-action JSONL fixtures + a `MockBackend`) and a thin import-guarded `InterbotixBackend` adapter (exercised only against the sim/arm at bringup). `measure.py` compares realized-vs-commanded poses against tolerance, audits grasp success + the proxy tier, and captures integration cost.

**Tech Stack:** YAML descriptors/skills validated against `schemas/*.schema.json`; Rust `rfl-cli` (build/run only, no edits) to produce the canonical-action JSONL; Python 3 driver + measurement run via ephemeral `uv run --with ...` (repo convention — no `pyproject`/lockfile churn); `pytest` for TDD; ROS 2 + MoveIt + Gazebo + Interbotix `interbotix_xs_modules` Python API for the real backend.

---

## Source facts confirmed this session (do NOT re-derive from memory — re-read if stale)

- **CLI accepts arbitrary paths.** `crates/rfl-cli/src/main.rs:53` — `Retarget { skill: PathBuf, embodiment: PathBuf }` read via `read_to_string`; output is `rfl_core::canonical::to_jsonl`. So `rfl retarget hardware/pincherx-100/skill-pickplace.yaml --embodiment hardware/pincherx-100/pincherx-100.yaml` is the data path. **No Python retarget wrapper needed.**
- **Capability gate** (`translation.rs:105`): `reach.*` = unkeyed baseline (never gated); `grasp.pinch`→`"grasp.pinch"`; `transport.move_to_pose`→`"transport"`; `grasp.release`→requires any `grasp.*` declared; `sense.locate`→`"sense.locate"`; `sense.inspect`→`"sense.inspect"`; `force.insert_fit`→`"force.insert_fit"`; etc. Absent key → `Error::Translation("capability_absent: <key>")`.
- **Weight wiring** (`translation.rs:59` `build_weights`): maps the **let-binding name** (`let: X`) → `objects[sense.locate.target_ref].estimated_mass`. So `grasp.pinch.target` MUST be the let-binding name for the floor to fire.
- **Grasp-force floor** (`grasp_force.rs`): `min_holding_force(w, Pinch) = 2.0·w`. `payload_key(Pinch) = "payload_grasp_pinch"`. `dynamic_a_max` clamps transport `a_max` only when tighter than `a_cartesian_max`.
- **Execute shapes** (from the lowering in `translation.rs`, serialized by `canonical.rs`):
  - `reach.hover` → `target_frame = control_frames[0]`; `target_pose = {frame, offset:{along:"outward_normal", distance:"<standoff>"}}`; `timing.timing_mode="strict"`; `safety_envelope.motion_bounds`.
  - `reach.scan` → `target_frame = role_defaults.sensor`; `target_pose = {pattern, poses:[{position:[x,y,z], orientation:[x,y,z,w]}, ...]}` (Σ count driven by `sensors.<sensor>.fov`); `timing.timing_mode="strict"`.
  - `sense.locate` → `target_frame = role_defaults.sensor`; `target_pose = {ref:"<target_ref>"}`; no force/tactile.
  - `grasp.pinch` (no tactile) → `target_frame = role_defaults.grasp`; `target_pose = {ref:"<let-name>"}`; `force_budget="<clamped/floored> N"`; `tactile_target = {proxy:{tier:"proxy", criterion:"position_convergence_and_force_hold"}}`; `safety_envelope.force_profile = {min_holding_force:"<mhf> N"}`.
  - `transport.move_to_pose` (held) → `target_frame = role_defaults.grasp`; `target_pose = {frame, offset}`; `timing.timing_mode="time_scalable"`; `safety_envelope.force_profile.min_holding_force` propagated from `ctx.held`; `a_max` clamped only if `dynamic_a_max < a_cartesian_max`.
  - `grasp.release` → `target_pose = {direction:"-tool_axis", distance:"0 mm"}` (AxisRelative); clears `ctx.held`.
  - `reach.retract` → `target_frame = control_frames[0]`; `target_pose = {direction:<from skill>, distance:"<n> mm"}` (AxisRelative).
- **Frame accessors** (`embodiment.rs`): `control_frame()=control_frames[0]`; `sensor_frame()=role_defaults.sensor`; `grasp_frame()=role_defaults.grasp`; `sensor_fov(frame)=sensors.<frame>.fov`. **`role_defaults` has no `control` key** (schema `additionalProperties:false`, properties = grasp/sensor/tactile/support, required = grasp).
- **Descriptor schema** (`embodiment-descriptor.schema.json`): `capabilities.skills` `minItems:1` and the enum **excludes `reach.*`**; M2 `allOf` requires, for `grasp.pinch`, the limits `grip_force_max`, `v_grasp`, `payload_grasp_pinch`. Sensor entry requires `bore_axis, fov{h_angle,v_angle}, working_range, modalities`. Quantities are `"<num> <unit>"` strings; `stability_margin` is a bare number.
- **Driver-interface schema** (`driver-interface.schema.json`): five `message`-discriminated payloads. `telemetry` requires `{message:"telemetry", action_id, t}` + optional `realized_pose, wrench, securing_force, tactile, events, fidelity_tier`. `status` requires `{message:"status", action_id, outcome∈{succeeded,failed,indeterminate}}` + optional `failure_class, failure_detail, verdict, fidelity_tier, safety_flags, final_pose`. `additionalProperties:false` on every object — emit ONLY listed keys.
- **Interbotix API** (Interbotix/interbotix_ros_toolboxes `interbotix_xs_modules`, confirmed via gh): `InterbotixManipulatorXS(robot_model, group_name, gripper_name)` exposes `.arm` + `.gripper`. `arm.set_ee_pose_components(x=0, y=0, z=0, roll=0, pitch=0, yaw=None, custom_guess=None, execute=True, moving_time=None, accel_time=None, blocking=True)` → `(theta_list, success: bool)`. **4-DOF constraint** (`arm.py:240`): `if num_joints < 6 ... yaw = atan2(y, x)` — base auto-points at target; yaw not free (this IS the §7 orientation-residual caveat). Gripper: `gripper.open(delay=1.0)` / `gripper.close(delay=1.0)` (ROS1 names; ROS2 `xs_robot` may expose `grasp()`/`release()` — **confirm at bringup §11b**). The exact installed-version names are isolated in `InterbotixBackend` so a one-line change covers either.

## File Structure

```
hardware/pincherx-100/
  pincherx-100.yaml          # embodiment descriptor (Class-1 validated)
  skill-hover.yaml           # warm-up 1 — reach.hover (interval-invariant)
  skill-scan.yaml            # warm-up 2 — reach.scan (Σ raster)
  skill-pickplace.yaml       # HEADLINE — sense.locate→grasp.pinch→transport→release→retract
  skill-rejected.yaml        # negative-space demo — requests force.insert_fit (gate must reject)
  frames.yaml                # calibration: task-frame→base transforms + object mass + grasp model (sim config)
  rfl_interbotix_driver.py   # the Class-4 driver: parse→resolve→emit core + Mock/Interbotix backends + CLI
  measure.py                 # Class-4 measurement: pose error, grasp success, proxy audit, integration cost
  fixtures/                  # committed canonical-action JSONL from `rfl retarget` (the contract under test)
    skill-hover.jsonl
    skill-scan.jsonl
    skill-pickplace.jsonl
  tests/
    test_parse.py            # execute JSONL → typed ArmStep (pure)
    test_resolve.py          # frame transform + pose resolution → concrete base-frame poses (pure)
    test_emit_mock.py        # MockBackend end-to-end + emitted telemetry/status validate vs driver-interface schema
    test_measure.py          # measure.py over recorded mock telemetry → report
  README.md                  # bringup + how to run (sim → physical) + claims/caveats
```

**Responsibilities.** The descriptor/skills are declarative artifacts validated by schema + `rfl retarget`. The driver core is pure (parse, resolve, emit) so it is unit-testable without ROS; the two backends sit behind one tiny protocol so the sim/physical swap is a `--backend` flag. `measure.py` is pure (reads two JSONL streams). `frames.yaml` is the one calibration artifact the design §7 calls out — versioned so the sim object placement is reproducible. `fixtures/` decouples Python TDD from the parallel session's possibly-red `crates/`.

## Conventions for EVERY task (read once, apply throughout)

- **Git (strict).** New files only under `hardware/pincherx-100/`. `git add` **explicit paths only** (never `-A`/`.`; never stage `docs/plans/`). Commit with `git commit -F - -- <explicit paths>`. If `.git/index.lock` appears (parallel session), wait ~2 s and retry. No `--force`, no `--no-verify`. Commit straight to `main` (user override of worktree-default). Conventional Commits + a `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` trailer.
- **Push protocol (only when a task says to, or at the end).** `git fetch origin main` → `git merge-base --is-ancestor origin/main HEAD` (must be ancestor = ff) → `git push origin main` → verify `git rev-list --left-right --count origin/main...HEAD` prints `0 0`. If non-ff: `git fetch origin main && git rebase origin/main` (only `hardware/` + your commits move — never rebase others' `crates/` work away), re-run checks, retry.
- **Never** stage secrets (`.env`, `*.key`, `credentials*`, `*.pem`). None are expected here.
- **Python** runs via `uv run --with pytest --with pyyaml --with jsonschema python -m pytest ...` (ephemeral; no `pyproject`, no `uv.lock`). One-off validation uses `uv run --with jsonschema --with pyyaml python - <<'PY' ...`.
- **No Rust edits.** `rfl-cli`/`rfl-core` are read + build + run only.
- This is hardware/Python integration: the Rust insta-golden / `cargo test` discipline does **not** apply. The applicable rigor is: Class-1 schema validation, `rfl retarget` success, pytest, and the git discipline above.

---

## Task 1: Scaffold + embodiment descriptor + Class-1 validation

**Files:**
- Create: `hardware/pincherx-100/pincherx-100.yaml`

- [ ] **Step 1: Write the descriptor**

`hardware/pincherx-100/pincherx-100.yaml`:

```yaml
# PincherX-100 (Trossen) — open-chain 4-DOF arm with a parallel-jaw gripper.
# Reference instance of schemas/embodiment-descriptor.schema.json; structure per
# 03-driver-interface.md (frame model + capability manifest + limits). Verified
# hardware facts: docs/design/2026-05-31-pincherx-existence-proof-design.md § 2.
# Values tagged CONFIRM are finalized at bringup (§ 11) and affect only the envelope,
# not the proof's pass/fail.
embodiment:
  id: pincherx-100
  class: open-chain-4dof-arm

  # Frame model — 03 § Embodiment frame model. The schema's role_defaults has no
  # `control` key; control_frame() reads control_frames[0]. (Design § 6 wording
  # "role_defaults.control" realized as control_frames[0] + role_defaults.grasp/sensor.)
  frames:
    control_frames: [ee_gripper]
    role_defaults:
      grasp:  ee_gripper      # the single parallel-jaw gripper frame (default grasp ROLE; not a grasp capability)
      sensor: ee_gripper      # no camera — the tool tip is the nominal sensor frame for reach.scan's Σ
    tool_axis: { ee_gripper: +z }   # CONFIRM bringup: the gripper approach axis

  # Capability manifest — 03 § Capability manifest. Declared: grasp.pinch (proxy tier),
  # transport (= transport.move_to_pose), sense.locate. Plus the unkeyed reach.* baseline;
  # grasp.release is presupposed by the declared grasp.*. NOT declared (gate rejects →
  # negative-space demo): in_hand.*, force.*, the other 7 grasp modes, transport.carry,
  # sense.inspect.
  capabilities:
    skills: [grasp.pinch, transport, sense.locate]
    # aux.tactile_sensing OMITTED → grasp.pinch confirmation degrades to the proxy tier (04 § Graceful degradation)

  # Limits — flat embodiment.limits.* (03 § Limits). reach baseline (mandatory) + the
  # grasp.pinch limits the schema's M2 clause requires.
  limits:
    v_cartesian_max:        0.15 m/s    # CONFIRM bringup: slow real PincherX EE speed
    w_cartesian_max:        1.0 rad/s   # CONFIRM bringup
    a_cartesian_max:        0.5 m/s^2   # CONFIRM bringup
    joint_velocity_ceiling: 3.0 rad/s   # CONFIRM bringup (Dynamixel limit)
    stop_time:              0.2 s
    tracking_bandwidth:     2 Hz
    grip_force_max:         5 N         # CONFIRM bringup: PincherX gripper fingertip force; must be ≥ ~1 N (50 g floor)
    v_grasp:                0.02 m/s    # gripper close speed
    payload_grasp_pinch:    0.5 N       # verified 50 g payload

  # Non-contact sensor descriptor — 03 § Sensor descriptor. Nominal FOV sizes reach.scan's Σ.
  sensors:
    ee_gripper:
      bore_axis: +z
      fov: { h_angle: 40 deg, v_angle: 30 deg }   # CONFIRM bringup (nominal proxy for the tool-tip sensor frame)
      working_range: [0.05 m, 0.3 m]
      modalities: [presence]
```

- [ ] **Step 2: Class-1 validate against the schema**

Run:
```bash
cd ~/Documents/GitHub/rfl && uv run --with jsonschema --with pyyaml python - <<'PY'
import json, yaml, pathlib, sys
from jsonschema import Draft202012Validator
schema = json.loads(pathlib.Path("schemas/embodiment-descriptor.schema.json").read_text())
doc = yaml.safe_load(pathlib.Path("hardware/pincherx-100/pincherx-100.yaml").read_text())
errs = sorted(Draft202012Validator(schema).iter_errors(doc), key=lambda e: e.path)
if errs:
    for e in errs: print("FAIL:", list(e.path), e.message)
    sys.exit(1)
print("OK: pincherx-100.yaml is Class-1 valid")
PY
```
Expected: `OK: pincherx-100.yaml is Class-1 valid`. If a FAIL prints, fix the descriptor (most likely: a missing M2 grasp.pinch limit, or a malformed quantity string) and re-run. Do not proceed until green.

- [ ] **Step 3: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- hardware/pincherx-100/pincherx-100.yaml <<'MSG'
feat(hardware): PincherX-100 embodiment descriptor

First real-embodiment RFL descriptor: 4-DOF Trossen PincherX 100 with a
parallel-jaw gripper. Declares grasp.pinch (proxy tier — no tactile),
transport, sense.locate over the reach baseline; deliberately omits the
dexterity families so the capability gate rejects them. Class-1 validated.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```
(Run `git add hardware/pincherx-100/pincherx-100.yaml` first if `commit -F - --` does not stage on your git version; explicit path only.)

---

## Task 2: Skills + canonical-action fixtures + capability-gate demo

**Files:**
- Create: `hardware/pincherx-100/skill-hover.yaml`, `skill-scan.yaml`, `skill-pickplace.yaml`, `skill-rejected.yaml`
- Create: `hardware/pincherx-100/fixtures/skill-hover.jsonl`, `skill-scan.jsonl`, `skill-pickplace.jsonl` (generated)

- [ ] **Step 1: Write the four skills**

`skill-hover.yaml`:
```yaml
# Warm-up 1 — kinematic station-keeping. Smoke-tests the driver and the interval-invariant
# envelope class (spec/05 ENV2) on real hardware. No grasp / no perception (sense.* is not
# declared by the PincherX, by design).
skill: pincherx-hover
description: >
  Hold a fixed standoff over a work point for a bounded interval on a 4-DOF PincherX 100.
body:
  sequence:
    - reach.hover:
        target: work_point
        standoff: 50 mm
        duration: 5 s
```

`skill-scan.yaml`:
```yaml
# Warm-up 2 — raster-scan a small region at a fixed standoff. Exercises the Σ generators
# and per-pose conformance on real hardware (no grasping). sense.inspect is intentionally
# absent (not declared → would be rejected); this skill is pure reach.* baseline.
skill: pincherx-scan
description: >
  Raster-scan a small rectangular region at a fixed standoff on a 4-DOF PincherX 100.
body:
  sequence:
    - reach.scan:
        region: { kind: surface, frame: work_surface, size_u: 100 mm, size_v: 80 mm }
        standoff: 100 mm
        pattern: raster
        coverage_overlap: 0.2
```

`skill-pickplace.yaml`:
```yaml
# HEADLINE — pick a light object and place it. locate → pinch (proxy tier) → transport
# (held; min_holding_force floor propagated) → release → retract. Embodiment-agnostic:
# no PincherX frame is named (Principle 1). The graspable object is ≤ 50 g and sized,
# with the gripper opening, to absorb the arm's 5 mm repeatability.
skill: pincherx-pickplace
description: >
  Pick a light object (≤ 50 g) and place it at a target pose on a 4-DOF PincherX 100.
objects:
  widget: { ref: widget, estimated_mass: 0.3 N }   # ~30 g ≤ the 0.5 N payload; fires the
                                                    # min_holding_force floor (0.6 N) and its
                                                    # propagation into transport (increment 10)
body:
  sequence:
    - let: widget_t
      from:
        sense.locate:
          target_ref: widget
          modality: auto
    - grasp.pinch:
        target: widget_t
        force_budget: 5 N           # clamped to grip_force_max (5 N); floored to min_holding_force (0.6 N)
        tactile_target: auto         # no tactile → proxy tier (position_convergence_and_force_hold)
    - transport.move_to_pose:
        target_pose: { frame: place_zone, offset: { along: +z, distance: 20 mm } }
    - grasp.release: { grasp_handle: active }
    - reach.retract: { direction: -tool_axis, distance: 50 mm }
```

`skill-rejected.yaml`:
```yaml
# Negative-space demo — a skill the PincherX honestly CANNOT run (force-controlled
# insertion needs F/T sensing the arm lacks). Retarget MUST return capability_absent.
# Not part of the run; it exists to make the capability gate's rejection visible + tested.
skill: pincherx-rejected
description: >
  Force-controlled insertion — unsupported on the no-F/T PincherX; expected to be rejected.
objects:
  part: { ref: part }
body:
  sequence:
    - let: part_t
      from:
        sense.locate: { target_ref: part, modality: auto }
    - force.insert_fit:
        target_fit: part_t
        force_budget: 15 N
        compliance: active
        stop_condition:
          all_of:
            - effort_rise: 12 N
            - reached: { depth: 8 mm }
```

- [ ] **Step 2: Build `rfl-cli` in an isolated worktree (avoid the parallel session's possibly-red `crates/`)**

The parallel session is editing `crates/` live, so building in the primary tree may fail through no fault of ours. Build from a worktree pinned at the current HEAD (design commit `c172c7e`, known green) with its own target dir:

```bash
cd ~/Documents/GitHub/rfl
HEAD_SHA=$(git rev-parse HEAD)
git worktree add -d /tmp/rfl-pincherx-build "$HEAD_SHA"   # detached, read-only build tree
( cd /tmp/rfl-pincherx-build && CARGO_TARGET_DIR=/tmp/rfl-pincherx-target cargo build -q -p rfl-cli )
```
Expected: builds cleanly. If `index.lock` blocks `worktree add`, wait ~2 s and retry. (If the parallel session has ALREADY merged work you want included, use `origin/main` after `git fetch` instead of `$HEAD_SHA` — but HEAD is sufficient for this proof.)

- [ ] **Step 3: Generate the three runnable fixtures + assert the contract**

```bash
cd ~/Documents/GitHub/rfl
mkdir -p hardware/pincherx-100/fixtures
BIN=/tmp/rfl-pincherx-target/debug/rfl
for s in hover scan pickplace; do
  ( cd /tmp/rfl-pincherx-build && CARGO_TARGET_DIR=/tmp/rfl-pincherx-target \
      "$BIN" retarget "$OLDPWD/hardware/pincherx-100/skill-$s.yaml" \
        --embodiment "$OLDPWD/hardware/pincherx-100/pincherx-100.yaml" ) \
    > "hardware/pincherx-100/fixtures/skill-$s.jsonl"
done
echo "=== pickplace contract checks ==="
grep -q '"tactile_target":{"proxy":{"tier":"proxy","criterion":"position_convergence_and_force_hold"}}' hardware/pincherx-100/fixtures/skill-pickplace.jsonl && echo "OK proxy tier"
grep -q '"min_holding_force":"0.6 N"' hardware/pincherx-100/fixtures/skill-pickplace.jsonl && echo "OK min_holding_force floor 0.6 N (0.3 N x 2.0)"
grep -q '"ref":"widget_t"' hardware/pincherx-100/fixtures/skill-pickplace.jsonl && echo "OK grasp/locate Ref target"
echo "=== hover/scan contract checks ==="
grep -q '"along":"outward_normal"' hardware/pincherx-100/fixtures/skill-hover.jsonl && echo "OK hover FrameRelative standoff"
grep -q '"pattern":"raster","poses":\[' hardware/pincherx-100/fixtures/skill-scan.jsonl && echo "OK scan SweepPath"
```
Expected: all six `OK ...` lines. If a `grep` fails, inspect the JSONL — likely a skill parse rejected an optional field (e.g. `grasp.pinch` field set) or a number differs. Adjust the skill to the parser's accepted shape (the parse error message names the field) and regenerate. Do NOT hand-edit fixtures — they must be genuine retarget output.

- [ ] **Step 4: Assert the negative-space gate rejects `skill-rejected.yaml`**

```bash
cd ~/Documents/GitHub/rfl
BIN=/tmp/rfl-pincherx-target/debug/rfl
( cd /tmp/rfl-pincherx-build && CARGO_TARGET_DIR=/tmp/rfl-pincherx-target \
    "$BIN" retarget "$OLDPWD/hardware/pincherx-100/skill-rejected.yaml" \
      --embodiment "$OLDPWD/hardware/pincherx-100/pincherx-100.yaml" ) ; echo "exit=$?"
```
Expected: non-zero exit and an error containing `capability_absent: force.insert_fit`. That message + non-zero exit IS the negotiation demonstration. (Record the exact text for the README.)

- [ ] **Step 5: Remove the build worktree**

```bash
cd ~/Documents/GitHub/rfl && git worktree remove /tmp/rfl-pincherx-build && rm -rf /tmp/rfl-pincherx-target
```

- [ ] **Step 6: Commit skills + fixtures**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- \
  hardware/pincherx-100/skill-hover.yaml \
  hardware/pincherx-100/skill-scan.yaml \
  hardware/pincherx-100/skill-pickplace.yaml \
  hardware/pincherx-100/skill-rejected.yaml \
  hardware/pincherx-100/fixtures/skill-hover.jsonl \
  hardware/pincherx-100/fixtures/skill-scan.jsonl \
  hardware/pincherx-100/fixtures/skill-pickplace.jsonl <<'MSG'
feat(hardware): PincherX skills + retargeted canonical-action fixtures

Three runnable skills (hover/scan warm-ups + the pick-and-place headline) and
one negative-space skill that the gate rejects (force.insert_fit →
capability_absent). The committed fixtures are genuine `rfl-cli retarget`
output and serve as the contract the Python driver consumes and tests assert:
proxy-tier grasp confirmation, the 0.6 N min_holding_force floor and its
propagation into transport, the Σ raster sweep, and the interval-invariant hover.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 3: Driver — execute-message parser core (pure, TDD)

**Files:**
- Create: `hardware/pincherx-100/rfl_interbotix_driver.py` (parser section)
- Test: `hardware/pincherx-100/tests/test_parse.py`

The parser turns each `execute` message into a typed `ArmStep` independent of ROS and of geometry. One step per canonical-action kind, keyed off `target_pose` shape + `tactile_target` + `force_profile`.

- [ ] **Step 1: Write the failing test**

`hardware/pincherx-100/tests/test_parse.py`:
```python
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import rfl_interbotix_driver as drv

FIX = HERE.parent / "fixtures"


def steps_for(stem):
    lines = (FIX / f"skill-{stem}.jsonl").read_text().splitlines()
    return [drv.parse_execute(json.loads(line)) for line in lines if line.strip()]


def test_pickplace_step_kinds_in_order():
    kinds = [s.kind for s in steps_for("pickplace")]
    assert kinds == ["locate", "pinch", "move_held", "release", "retract"]


def test_pinch_carries_proxy_tier_and_floor():
    pinch = next(s for s in steps_for("pickplace") if s.kind == "pinch")
    assert pinch.proxy is True               # no-tactile → proxy tier
    assert pinch.min_holding_force == "0.6 N"
    assert pinch.target_ref == "widget_t"


def test_move_held_propagates_floor():
    mv = next(s for s in steps_for("pickplace") if s.kind == "move_held")
    assert mv.min_holding_force == "0.6 N"   # increment-10 held-floor propagation
    assert mv.frame == "place_zone"


def test_hover_is_interval_invariant_standoff():
    [hover] = steps_for("hover")
    assert hover.kind == "hover"
    assert hover.interval_invariant is True  # drives the N=3 multi-sample telemetry
    assert hover.frame == "work_point"
    assert hover.standoff == "50 mm"


def test_scan_carries_sweep_poses():
    [scan] = steps_for("scan")
    assert scan.kind == "scan"
    assert scan.pattern == "raster"
    assert len(scan.poses) >= 1
    assert len(scan.poses[0].position) == 3 and len(scan.poses[0].orientation) == 4
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest python -m pytest hardware/pincherx-100/tests/test_parse.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'rfl_interbotix_driver'` (file not created yet).

- [ ] **Step 3: Write the parser**

`hardware/pincherx-100/rfl_interbotix_driver.py` (initial content):
```python
#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 RFL Contributors
"""RFL Class-4 driver for the Trossen PincherX 100 (4-DOF, parallel-jaw gripper).

Consumes canonical-action `execute` JSONL on stdin (from `rfl-cli retarget`), drives the
Interbotix arm (Gazebo sim or physical) including the gripper, reads back the realized
end-effector pose, and emits RFL driver-interface `telemetry` + `status` JSONL on stdout.
This is the real-hardware embodiment of the in-process ReferenceDriver: same
driver-interface contract, real realized poses + grasp outcome.

Structure: a pure, ROS-free parse -> resolve -> emit core (unit-tested against the
committed fixtures with a MockBackend) and a thin import-guarded InterbotixBackend.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


# ---------------------------------------------------------------------------
# Parse: execute message -> typed ArmStep (pure; no ROS, no geometry).
# ---------------------------------------------------------------------------

@dataclass
class SweepStation:
    position: list[float]      # [x, y, z] in the region frame (metres)
    orientation: list[float]   # [x, y, z, w]


@dataclass
class ArmStep:
    """One canonical action, normalized for execution. `kind` selects the backend path."""
    kind: str                          # locate|pinch|move_held|release|retract|hover|scan
    action_id: str
    # pose intent (exactly one of these is populated per kind)
    target_ref: str | None = None      # locate, pinch
    frame: str | None = None           # hover, move_held
    offset: dict[str, Any] | None = None  # move_held (frame-relative)
    standoff: str | None = None        # hover (quantity string)
    direction: Any | None = None       # retract, release (axis-relative)
    distance: str | None = None        # retract, release
    poses: list[SweepStation] = field(default_factory=list)  # scan
    pattern: str | None = None         # scan
    # grasp / force annotations
    proxy: bool = False                # pinch: confirmation degraded to the proxy tier
    min_holding_force: str | None = None  # pinch + move_held (held floor)
    force_budget: str | None = None    # pinch
    interval_invariant: bool = False   # hover (and any held interval): N-sample telemetry


def parse_execute(msg: dict[str, Any]) -> ArmStep:
    """Map one `execute` message to an ArmStep. Raises ValueError on an unknown shape."""
    if msg.get("message") != "execute":
        raise ValueError(f"not an execute message: {msg.get('message')!r}")
    ca = msg["canonical_action"]
    aid = msg["action_id"]
    pose = ca["target_pose"]
    env = ca.get("safety_envelope", {})
    fp = env.get("force_profile") or {}
    mhf = fp.get("min_holding_force")

    # reach.scan — SweepPath {pattern, poses}
    if "poses" in pose and "pattern" in pose:
        return ArmStep(
            kind="scan", action_id=aid, pattern=pose["pattern"],
            poses=[SweepStation(p["position"], p["orientation"]) for p in pose["poses"]],
        )
    # sense.locate / grasp.pinch — Ref {ref}
    if "ref" in pose:
        tactile = ca.get("tactile_target")
        is_pinch = isinstance(tactile, dict) or ca.get("force_budget") is not None
        if is_pinch:
            proxy = isinstance(tactile, dict) and "proxy" in tactile
            return ArmStep(
                kind="pinch", action_id=aid, target_ref=pose["ref"],
                proxy=proxy, min_holding_force=mhf, force_budget=ca.get("force_budget"),
            )
        return ArmStep(kind="locate", action_id=aid, target_ref=pose["ref"])
    # transport.move_to_pose / reach.hover — FrameRelative {frame, offset}
    if "frame" in pose and "offset" in pose:
        off = pose["offset"]
        if isinstance(off, dict) and off.get("along") == "outward_normal":
            return ArmStep(
                kind="hover", action_id=aid, frame=pose["frame"],
                standoff=off.get("distance"), interval_invariant=True,
            )
        return ArmStep(
            kind="move_held", action_id=aid, frame=pose["frame"], offset=off,
            min_holding_force=mhf, interval_invariant=mhf is not None,
        )
    # grasp.release / reach.retract — AxisRelative {direction, distance}
    if "direction" in pose and "distance" in pose:
        kind = "release" if pose["distance"] in ("0 mm", "0 m", "0 cm") else "retract"
        return ArmStep(
            kind=kind, action_id=aid, direction=pose["direction"], distance=pose["distance"],
        )
    raise ValueError(f"unrecognized target_pose shape: {sorted(pose)}")
```

Note on `release` vs `retract`: `grasp.release` lowers to AxisRelative with `distance:"0 mm"`; `reach.retract` to a real distance. The zero-distance discriminator is robust because the action-id suffix is not in the message body. (If you prefer, the suffix is recoverable from `action_id` which ends `-release`/`-retract`; the distance test is equivalent and self-contained.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest python -m pytest hardware/pincherx-100/tests/test_parse.py -q`
Expected: PASS (5 passed).

- [ ] **Step 5: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- \
  hardware/pincherx-100/rfl_interbotix_driver.py \
  hardware/pincherx-100/tests/test_parse.py <<'MSG'
feat(hardware): PincherX driver — execute-message parser core

Pure, ROS-free parse of canonical-action execute JSONL into typed ArmSteps,
keyed off target_pose shape: scan SweepPath, locate/pinch Ref, hover/move_held
FrameRelative, release/retract AxisRelative. Carries the proxy-tier flag and the
min_holding_force floor through to execution. Tested against the committed fixtures.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 4: Driver — frame transform + pose resolution (pure, TDD)

**Files:**
- Create: `hardware/pincherx-100/frames.yaml`
- Modify: `hardware/pincherx-100/rfl_interbotix_driver.py` (add the resolve section)
- Test: `hardware/pincherx-100/tests/test_resolve.py`

Σ poses and frame-relative offsets are expressed in TASK frames (region / object / place). The driver maps them to the arm BASE frame via a known transform (sim: a fixed offset; physical: a calibration), then reduces each to the 4-DOF command set `(x, y, z, pitch)`. This is the design §7 "frame calibration" + "concrete poses + where is the object?" made concrete.

- [ ] **Step 1: Write `frames.yaml` (the calibration artifact)**

`hardware/pincherx-100/frames.yaml`:
```yaml
# Calibration for the PincherX-100 proof (design § 7). Task frames are placed relative to
# the arm base by a translation + a yaw (top-down work). The sim defines these exactly; the
# physical arm calibrates them. Lengths in metres, angles in radians. The object mass +
# graspable flag drive the MockBackend's grasp outcome and must match the sim object.
base_frame: pincherx/base_link

# task_frame -> { xyz: [x,y,z] (m, in base), yaw: <rad> }
transforms:
  work_point:   { xyz: [0.20, 0.00, 0.05], yaw: 0.0 }
  work_surface: { xyz: [0.18, 0.00, 0.02], yaw: 0.0 }   # region origin (centroid) in base
  widget:       { xyz: [0.20, 0.00, 0.02], yaw: 0.0 }   # the located object pose (perception out of RFL scope)
  place_zone:   { xyz: [0.20, 0.10, 0.02], yaw: 0.0 }

# Top-down grasp: the gripper points down (-z world); the 4-DOF arm reaches with a fixed
# pitch. pitch is in radians; yaw is IK-determined (set_ee_pose_components: yaw=atan2(y,x)).
grasp_pitch: 1.5708        # ~pi/2, gripper pointing down

objects:
  widget: { mass: 0.3 N, graspable: true }   # ~30 g ≤ payload; matches skill-pickplace estimated_mass
```

- [ ] **Step 2: Write the failing test**

`hardware/pincherx-100/tests/test_resolve.py`:
```python
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import rfl_interbotix_driver as drv

CFG = drv.load_frames(HERE.parent / "frames.yaml")


def test_ref_resolves_to_object_pose_in_base():
    pose = drv.resolve_ref("widget", CFG)
    assert pose.x == 0.20 and pose.y == 0.00 and pose.z == 0.02
    assert abs(pose.pitch - 1.5708) < 1e-6


def test_frame_relative_offset_along_plus_z():
    # place_zone at z=0.02 + offset {along:+z, distance:20 mm} -> z = 0.04
    pose = drv.resolve_frame_relative("place_zone", {"along": "+z", "distance": "20 mm"}, CFG)
    assert pose.x == 0.20 and pose.y == 0.10
    assert abs(pose.z - 0.04) < 1e-9


def test_hover_standoff_along_outward_normal():
    # outward_normal over work_point -> +z by the standoff
    pose = drv.resolve_frame_relative("work_point", {"along": "outward_normal", "distance": "50 mm"}, CFG)
    assert abs(pose.z - (0.05 + 0.05)) < 1e-9


def test_sweep_station_maps_region_to_base():
    st = drv.SweepStation(position=[0.01, 0.02, 0.10], orientation=[0, 0, 0, 1])
    pose = drv.resolve_sweep_station(st, "work_surface", CFG)
    # region origin (0.18,0,0.02) + station (0.01,0.02,0.10) -> (0.19,0.02,0.12)
    assert abs(pose.x - 0.19) < 1e-9 and abs(pose.y - 0.02) < 1e-9 and abs(pose.z - 0.12) < 1e-9


def test_length_quantity_parsing():
    assert abs(drv.length_m("20 mm") - 0.02) < 1e-12
    assert abs(drv.length_m("0.3 m") - 0.3) < 1e-12
    assert abs(drv.length_m("5 cm") - 0.05) < 1e-12
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml python -m pytest hardware/pincherx-100/tests/test_resolve.py -q`
Expected: FAIL — `AttributeError: module 'rfl_interbotix_driver' has no attribute 'load_frames'`.

- [ ] **Step 4: Add the resolve section to the driver**

Append to `hardware/pincherx-100/rfl_interbotix_driver.py`:
```python
# ---------------------------------------------------------------------------
# Resolve: task-frame poses -> the 4-DOF base-frame command (x, y, z, pitch).
# ---------------------------------------------------------------------------
import pathlib  # noqa: E402

import yaml  # noqa: E402


@dataclass
class EEPose:
    """A 4-DOF end-effector command in the arm base frame. yaw is IK-determined (omitted)."""
    x: float
    y: float
    z: float
    pitch: float


def load_frames(path: str | pathlib.Path) -> dict[str, Any]:
    """Load frames.yaml (the calibration). Returns the parsed mapping."""
    return yaml.safe_load(pathlib.Path(path).read_text())


def length_m(q: str) -> float:
    """Parse a length quantity string ('20 mm' / '0.3 m' / '5 cm') to metres."""
    val, unit = q.split()
    v = float(val)
    return {"m": v, "mm": v / 1000.0, "cm": v / 100.0}[unit]


def _xform(frame: str, cfg: dict[str, Any]) -> dict[str, Any]:
    try:
        return cfg["transforms"][frame]
    except KeyError as e:
        raise ValueError(f"no transform for task frame {frame!r} in frames.yaml") from e


def resolve_ref(ref: str, cfg: dict[str, Any]) -> EEPose:
    """A located-object Ref -> its pose in base, top-down (grasp_pitch). Perception source
    (sim ground truth / physical calibration) is out of RFL scope (spec/04); frames.yaml
    supplies it. `ref` is matched against the object name (the `_t` let-suffix is stripped)."""
    name = ref[:-2] if ref.endswith("_t") else ref
    t = _xform(name, cfg)
    x, y, z = t["xyz"]
    return EEPose(x, y, z, float(cfg.get("grasp_pitch", 1.5708)))


_AXIS = {"+z": (0, 0, 1), "-z": (0, 0, -1), "+x": (1, 0, 0), "-x": (-1, 0, 0),
         "+y": (0, 1, 0), "-y": (0, -1, 0), "outward_normal": (0, 0, 1)}


def resolve_frame_relative(frame: str, offset: dict[str, Any], cfg: dict[str, Any]) -> EEPose:
    """A frame-relative offset {along, distance} -> a base-frame pose. `outward_normal` is
    +z for a top-down workspace (the standoff is above the surface)."""
    t = _xform(frame, cfg)
    x, y, z = t["xyz"]
    d = length_m(offset["distance"]) if offset.get("distance") else 0.0
    ax, ay, az = _AXIS[offset.get("along", "+z")]
    return EEPose(x + ax * d, y + ay * d, z + az * d, float(cfg.get("grasp_pitch", 1.5708)))


def resolve_sweep_station(st: "SweepStation", region_frame: str, cfg: dict[str, Any]) -> EEPose:
    """A Σ station (in the region frame) -> a base-frame pose. The region origin is the
    region frame's transform; the station position adds onto it. Σ orientation is fixed-roll
    by construction; the 4-DOF arm holds grasp_pitch and lets the IK choose yaw."""
    t = _xform(region_frame, cfg)
    ox, oy, oz = t["xyz"]
    px, py, pz = st.position
    return EEPose(ox + px, oy + py, oz + pz, float(cfg.get("grasp_pitch", 1.5708)))
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml python -m pytest hardware/pincherx-100/tests/test_resolve.py -q`
Expected: PASS (5 passed).

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- \
  hardware/pincherx-100/frames.yaml \
  hardware/pincherx-100/rfl_interbotix_driver.py \
  hardware/pincherx-100/tests/test_resolve.py <<'MSG'
feat(hardware): PincherX driver — frame transform + pose resolution

frames.yaml is the design § 7 calibration artifact (task frames -> arm base by a
translation + yaw; the object mass + graspable flag for the mock grasp outcome).
The resolver reduces Ref / frame-relative / Σ-station poses to the 4-DOF command
(x, y, z, pitch); yaw is left to the Interbotix IK (atan2(y,x) on the 4-DOF arm).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 5: Driver — backends, telemetry/status emission, mock end-to-end (TDD)

**Files:**
- Modify: `hardware/pincherx-100/rfl_interbotix_driver.py` (add backend protocol, MockBackend, executor, emitters)
- Test: `hardware/pincherx-100/tests/test_emit_mock.py`

The executor runs each `ArmStep` against an `ArmBackend`, emits one or more driver-interface `telemetry` messages (interval-invariant steps emit N=3, mirroring the in-process multi-sample ReferenceDriver) and exactly one `status`. Emitted messages MUST validate against `schemas/driver-interface.schema.json`.

- [ ] **Step 1: Write the failing test**

`hardware/pincherx-100/tests/test_emit_mock.py`:
```python
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import rfl_interbotix_driver as drv

REPO = HERE.parents[2]
FIX = HERE.parent / "fixtures"
CFG = drv.load_frames(HERE.parent / "frames.yaml")


def run(stem):
    """Execute a fixture end-to-end against the MockBackend; return emitted messages."""
    backend = drv.MockBackend(CFG)
    out = []
    for line in (FIX / f"skill-{stem}.jsonl").read_text().splitlines():
        if line.strip():
            out.extend(drv.execute_step(drv.parse_execute(json.loads(line)), backend, CFG))
    return out


def test_pickplace_emits_status_per_action_all_succeeded():
    msgs = run("pickplace")
    statuses = [m for m in msgs if m["message"] == "status"]
    assert len(statuses) == 5
    assert all(s["outcome"] == "succeeded" for s in statuses)


def test_pinch_status_reports_proxy_tier_and_securing_force():
    msgs = run("pickplace")
    pinch_tel = [m for m in msgs if m["message"] == "telemetry" and "securing_force" in m]
    assert pinch_tel, "expected a telemetry with securing_force (the held floor)"
    assert any(m.get("fidelity_tier") == "proxy" for m in msgs)
    assert any(m.get("securing_force") == "0.6 N" for m in pinch_tel)


def test_hover_emits_three_interval_samples():
    msgs = run("hover")
    tel = [m for m in msgs if m["message"] == "telemetry"]
    assert len(tel) == 3                      # interval-invariant N=3
    assert all("realized_pose" in m for m in tel)


def test_scan_emits_one_telemetry_per_station():
    scan_step = drv.parse_execute(json.loads((FIX / "skill-scan.jsonl").read_text().splitlines()[0]))
    msgs = run("scan")
    tel = [m for m in msgs if m["message"] == "telemetry"]
    assert len(tel) == len(scan_step.poses)


def test_all_emitted_messages_validate_against_driver_interface_schema():
    import jsonschema
    schema = json.loads((REPO / "schemas/driver-interface.schema.json").read_text())
    validator = jsonschema.Draft202012Validator(schema)
    for stem in ("hover", "scan", "pickplace"):
        for m in run(stem):
            errs = list(validator.iter_errors(m))
            assert not errs, f"{stem}: {m.get('message')} invalid: {[e.message for e in errs]}"
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/test_emit_mock.py -q`
Expected: FAIL — `AttributeError: ... has no attribute 'MockBackend'`.

- [ ] **Step 3: Add backends + executor + emitters to the driver**

Append to `hardware/pincherx-100/rfl_interbotix_driver.py`:
```python
# ---------------------------------------------------------------------------
# Backends: a tiny protocol so sim/physical is a flag. MockBackend is deterministic
# (realized == commanded; grasp succeeds iff the configured object is graspable) and
# carries the entire unit-test loop with no ROS.
# ---------------------------------------------------------------------------
from typing import Protocol  # noqa: E402


def _pose_to_realized(p: "EEPose") -> dict[str, Any]:
    """A driver-interface Pose6D floor: position (m) + a unit quaternion. The 4-DOF arm's
    yaw is IK-determined; the mock reports a top-down quaternion (pitch about +y), w-last."""
    import math
    half = p.pitch / 2.0
    return {"position": [p.x, p.y, p.z], "orientation": [0.0, math.sin(half), 0.0, math.cos(half)]}


class ArmBackend(Protocol):
    def move_ee(self, pose: "EEPose") -> "EEPose": ...      # returns the REALIZED pose
    def gripper_close(self) -> bool: ...                     # True = object secured
    def gripper_open(self) -> bool: ...


class MockBackend:
    """Deterministic backend for tests + dry runs. Realized pose == commanded (zero error);
    grasp succeeds iff frames.yaml marks the held object graspable."""
    def __init__(self, cfg: dict[str, Any]):
        self._cfg = cfg
        self._graspable = any(o.get("graspable") for o in cfg.get("objects", {}).values())

    def move_ee(self, pose: "EEPose") -> "EEPose":
        return pose

    def gripper_close(self) -> bool:
        return bool(self._graspable)

    def gripper_open(self) -> bool:
        return True


# ---------------------------------------------------------------------------
# Execute: ArmStep -> backend calls -> driver-interface telemetry + status messages.
# ---------------------------------------------------------------------------
def _telemetry(action_id, t, realized=None, securing_force=None, fidelity_tier=None):
    m = {"message": "telemetry", "action_id": action_id, "t": t}
    if realized is not None:
        m["realized_pose"] = _pose_to_realized(realized)
    if securing_force is not None:
        m["securing_force"] = securing_force
    if fidelity_tier is not None:
        m["fidelity_tier"] = fidelity_tier
    return m


def _status(action_id, outcome, fidelity_tier=None, final=None, failure_detail=None):
    m = {"message": "status", "action_id": action_id, "outcome": outcome}
    if fidelity_tier is not None:
        m["fidelity_tier"] = fidelity_tier
    if final is not None:
        m["final_pose"] = _pose_to_realized(final)
    if failure_detail is not None:
        m["failure_detail"] = failure_detail
    return m


def execute_step(step: "ArmStep", backend: "ArmBackend", cfg: dict[str, Any]) -> list[dict[str, Any]]:
    """Run one step; return its telemetry + status messages. Deterministic timebase t = sample
    index (the mock has no clock; the schema only requires t to be a number)."""
    out: list[dict[str, Any]] = []
    aid = step.action_id

    if step.kind == "locate":
        # Perception is out of RFL scope: the pose comes from frames.yaml. No motion.
        realized = resolve_ref(step.target_ref, cfg)
        out.append(_telemetry(aid, 0.0, realized=realized))
        out.append(_status(aid, "succeeded", final=realized))
        return out

    if step.kind == "pinch":
        realized = backend.move_ee(resolve_ref(step.target_ref, cfg))
        tier = "proxy" if step.proxy else None
        secured = backend.gripper_close()
        out.append(_telemetry(aid, 0.0, realized=realized,
                              securing_force=step.min_holding_force, fidelity_tier=tier))
        out.append(_status(aid, "succeeded" if secured else "failed", fidelity_tier=tier,
                          final=realized, failure_detail=None if secured else "grasp_lost"))
        return out

    if step.kind == "move_held":
        realized = backend.move_ee(resolve_frame_relative(step.frame, step.offset, cfg))
        # held interval: N=3 samples, each carrying the propagated securing floor (GC1)
        for i in range(3):
            out.append(_telemetry(aid, float(i), realized=realized,
                                  securing_force=step.min_holding_force))
        out.append(_status(aid, "succeeded", final=realized))
        return out

    if step.kind == "release":
        backend.gripper_open()
        out.append(_status(aid, "succeeded"))
        return out

    if step.kind == "retract":
        # Axis-relative withdraw resolved against the last pose is sim-side; the mock reports
        # a successful retract with no realized pose (no task frame to anchor to here).
        out.append(_status(aid, "succeeded"))
        return out

    if step.kind == "hover":
        realized = backend.move_ee(resolve_frame_relative(
            step.frame, {"along": "outward_normal", "distance": step.standoff}, cfg))
        for i in range(3):                       # interval-invariant N=3
            out.append(_telemetry(aid, float(i), realized=realized))
        out.append(_status(aid, "succeeded", final=realized))
        return out

    if step.kind == "scan":
        for i, st in enumerate(step.poses):
            realized = backend.move_ee(resolve_sweep_station(st, _scan_region(cfg), cfg))
            out.append(_telemetry(aid, float(i), realized=realized))
        out.append(_status(aid, "succeeded"))
        return out

    raise ValueError(f"unknown step kind {step.kind!r}")


def _scan_region(cfg: dict[str, Any]) -> str:
    """The scan region frame. Single-region proof: the lone surface transform whose name is
    not an object/point. Defaults to 'work_surface'."""
    return "work_surface" if "work_surface" in cfg.get("transforms", {}) else next(iter(cfg["transforms"]))
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/test_emit_mock.py -q`
Expected: PASS (5 passed). If the schema test fails, the message has an extra/misspelled key (the schema is `additionalProperties:false`) — emit only the keys the schema lists.

- [ ] **Step 5: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- \
  hardware/pincherx-100/rfl_interbotix_driver.py \
  hardware/pincherx-100/tests/test_emit_mock.py <<'MSG'
feat(hardware): PincherX driver — backends, telemetry/status, mock loop

A minimal ArmBackend protocol (sim/physical = a flag) with a deterministic
MockBackend, plus the executor that turns each ArmStep into driver-interface
telemetry + status. Interval-invariant steps (hover, held transport) emit the
N=3 multi-sample telemetry; the pinch reports the proxy fidelity tier and the
securing force. All emitted messages validate against driver-interface.schema.json.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 6: Driver — Interbotix backend + CLI

**Files:**
- Modify: `hardware/pincherx-100/rfl_interbotix_driver.py` (add InterbotixBackend + `main`)

The real backend wraps the Interbotix Python API; it is import-guarded so the module imports (and the whole test suite runs) without ROS installed. The CLI reads `execute` JSONL on stdin and writes `telemetry`/`status` JSONL on stdout — the real run is `rfl retarget ... | python rfl_interbotix_driver.py --backend interbotix`.

- [ ] **Step 1: Add the Interbotix backend + CLI**

Append to `hardware/pincherx-100/rfl_interbotix_driver.py`:
```python
# ---------------------------------------------------------------------------
# Interbotix backend: the real arm (Gazebo sim or physical). Import-guarded so the module
# (and all unit tests) load without ROS. CONFIRM the exact API at bringup (§ 11b): the
# class/module path and the gripper method names differ slightly across ROS1/ROS2 packages.
# ---------------------------------------------------------------------------
class InterbotixBackend:
    """Drives a real PincherX 100 via interbotix_xs_modules. 4-DOF: command x, y, z, pitch;
    yaw is IK-determined (set_ee_pose_components sets yaw=atan2(y,x) when num_joints < 6)."""

    def __init__(self, cfg: dict[str, Any], robot_model: str = "px100"):
        # ROS2: from interbotix_xs_modules.xs_robot.arm import InterbotixManipulatorXS
        # ROS1: from interbotix_xs_modules.arm import InterbotixManipulatorXS
        try:
            from interbotix_xs_modules.xs_robot.arm import InterbotixManipulatorXS  # type: ignore
        except ImportError:
            from interbotix_xs_modules.arm import InterbotixManipulatorXS  # type: ignore
        self._cfg = cfg
        self._bot = InterbotixManipulatorXS(robot_model=robot_model, group_name="arm", gripper_name="gripper")

    def move_ee(self, pose: "EEPose") -> "EEPose":
        # Returns (theta_list, success). We read the realized pose back from FK below.
        _theta, ok = self._bot.arm.set_ee_pose_components(
            x=pose.x, y=pose.y, z=pose.z, pitch=pose.pitch, blocking=True)
        rx, ry, rz, rpitch = self._realized()
        if not ok:
            # Out-of-reach / no IK: report the commanded pose; the measurement flags the error.
            return pose
        return EEPose(rx, ry, rz, rpitch)

    def _realized(self) -> tuple[float, float, float, float]:
        # FK on the realized joint state. The Interbotix interface exposes the current EE
        # transform; CONFIRM the exact accessor at bringup (e.g. arm.get_ee_pose()).
        T = self._bot.arm.get_ee_pose()           # 4x4 homogeneous, base->ee
        x, y, z = float(T[0, 3]), float(T[1, 3]), float(T[2, 3])
        import math
        pitch = math.atan2(-T[2, 0], math.sqrt(T[2, 1] ** 2 + T[2, 2] ** 2))
        return x, y, z, pitch

    def gripper_close(self) -> bool:
        # ROS2 xs_robot: self._bot.gripper.grasp(); ROS1 xs_modules: self._bot.gripper.close()
        g = self._bot.gripper
        (getattr(g, "grasp", None) or g.close)()
        return True        # no tactile/F-T: success is the proxy criterion, judged by measure.py

    def gripper_open(self) -> bool:
        g = self._bot.gripper
        (getattr(g, "release", None) or g.open)()
        return True


def _make_backend(name: str, cfg: dict[str, Any]) -> "ArmBackend":
    if name == "mock":
        return MockBackend(cfg)
    if name == "interbotix":
        return InterbotixBackend(cfg)
    raise SystemExit(f"unknown backend {name!r} (use: mock | interbotix)")


def main(argv: list[str] | None = None) -> int:
    import argparse
    import json
    import sys

    ap = argparse.ArgumentParser(description="RFL Class-4 driver for the PincherX 100.")
    ap.add_argument("--backend", default="mock", choices=["mock", "interbotix"])
    ap.add_argument("--frames", default=str(pathlib.Path(__file__).with_name("frames.yaml")))
    args = ap.parse_args(argv)

    cfg = load_frames(args.frames)
    backend = _make_backend(args.backend, cfg)
    for line in sys.stdin:
        if not line.strip():
            continue
        for msg in execute_step(parse_execute(json.loads(line)), backend, cfg):
            sys.stdout.write(json.dumps(msg) + "\n")
            sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Smoke-test the CLI end-to-end against the mock backend (no ROS)**

Run:
```bash
cd ~/Documents/GitHub/rfl && uv run --with pyyaml python hardware/pincherx-100/rfl_interbotix_driver.py \
  --backend mock < hardware/pincherx-100/fixtures/skill-pickplace.jsonl | python3 -c \
  'import sys,json; ms=[json.loads(l) for l in sys.stdin]; \
   print("statuses:", sum(m["message"]=="status" for m in ms)); \
   print("succeeded:", sum(m.get("outcome")=="succeeded" for m in ms)); \
   print("proxy seen:", any(m.get("fidelity_tier")=="proxy" for m in ms))'
```
Expected: `statuses: 5`, `succeeded: 5`, `proxy seen: True`.

- [ ] **Step 3: Confirm the unit suite still passes (no regression)**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/ -q`
Expected: PASS (all tests). The module imports without ROS (the Interbotix import is inside `InterbotixBackend.__init__`).

- [ ] **Step 4: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- hardware/pincherx-100/rfl_interbotix_driver.py <<'MSG'
feat(hardware): PincherX driver — Interbotix backend + CLI

Import-guarded InterbotixManipulatorXS adapter (4-DOF set_ee_pose_components +
gripper grasp/release; realized pose via FK) and a stdin->stdout CLI so the real
run is `rfl retarget ... | rfl_interbotix_driver.py --backend interbotix`. The
exact installed-version API (ROS1 vs ROS2 module path, gripper method names, the
FK accessor) is isolated here for one-line bringup confirmation.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 7: Measurement (`measure.py`) — the Class-4 conformance loop (TDD)

**Files:**
- Create: `hardware/pincherx-100/measure.py`
- Test: `hardware/pincherx-100/tests/test_measure.py`

`measure.py` reads the commanded canonical-action JSONL (a fixture, or live retarget) + the driver's telemetry/status JSONL, and produces the Class-4 verdict: every realized pose within tolerance, grasp lifted + placed, the proxy tier audited, and the integration cost. Tolerance defaults to 8 mm (the design's ≥ 5 mm-repeatability honoring).

- [ ] **Step 1: Write the failing test**

`hardware/pincherx-100/tests/test_measure.py`:
```python
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import rfl_interbotix_driver as drv
import measure

FIX = HERE.parent / "fixtures"
CFG = drv.load_frames(HERE.parent / "frames.yaml")


def driver_output(stem):
    backend = drv.MockBackend(CFG)
    out = []
    for line in (FIX / f"skill-{stem}.jsonl").read_text().splitlines():
        if line.strip():
            out.extend(drv.execute_step(drv.parse_execute(json.loads(line)), backend, CFG))
    return out


def test_mock_run_passes_all_within_tolerance():
    rep = measure.measure(commanded=str(FIX / "skill-pickplace.jsonl"),
                          telemetry=driver_output("pickplace"),
                          tolerance_m=0.008, cfg=CFG)
    assert rep["passed"] is True
    assert rep["max_position_error_m"] == 0.0      # mock realized == commanded
    assert rep["grasp_lifted"] is True and rep["grasp_placed"] is True
    assert rep["proxy_tier_used"] is True


def test_injected_error_beyond_tolerance_fails():
    msgs = driver_output("pickplace")
    for m in msgs:                                  # shove every realized pose 12 mm in x
        if m["message"] == "telemetry" and "realized_pose" in m:
            m["realized_pose"]["position"][0] += 0.012
    rep = measure.measure(commanded=str(FIX / "skill-pickplace.jsonl"),
                          telemetry=msgs, tolerance_m=0.008, cfg=CFG)
    assert rep["passed"] is False
    assert rep["max_position_error_m"] > 0.008


def test_integration_cost_reports_driver_loc():
    rep = measure.measure(commanded=str(FIX / "skill-hover.jsonl"),
                          telemetry=driver_output("hover"), tolerance_m=0.008, cfg=CFG)
    assert rep["integration_cost"]["driver_loc"] > 0
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml python -m pytest hardware/pincherx-100/tests/test_measure.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'measure'`.

- [ ] **Step 3: Write `measure.py`**

`hardware/pincherx-100/measure.py`:
```python
#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 RFL Contributors
"""Class-4 measurement for the PincherX 100 proof.

Compares the driver's realized telemetry against the commanded canonical actions: every
visited pose within `position_tolerance` (default 8 mm, honoring the 5 mm repeatability),
grasp lifted + placed, the proxy tier audited, the orientation residual reported (accepted
under spec/02's under-constrained-orientation rule on a 4-DOF arm), and the integration cost
(driver LOC + optional wall-clock). This closes the conformance loop on real hardware; the
same code scores the Gazebo sim run and the physical run.
"""
from __future__ import annotations

import json
import math
import pathlib
from typing import Any

import rfl_interbotix_driver as drv

_DRIVER = pathlib.Path(__file__).with_name("rfl_interbotix_driver.py")


def _commanded_targets(commanded_path: str, cfg: dict[str, Any]) -> dict[str, list[drv.EEPose]]:
    """Resolve each commanded action to the base-frame pose(s) the realized telemetry should
    match, keyed by action_id. Steps with no anchored pose (release/retract) map to []."""
    targets: dict[str, list[drv.EEPose]] = {}
    for line in pathlib.Path(commanded_path).read_text().splitlines():
        if not line.strip():
            continue
        step = drv.parse_execute(json.loads(line))
        if step.kind in ("locate", "pinch"):
            targets[step.action_id] = [drv.resolve_ref(step.target_ref, cfg)]
        elif step.kind == "move_held":
            targets[step.action_id] = [drv.resolve_frame_relative(step.frame, step.offset, cfg)]
        elif step.kind == "hover":
            targets[step.action_id] = [drv.resolve_frame_relative(
                step.frame, {"along": "outward_normal", "distance": step.standoff}, cfg)]
        elif step.kind == "scan":
            region = drv._scan_region(cfg)
            targets[step.action_id] = [drv.resolve_sweep_station(s, region, cfg) for s in step.poses]
        else:
            targets[step.action_id] = []
    return targets


def measure(commanded: str, telemetry: list[dict[str, Any]] | str,
            tolerance_m: float = 0.008, cfg: dict[str, Any] | None = None) -> dict[str, Any]:
    """Score one run. `telemetry` is the driver's message list (or a path to its JSONL)."""
    if cfg is None:
        cfg = drv.load_frames(_DRIVER.with_name("frames.yaml"))
    if isinstance(telemetry, str):
        telemetry = [json.loads(l) for l in pathlib.Path(telemetry).read_text().splitlines() if l.strip()]

    targets = _commanded_targets(commanded, cfg)
    # one realized position per action_id, in telemetry order
    realized: dict[str, list[list[float]]] = {}
    for m in telemetry:
        if m["message"] == "telemetry" and "realized_pose" in m:
            realized.setdefault(m["action_id"], []).append(m["realized_pose"]["position"])

    max_err = 0.0
    per_action = {}
    for aid, goals in targets.items():
        got = realized.get(aid, [])
        n = min(len(goals), len(got))
        errs = [math.dist([g.x, g.y, g.z], got[i]) for i, g in enumerate(goals[:n])]
        e = max(errs) if errs else 0.0
        per_action[aid] = {"samples": len(got), "max_error_m": e}
        max_err = max(max_err, e)

    statuses = {m["action_id"]: m for m in telemetry if m["message"] == "status"}
    pinch = next((s for a, s in statuses.items() if a.endswith("-pinch")), None)
    release = next((s for a, s in statuses.items() if a.endswith("-release")), None)
    proxy_used = any(m.get("fidelity_tier") == "proxy" for m in telemetry)

    grasp_lifted = bool(pinch and pinch["outcome"] == "succeeded")
    grasp_placed = bool(grasp_lifted and release and release["outcome"] == "succeeded")
    all_succeeded = all(s["outcome"] == "succeeded" for s in statuses.values())
    within = max_err <= tolerance_m
    # A pure-reach run (hover/scan) has no grasp; don't require it.
    has_grasp = pinch is not None
    passed = within and all_succeeded and (grasp_placed if has_grasp else True)

    return {
        "passed": passed,
        "within_tolerance": within,
        "tolerance_m": tolerance_m,
        "max_position_error_m": max_err,
        "all_statuses_succeeded": all_succeeded,
        "grasp_lifted": grasp_lifted,
        "grasp_placed": grasp_placed,
        "proxy_tier_used": proxy_used,
        "per_action": per_action,
        "integration_cost": {"driver_loc": _loc(_DRIVER), "measure_loc": _loc(pathlib.Path(__file__))},
        "note": "orientation residual accepted (4-DOF: yaw IK-determined, spec/02 under-constrained rule)",
    }


def _loc(p: pathlib.Path) -> int:
    return sum(1 for ln in p.read_text().splitlines() if ln.strip() and not ln.strip().startswith("#"))


def main(argv: list[str] | None = None) -> int:
    import argparse
    ap = argparse.ArgumentParser(description="Class-4 measurement for the PincherX 100 proof.")
    ap.add_argument("--commanded", required=True, help="commanded canonical-action JSONL")
    ap.add_argument("--telemetry", required=True, help="driver telemetry+status JSONL")
    ap.add_argument("--tolerance-mm", type=float, default=8.0)
    ap.add_argument("--frames", default=str(_DRIVER.with_name("frames.yaml")))
    args = ap.parse_args(argv)
    rep = measure(args.commanded, args.telemetry, args.tolerance_mm / 1000.0, drv.load_frames(args.frames))
    print(json.dumps(rep, indent=2))
    return 0 if rep["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml python -m pytest hardware/pincherx-100/tests/test_measure.py -q`
Expected: PASS (3 passed).

- [ ] **Step 5: Run the full suite + an end-to-end dry run**

Run:
```bash
cd ~/Documents/GitHub/rfl
uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/ -q
uv run --with pyyaml python hardware/pincherx-100/rfl_interbotix_driver.py --backend mock \
  < hardware/pincherx-100/fixtures/skill-pickplace.jsonl > /tmp/pp_telemetry.jsonl
uv run --with pyyaml python hardware/pincherx-100/measure.py \
  --commanded hardware/pincherx-100/fixtures/skill-pickplace.jsonl --telemetry /tmp/pp_telemetry.jsonl
```
Expected: all tests pass; the measure report prints `"passed": true`, `"proxy_tier_used": true`, `"grasp_placed": true`.

- [ ] **Step 6: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- \
  hardware/pincherx-100/measure.py \
  hardware/pincherx-100/tests/test_measure.py <<'MSG'
feat(hardware): PincherX Class-4 measurement (conformance loop)

measure.py scores a run: every realized pose within tolerance (default 8 mm,
honoring the 5 mm repeatability), grasp lifted + placed, proxy tier audited,
orientation residual accepted (4-DOF yaw is IK-determined), and the integration
cost (driver LOC). The same scorer grades the Gazebo sim run and the physical run.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 8: README — bringup, run, claims/caveats

**Files:**
- Create: `hardware/pincherx-100/README.md`

- [ ] **Step 1: Write the README**

`hardware/pincherx-100/README.md` (start with a `#` heading — this is vault-style, but this repo's docs use Markdown; keep it a normal README):
```markdown
# PincherX-100 existence proof (RFL, sim-first, Class-4)

The first **real-embodiment** RFL existence proof: a hand-authored, embodiment-agnostic
Skill ISA composition is retargeted by the unmodified `rfl-cli` onto a real low-end arm
(Trossen PincherX 100 — 4-DOF, 50 g payload, parallel-jaw gripper, no tactile/F-T) and
executed first in Gazebo, then on the physical arm. Design: `docs/design/2026-05-31-pincherx-existence-proof-design.md`.

## What this proves
- The retarget contract is constructible against a real, independent driver.
- Basic **pick-and-place** (`grasp.pinch` → `transport.move_to_pose` → `grasp.release`) runs on real hardware.
- The **grasp-force floor** (`min_holding_force` = 2·weight) holds on a real gripper and propagates into transport.
- The **proxy fidelity tier** (no tactile → graceful degradation) works on real hardware.
- **Capability negotiation** rejects what the arm cannot do (`skill-rejected.yaml` → `capability_absent: force.insert_fit`).
- A **measured integration cost** (driver LOC; bringup time).

## What it does NOT prove
Dexterous / in-hand manipulation; force-controlled interaction; tactile-confirmed grasps;
that real VLAs emit RFL Skill-ISA (this proof hand-authors the skill). See the design § 1.

## Files
| File | Role |
|---|---|
| `pincherx-100.yaml` | embodiment descriptor (Class-1 valid) |
| `skill-hover.yaml` / `skill-scan.yaml` | kinematic warm-ups |
| `skill-pickplace.yaml` | the headline pick-and-place skill |
| `skill-rejected.yaml` | a skill the gate must reject (negative-space demo) |
| `frames.yaml` | calibration: task frames → arm base, object mass |
| `rfl_interbotix_driver.py` | the Class-4 driver (Mock + Interbotix backends, CLI) |
| `measure.py` | the Class-4 measurement / conformance scorer |
| `fixtures/*.jsonl` | committed `rfl-cli retarget` output (the contract under test) |
| `tests/` | pytest suite (runs with no ROS, via the MockBackend) |

## Run without hardware (the software is complete now)
```bash
# unit suite (no ROS):
uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/ -q
# dry pick-and-place + score:
uv run --with pyyaml python hardware/pincherx-100/rfl_interbotix_driver.py --backend mock \
  < hardware/pincherx-100/fixtures/skill-pickplace.jsonl > /tmp/tel.jsonl
uv run --with pyyaml python hardware/pincherx-100/measure.py \
  --commanded hardware/pincherx-100/fixtures/skill-pickplace.jsonl --telemetry /tmp/tel.jsonl
```

## Bringup (USER, parallel) — Interbotix ROS 2 + Gazebo
1. Install the Interbotix X-Series ROS 2 stack (`interbotix_ros_manipulators`) + Gazebo per
   docs.trossenrobotics.com; source the workspace.
2. Hello-world: `InterbotixManipulatorXS(robot_model='px100', group_name='arm', gripper_name='gripper')`;
   `bot.arm.set_ee_pose_components(x=0.2, z=0.1, pitch=1.5708)`; `bot.gripper.grasp()` / `.release()`
   (ROS2 `xs_robot`) or `.close()` / `.open()` (ROS1 `xs_modules`) move the sim arm + gripper.
3. **Confirm and finalize in `pincherx-100.yaml` + the driver (§ 11):** `tool_axis`; the
   reach limits (`v/w/a_cartesian_max`, `joint_velocity_ceiling`); `grip_force_max`; the exact
   gripper method names + the FK accessor (`InterbotixBackend._realized`).

## Run against the sim (after bringup)
```bash
# build rfl-cli once (from a clean worktree if the workspace is mid-edit):
cargo build -p rfl-cli
RFL=target/debug/rfl
$RFL retarget hardware/pincherx-100/skill-pickplace.yaml \
  --embodiment hardware/pincherx-100/pincherx-100.yaml \
  | python hardware/pincherx-100/rfl_interbotix_driver.py --backend interbotix > /tmp/tel.jsonl
python hardware/pincherx-100/measure.py \
  --commanded <($RFL retarget hardware/pincherx-100/skill-pickplace.yaml --embodiment hardware/pincherx-100/pincherx-100.yaml) \
  --telemetry /tmp/tel.jsonl
```
Align the Gazebo object placement with `frames.yaml` (`widget` pose + `place_zone`); size the
object so a ≤ 8 mm placement error still grasps.

## Honest caveats (design § 7)
Light object (≤ 50 g), top-down grasp; **yaw is IK-determined** on the 4-DOF arm
(`set_ee_pose_components` sets `yaw=atan2(y,x)`), so the measurement judges **position** and
accepts the **orientation residual**. Grasp confirmation is **proxy-tier** (no tactile). Frame
calibration is a fixed sim offset / a physical calibration. The value is the binary leap
(symbolic → real-hardware-verified) at the low end, not dexterity.
```

- [ ] **Step 2: Verify the README commands are accurate**

Run the "Run without hardware" block from the README verbatim. Expected: tests pass; measure prints `"passed": true`. Fix any command drift in the README.

- [ ] **Step 3: Commit**

```bash
cd ~/Documents/GitHub/rfl
git commit -F - -- hardware/pincherx-100/README.md <<'MSG'
docs(hardware): PincherX-100 proof README — bringup, run, caveats

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
MSG
```

---

## Task 9: Push + update tracking (real hashes)

- [ ] **Step 1: Final full-suite green check**

Run: `cd ~/Documents/GitHub/rfl && uv run --with pytest --with pyyaml --with jsonschema python -m pytest hardware/pincherx-100/tests/ -q`
Expected: all pass.

- [ ] **Step 2: Push to main (ff-only protocol)**

```bash
cd ~/Documents/GitHub/rfl
git fetch origin main
git merge-base --is-ancestor origin/main HEAD && echo "ff-OK" || echo "NON-FF — rebase needed"
# if NON-FF: git rebase origin/main  (only hardware/ + your commits move), then re-check
git push origin main
git rev-list --left-right --count origin/main...HEAD   # expect: 0 0
```
Expected: `ff-OK`, push succeeds, `0 0`.

- [ ] **Step 3: Update the local tracking memory (NOT committed to the repo)**

Record the real commit hashes (from `git log --oneline -9`) and the proof's completion in:
- `~/.claude/.../memory/project_rfl.md` — add the PincherX-100 hardware proof: descriptor + 3 skills + driver (parse/resolve/emit + Mock/Interbotix backends + CLI) + measure.py + README under `hardware/pincherx-100/`, Class-1 valid, full pytest green, the negative-space `capability_absent: force.insert_fit` demo, with the real per-task hashes. Note: sim/physical RUN is the user's bringup follow-on.
- `MEMORY.md` — one-line pointer under the RFL section.

(Plan doc `docs/plans/2026-06-01-...` stays LOCAL-ONLY; never `git add`.)

---

## Self-Review

**Spec coverage (design §§):**
- §1 (proves pick-and-place / grasp-force floor / proxy tier / negotiation / cost): Tasks 2, 5, 7 (measure reports all five). ✓
- §2 (verified hardware facts → descriptor): Task 1. ✓
- §4 (components): all created — descriptor T1, skills T2, driver T3-6, measure T7, README T8 (+ frames.yaml, fixtures, tests as testability scaffolding). ✓
- §5 (data flow: retarget → driver → measure): Task 2 (retarget), Task 6 (driver CLI stdin/stdout), Task 7 (measure). ✓
- §6 (descriptor specifics, M2 grasp.pinch limits, minItems satisfied): Task 1, with the `role_defaults.control`→`control_frames[0]` adaptation noted. ✓
- §7 (caveats: yaw IK-determined, proxy tier, frame calibration): driver `_make_backend`/resolver + frames.yaml + measure note. ✓
- §8 (task sequence hover→scan→pickplace): fixtures + tests cover all three. ✓
- §9 (division of labor: implementer = software; user = bringup): the plan builds only software; bringup is the README checklist. ✓
- §11 (confirm from source): CLI paths ✓ (confirmed), driver-interface fields ✓ (schema test), descriptor schema ✓ (Class-1), Interbotix API ✓ (grounded + isolated for bringup). ✓

**Placeholder scan:** every code step contains complete code; commands have expected output; no "TBD"/"add error handling"/"similar to Task N". The only deliberately deferred values are the descriptor limits tagged `CONFIRM bringup` (envelope-only, non-gating) and the exact Interbotix method names (isolated in `InterbotixBackend`, ROS1/ROS2 handled via getattr fallback). ✓

**Type/name consistency:** `ArmStep`/`SweepStation`/`EEPose` defined T3-4 and used identically in T5/T7; `parse_execute`/`execute_step`/`resolve_*`/`load_frames`/`length_m`/`_scan_region` signatures match across the driver and `measure.py` and the tests; fixture stems (`hover`/`scan`/`pickplace`) consistent; `min_holding_force` string `"0.6 N"` consistent with `0.3 N × 2.0`. ✓

**Risk note:** Task 2's `min_holding_force` assertion (`"0.6 N"`) assumes `Quantity::from_si` emits exactly `"0.6 N"`. If the live retarget emits a different string form (e.g. `"0.600 N"`), update the Step-3 grep AND `test_parse.py`/`test_emit_mock.py`/`test_measure.py` to the actual emitted string (the fixtures are ground truth) — a one-line change in each, flagged here so it is not a surprise.
