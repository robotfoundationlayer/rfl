# Grasp-mode breadth — `power` / `lateral` / `hook` / `envelope` / `precision_tripod`

**Status**: design-complete, 2026-06-01. Implementation track wave 1 (primitive breadth, no new
obligation dimension).

## Goal

Complete the `grasp` closure-classification by adding the remaining `spec/01` § 2 modes the
Translation Layer can lower. Today v0 lowers `grasp.pinch` (2.1), `grasp.platform` (2.6),
`grasp.pin` (2.7) — three of ten. This wave adds five more: `power` (2.2), `hook` (2.3),
`precision_tripod` (2.4), `lateral` (2.5), `envelope` (2.8, both `conform` and `cage` sub-modes).
`grasp.adjust` (2.9) is a held→held modifier (deferred, separate increment); `grasp.release`
(2.10) already exists.

This is *breadth*, not a new obligation dimension. The closure-driven conformance machinery
(`StabilityMetadata::for_mode` → hold-test profile, envelope class, accel-clamp, STB3) already
handles every closure type; each mode is a new row in that table plus a lowering function. The one
genuinely-new exercise is **`grasp.hook` is the first Form-closure grasp**, so it is the first
primitive to drive `ReferenceDriver`'s `Closure::Form → "load_direction"` hold-test branch and
`check_hold_test`'s Form expectation — both already written, never before exercised end-to-end.

## Closure classification (`spec/01` § Grasp state model table, verbatim)

| mode | closure | secured_dof | stable_directions | flags | residual_mobility |
|---|---|---|---|---|---|
| power | force | `all_axes: friction_held` | omnidirectional | — | — |
| precision_tripod | force | `all_axes: friction_held` | omnidirectional | `rotation_constrained` | — |
| lateral | force | `clamp_normal: friction_held`, `in_plane: friction_held` | omnidirectional | — | — |
| hook | form | `load_direction: form_held` | `{load_direction}` | — | — |
| envelope_conform | form | `enclosure: friction_held` | omnidirectional | `compliant` | — |
| envelope_cage | form | `enclosure: friction_held` | omnidirectional | — | `cage_clearance` |

`StabilityMetadata::for_mode` is the single source of truth. The two `envelope` sub-modes map to
two `GraspMode` variants (`EnvelopeConform` / `EnvelopeCage`) so the `mode` parameter discriminates
at lowering, matching the spec's distinct postconditions.

## GraspMode additions (`grasp_force.rs`)

New variants: `Power`, `Lateral`, `Hook`, `PrecisionTripod`, `EnvelopeConform`, `EnvelopeCage`.

- `k_holding` / `k_reaction`: the force-closure modes (power, lateral, tripod) reuse the 2.0
  schematic factor (same `1/(2μ)` basis as pinch). Hook is form-closure (no grip-per-weight
  squeeze) — its retention is geometric, so `k_holding` is N/A (1.0, present for the exhaustive
  match, unused: hook emits no `min_holding_force`). Envelope modes are gentle distributed
  friction; reuse 2.0 (conservative).
- `payload_key`: `payload_grasp_power` / `payload_grasp_lateral` / `payload_grasp_tripod` /
  `payload_grasp_envelope`. Hook uses `hook_load_capacity` (its load rating, not a grip payload).

## Lowering

The four **force-closure** modes (power, lateral, tripod) lower identically to pinch — clamp
`force_budget` to `grip_force_max`, proxy-tier tactile criterion when no tactile sensing,
`min_holding_force` from the held weight, `ctx.held` set with the mode. Extract a shared
`force_closure_grasp_action` helper they all call (pinch adopts it too, byte-identically — verified
against the existing goldens). Per-mode deltas: the `GraspMode`, the proxy criterion string, and
(tripod) the `rotation_constrained` flag rides `StabilityMetadata::for_mode`.

`grasp.hook` is **form-closure, no squeeze**: it carries `load_budget` (not `force_budget`),
clamped to `hook_load_capacity`; no `min_holding_force` (directional retention, like `pin`), no
`ctx.held` grip load. The `load_direction` rides `force_profile` as a symbolic marker. Emits
`grasp_stability = for_mode(Hook)` (closure: form) — which makes `ReferenceDriver` emit
`hold_test:load_direction` and `check_hold_test` expect it. This is the wave's load-bearing
moment.

`grasp.envelope` lowers per `mode`: `conform` → `EnvelopeConform` (compliant flag), `cage` →
`EnvelopeCage` (`residual_mobility = cage_clearance`, carried symbolic). Both form-closure, gentle
`force_budget` clamped to `grip_force_max`. Crush-protection is the primary safety note; v0 carries
it on `force_profile`.

## Capability gate

Each mode keys on its dotted identifier: `grasp.power` / `grasp.hook` / `grasp.precision_tripod` /
`grasp.lateral` / `grasp.envelope` (`envelope` covers both sub-modes — one capability). The
descriptors already declare `grasp.lateral` (allegro, pneumatic) and `grasp.envelope` (allegro,
leap); `grasp.power` / `grasp.hook` / `grasp.precision_tripod` are added to descriptors as worked
examples need them, with their backing limits (`payload_grasp_*`, `enclosure_span`,
`hook_load_capacity`).

## Envelope class

Every new grasp suffix is `GraspContinuity` (ENV1): `power` / `lateral` / `hook` / `tripod` /
`conform` / `cage`. Added to `envelope_class_for`. Action-id suffixes: `power`, `lateral`, `hook`,
`tripod`, `conform`, `cage`.

## Worked examples + tests

A worked-example skill per mode under `examples/03-screw-fasten/skill-<mode>.yaml`, each a minimal
`locate → grasp.<mode> → grasp.release`, exercising hold-test (GC2) and the stability metadata.
Conformance test files mirror `pivot.rs` (3 insta goldens + byte-determinism + boon schema
validity per line). The hook test additionally asserts (in `battery.rs`) that the Form-closure
hold-test passes nominally and a wrong-profile driver fails — the first Form `check_hold_test`
exercise.

## Deferred / blocked

- **STB1 (tripod triangle-area non-degeneracy)** — needs the ε-table + un-reported contact
  geometry (3 contact points). Blocked: wire-format invention. Tripod ships the
  `rotation_constrained` flag only, no area check.
- `grasp.adjust` (held→held) — separate increment.
- `enclosure_completeness` / `cage_clearance: auto` derivations — no normative formula (the
  `coverage_overlap: auto` posture); v0 requires explicit or carries symbolic.
- Hook `seating_force` ≤ `max_contact_force` numeric guard — symbolic in v0.
