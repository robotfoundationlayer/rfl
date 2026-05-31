# Design: `rfl-cli` retarget v0 (cable-insertion path)

Status: approved design, pre-implementation (2026-05-31)

This document specifies the first implementation increment of the RFL reference
CLI: the `retarget(skill, embodiment) -> canonical_actions` engine, scoped to the
primitives exercised by `examples/01-cable-insertion`. The specification under
`spec/` is authoritative; this document describes how the reference
implementation realizes it, not new normative content.

## 1. Purpose and scope

`rfl retarget <skill.yaml> --embodiment <descriptor.yaml>` reads an
embodiment-agnostic Skill ISA composition (`spec/01`, validated by
`schemas/skill-isa.schema.json`) and an embodiment descriptor (`spec/03`,
validated by `schemas/embodiment-descriptor.schema.json`), and emits the
per-embodiment canonical action stream defined in `spec/02`
(`CanonicalAction` tuple), wrapped as driver-interface `execute` messages
(`spec/03` § Canonical driver messages, `schemas/driver-interface.schema.json`).

The increment makes `examples/01-cable-insertion/run.py` run on all three
embodiment descriptors (allegro / leap / pneumatic-6f) and makes conformance
test class 2 (retarget generation determinism, `spec/02` RD1c) runnable in CI.

Driver protocol execution (test class 3) and end-to-end execution (test class 4)
are out of scope for this increment.

## 2. Execution model

`retarget` is an ahead-of-time compilation, not a runtime executor. It resolves
everything that depends on the *embodiment* at generation time:

- clamping force budgets to the descriptor's declared grip limits,
- clamping motion bounds to the descriptor's declared kinematic limits,
- the `capability_absent` gate (`spec/03` § Capability checking),
- expanding `tactile_target: auto` and degrading it to the force/position proxy
  on an embodiment without `tactile_sensing` (`spec/04` § Graceful degradation),
- assembling the `Envelope` (`spec/02` § The Envelope).

Quantities that depend on *runtime measurement* (the pose returned by
`sense.locate`, an object's measured mass) are not invented at generation time.
They remain symbolic references in the emitted canonical action, threaded from
the skill's `let` bindings. The byte-determinism contract (RD1c) holds over this
embodiment-resolved, pose-symbolic output: identical `(skill, embodiment)` inputs
produce a byte-identical canonical action stream, symbolic references included.

A consequence worth stating early: much of the cable-insertion pose content is
runtime-resolved, so the geometric pose math (`pose.rs`) carries the types and
the `theta_orient` decision function but is only lightly exercised in v0. Numeric
pose resolution lands in a later increment.

## 3. v0 scope boundary

### In scope (what the cable-insertion skill exercises)

Algebra: `sequence`, `let` / `from` bindings (a symbol table threading a measured
pose forward), and the `objects:` reference table.

Seven primitives:
`sense.locate`, `grasp.pinch`, `transport.move_to_pose`, `reach.align`,
`force.insert_fit`, `grasp.release`, `reach.retract`.

Transformations the path requires:

- `capability_absent` gate: a primitive or grasp mode absent from the descriptor's
  capability manifest fails deterministically before any action is emitted.
- `tactile_target: auto` expansion to `normal_force >= epsilon at_least(2, antipodal)`
  on a tactile embodiment; degradation to the force/position proxy
  (`grasp_width` convergence and force-rise-and-hold) on the pneumatic embodiment,
  reported at `proxy` fidelity tier.
- `force_budget` clamping to descriptor constants (`grip_force_max`).
- `Envelope.motion_bounds` clamping to the descriptor's kinematic limits.
- `monitors` compilation: `force.insert_fit`'s `all_of{effort_rise, reached}`
  seating/jam condition; `reach.retract`'s force-monotonicity abort.
- under-constrained orientation: `reach.align(axes = [z])` emits the
  minimum-geodesic-rotation residual rule (`spec/02` CA2c) as a directive, since
  the current and target orientations are runtime quantities.
- `stop_at_goal = true` (the v0.1 rest-at-goal guarantee, CA3c).

### Deferred (with rationale)

- **Sweep generators `Sigma`** (`spec/02` Appendix A): `reach.scan` is not used by
  cable-insertion. Appendix A fixes the construction normatively, so this is a
  clean standalone module in a later increment.
- **Mass-dependent grasp-force numeric derivation** (the `m*g/(mu*geometry)`
  value of `min_holding_force`, the mass term of the dynamic-stability `a_max`):
  the cable-insertion skill does not declare the connector's mass (it is a runtime
  measurement). v0 numerically resolves (a) `force_budget` clamped to the descriptor
  constant `grip_force_max`, and (b) the transport acceleration clamped to the
  embodiment's kinematic ceiling `a_cartesian_max`, and leaves the mass-dependent
  tightening symbolic. The specification describes these derivations as schematic
  and provider-neutral (`spec/02` § Grasp-force and stability derivations), which
  is consistent with emitting the dependency rather than a fabricated number.
- **Capability-negotiation routing (RD4c), time-scaling (TG2c), multi-embodiment
  handoff (RD3c)**: none are on the cable-insertion path.
- **Driver protocol compliance (class 3) and end-to-end execution (class 4)**.

## 4. Architecture

The repository already scaffolds the workspace; this increment fills the existing
`// TODO` modules rather than restructuring.

```
rfl-core (lib)
  lib.rs          existing Error / Result / SPEC_VERSION, kept
  skill_isa.rs    [extend]  input AST + serde_yaml parser:
                            Skill, Statement(Primitive | LetBind), and the typed
                            parameters of the seven primitives
  embodiment.rs   [new]     input descriptor: limits, capabilities, frames,
                            sensors, tactile
  pose.rs         [new]     Pose6D (position + UnitQuaternion), theta_orient,
                            under-constrained residual rule (nalgebra)
  canonical.rs    [new]     output model: CanonicalAction, Envelope, Monitor,
                            TimingHints, and canonical-JSON serialization
  translation.rs  [fill]    pub fn retarget(&Skill, &Embodiment)
                            -> Result<Vec<CanonicalAction>>; per-statement
                            lowering dispatch (the engine)

rfl-cli (bin `rfl`)
  main.rs         [fill]    the existing Retarget subcommand arm: load skill,
                            load embodiment, retarget, emit execute messages as
                            JSONL on stdout

rfl-conformance (lib)
  lib.rs          [fill]    test class 2: insta golden per embodiment,
                            generate-twice byte-equality property, per-line
                            execute-schema validation
```

Each file has a single responsibility: `skill_isa` and `embodiment` are the two
inputs, `pose` is the geometry, `canonical` is the output model, `translation` is
the transformation. If `translation.rs` grows past a comfortable size, the
per-primitive lowering moves to a `retarget.rs` sibling; the public entry point
stays `translation::retarget`.

### Quantities as strings

Canonical actions carry physical quantities as unit-suffixed strings (`"8 N"`,
`"50 mm"`), matching the `Force` / `Length` patterns the schemas already define.
Clamping parses the magnitude to compare, then emits a controlled string. This
removes float-formatting nondeterminism from the output by construction and is the
primary determinism lever (see section 6). `nalgebra` is used for pose geometry
only.

## 5. Retarget pipeline

1. **Parse skill.** `skill_isa::parse(yaml) -> Skill`: the `objects` table, the
   `sequence` body, the `let` / `from` bindings, and typed primitive calls. The
   precise per-primitive parameter set is taken from `schemas/skill-isa.schema.json`
   (the typed `$defs` are the precise source), not from memory.
2. **Parse embodiment.** `embodiment::parse(yaml) -> Embodiment`: the capability
   manifest, the flat `limits` namespace, the frame model, the sensor and tactile
   descriptors.
3. **Lower each statement.** A symbol table threads `let` bindings. For each
   primitive:
   - the `capability_absent` gate checks the descriptor's manifest and fails
     deterministically if the primitive (or grasp mode) is absent;
   - the parameters compile into `CanonicalAction` fields, applying the
     embodiment-dependent resolution of section 3, carrying quantities as strings;
   - one statement lowers to zero or more canonical actions. In v0 the mapping is
     mostly one-to-one; `sense.locate` lowers to a perception action that binds its
     `let` variable, threading a symbolic pose reference forward.
4. **Emit.** Each `CanonicalAction` is wrapped as
   `ExecuteGoal { message: "execute", action_id, canonical_action }` and written as
   one JSON object per line (JSONL) on stdout.

## 6. Determinism strategy (RD1c)

Three mechanisms together guarantee byte-identical generation:

- **Quantities are strings.** No float is formatted into the output; magnitudes are
  compared during clamping but the emitted value is a controlled unit-suffixed
  string.
- **Keys are sorted.** Canonical JSON serializes object keys in a fixed
  (sorted) order; array order follows the order the specification fixes.
- **A generate-twice property test.** `retarget` is invoked twice on the same input
  and the two byte streams are asserted equal. This verifies RD1c directly, without
  a fixture, and `proptest` varies the embodiment to widen coverage.

## 7. Output format and conformance fixtures

The output is a JSONL stream of `execute` messages. Test class 2 pins it three
ways:

- **Golden regression (insta).** `insta::assert_snapshot!` stores the JSONL for
  each embodiment (allegro / leap / pneumatic-6f) as a committed snapshot, compared
  byte-for-byte in CI. `cargo insta review` is the diff-approval workflow when a
  deliberate change lands. `insta` is already a workspace dev-dependency.
- **Schema validation.** Each emitted line is validated against the `execute`
  variant of `schemas/driver-interface.schema.json`. The `canonical_action` field
  is an open floor in that schema (its representation is owned by `spec/02`), so
  this is a structural gate over the message envelope; the precise canonical-action
  shape is pinned by the golden snapshot. A Rust JSON Schema validator is added as a
  `rfl-conformance` dev-dependency; the Draft 2020-12 support of the candidate crates
  (`boon`, `jsonschema`) is verified against current documentation at implementation
  time before pinning one.
- **Determinism property.** The generate-twice test of section 6.

The existing `schemas/validate.py` (conformance test class 1, invariants C1 through
C7) is unchanged: it validates the input schema layer, a separate concern from
retarget output.

## 8. Testing strategy

- `rfl-core` unit tests cover the parsers, `theta_orient`, the residual-orientation
  rule, and each primitive's lowering.
- `rfl-conformance` carries the three class-2 checks above.
- The Python input-schema suite (`uv run --with jsonschema --with pyyaml python
  schemas/validate.py`) stays green.
- Test results are read and confirmed in a step separate from any commit, so a
  failing check is never committed.

## 9. Deferred work and authoritative spec references

Deferred to later increments: the `Sigma` sweep generators (`spec/02` Appendix A),
mass-dependent grasp-force numeric derivation, capability-negotiation routing
(RD4c), time-scaling (TG2c), multi-embodiment handoff (RD3c), and conformance test
classes 3 and 4.

Sections to transcribe from the specification during implementation (rather than
work from memory):

- `spec/01` skill-isa primitive parameter tables and the sequence / let-bind
  algebra; the typed `$defs` of `schemas/skill-isa.schema.json` are the precise
  source for parameter sets.
- `spec/03` § Capability checking and § Capability manifest (the exact
  `capability_absent` decision); § Reach baseline limits and § Grasp-mode
  capabilities and limits (the clamp targets).
- `spec/04` § The TactileTarget type and § Graceful degradation (the `auto`
  expansion and the proxy degradation rule).
- `spec/02` (already transcribed): the `CanonicalAction` tuple, the `Envelope`,
  `Pose6D`, the geodesic orientation error, and the under-constrained residual
  rule.
