# Schemas

JSON Schema definitions for the machine-readable RFL artifacts. They formalize the file formats the spec chapters describe and back conformance **test class 1** (parser conformance, `05-conformance.md`).

| Schema | Validates | Spec owner | Status |
|---|---|---|---|
| `embodiment-descriptor.schema.json` | the embodiment descriptor (frame model + capability manifest + limits + tactile / sensor descriptors) | `02` (format) · `03` · `04` | present |
| `skill-isa.schema.json` | a Skill ISA composition file | `01` | present |
| `driver-interface.schema.json` | the canonical driver messages (execute / telemetry / status / clearance-query) | `03` | present |
| `tactile-manifold/` | per-sensor-class adapter mappings (`adapter.schema.json` + ft / array / visuotactile) | `04` | present |

## Conventions

- **Draft**: JSON Schema 2020-12.
- **Quantities** are unit-suffixed strings (`"20 N"`, `"0.05 m/s"`, `"60 deg"`, `"1.5 m/s^2"`) — the JSON / YAML equivalent of the spec's XML `value` + `unit` attributes. A `[min, max]` range is a two-element array of quantities.
- **Signed axes** are `"+z"` / `"-x"` etc.

## Reference instances

`examples/01-cable-insertion/embodiments/{allegro,leap,pneumatic-6f}.yaml` validate against `embodiment-descriptor.schema.json` (verified, Draft 2020-12). They pin the format by demonstration across three structurally distinct hands, including the no-tactile pneumatic case — where the descriptor's `tactile` block and the `aux.tactile_sensing` key are simply absent, which is exactly what routes confirmation to the force/position proxy (`04`). The descriptor is not merely structural. The capability manifest's `skills` are validated against the exact set of Skill ISA capability keys (`skill-isa.schema.json`'s `PrimitiveId` enum minus the baseline `reach.*`, plus the four category gates `transport` / `in_hand` / `force` / `sense`, or an `ext.<ns>.<name>` key), so an unknown skill id is rejected. The `aux` block types each known auxiliary (`compliance` is a set or a scalar shorthand over `{passive, active, virtual}`, `tool_safety` and `human_collaboration_safety` carry their `03 §` structures, `coordination_channel` / `tactile_sensing` are booleans), with namespace-prefixed unknown keys still permitted. Limit-completeness (M2) is enforced as `if/then` clauses on the embodiment: the reach baseline (`v_cartesian_max`, `w_cartesian_max`, `a_cartesian_max`, `joint_velocity_ceiling`, `stop_time`, `tracking_bandwidth`) is required of every embodiment, and asserting a capability requires its `03 §` table limits (e.g. `grasp.envelope` needs `payload_grasp_envelope` and the `enclosure_range` range; `grasp.pin` needs no payload because the external surface bears the load; `transport.carry` needs `stability_margin`; `force.scrub` needs `normal_compliance_range` and `oscillation_max`; `in_hand.rotate` / `translate` carry the grasp-mode-keyed range maps `inhand_rotation_range` / `inhand_translation_range`). Place and sense add no new limit (PFS4c), so M2 spans all seven categories. The three reference descriptors declare every limit their capabilities require, and a cross-check derives the valid key set from `skill-isa.schema.json` and asserts the descriptor's capability enum and per-capability required-limit sets equal the `03` tables exactly.

`examples/01-cable-insertion/skill.yaml` validates against `skill-isa.schema.json` (verified, Draft 2020-12). It exercises the compositional algebra: a `sequence` of primitive calls with two `let` bindings carrying `sense.locate` results downstream. The schema is **structural with incremental per-primitive typing** — it enforces the algebra shape (sequence / parallel / reactive / repeat / branch / let-bind), that every primitive call key is one of the 50 ISA ids, and the `ext.<ns>.<name>` form for extensions. Every one of the 50 primitives across all seven categories is now precisely typed: each constrains its parameter names / units / enums against its `01` parameter table — category 1 `reach` (§ 1.1–1.6), category 2 `grasp` (§ 2.1–2.10), category 3 `in_hand` (§ 3.1–3.7), category 4 `transport` (§ 4.1–4.6), category 5 `place` (§ 5.1–5.6), category 6 `force` (§ 6.1–6.10), category 7 `sense` (§ 7.1–7.5). `reach.retract` (§ 1.4) and `force.insert_fit` (§ 6.1), whose `01` tables diverged from the reference's task-level spelling, were reconciled by a pre-freeze formatting pass (`retract_axis` → `direction`; `axial_force_budget` → `force_budget`; `seating_condition: SeatingSpec` → `stop_condition: StopCondition`). The `StopCondition` family (`01` § Stop / completion conditions) is typed precisely, not deferred: eleven shared member building blocks (`SC_reached_depth` / `SC_reached_distance` / `SC_reached_advance`, `SC_effort_rise_force` / `SC_effort_rise_torque`, `SC_effort_drop`, `SC_detent`, `SC_count`, `SC_elapsed`, `SC_landmark`, `SC_predicate`) compose into the per-primitive restriction subtypes of `01` § 315–321 — `SeatingSpec` (insert_fit), `PullStop` (pull), `ScrewStop` (screw / unscrew), `ActuationSpec` (press_button / snap_engage), `CutStop` (cut), `ScrubStop` (scrub), `SlideStop` (in_hand.slide) — so each stop / completion parameter accepts only the family members its primitive allows (e.g. `pull.stop_condition` rejects `detent`; `screw.completion` requires a Torque effort, not a Force; `press_button.actuation` is `detent | effort_rise(force)` only). Per a project decision the `unscrew` `effort_drop` variant noted in § 318 is not typed; screw and unscrew share one `ScrewStop`. Each primitive is tightened by one `if/then` clause in `PrimitiveCall.allOf` plus its `$defs` entry; a value may be a let-reference anywhere a typed parameter is expected, and `auto` is accepted only where the spec types the parameter as `T | auto`. The typing is checked against the spec by a cross-check that asserts each primitive's schema property set equals its `01` parameter-table column exactly (catching a dropped or fabricated parameter); the parameter table is the source of truth, not the reference instance. One compound type is typed beyond the open-object floor: `tactile_target` (carried by the nine grasp params) is now the precise `TactileTarget` of `04` § The TactileTarget type — `all_of` a non-empty set of clauses, each a `(feature, comparator, threshold, quantifier)` where `feature` is one of the eleven closed-core TactileManifold features or an `ext.<ns>.<name>` extension feature, `comparator` ∈ `{>=, <=, ==, within}`, and `quantifier` is `at_least(n, role)` / `all(role)` / `exactly(n, role)` over a site role ∈ `{antipodal, enclosure, support, probe}` — with `auto` and the let-reference form still accepted (`TactileTargetOrAuto`). `SurfaceTarget` (the `reach.approach` / `reach.hover` `target`, `grasp.pin.against_surface`, and the `place.put_down` / `orient` `target_surface`) is likewise typed from `01` § Target types — `point` (a `Pose6D` position, its representation deferred to `02` so it stays the object/array/reference floor) + outward `normal: Direction` + `frame: FrameRef` + optional `uncertainty: UncertaintyBound` — with the `auto` and let-reference forms carried by `SurfaceTargetOrAuto` (and a bare `FrameRef` reaching a hover target as a reference). `ObjectTarget` (the eight grasp `target`s, `place.stack.support_object`, `place.insert_loose.container`, and one arm of `sense.inspect.target`) is typed from `01` § Target types too — `pose` (a `Pose6D`) required, plus optional `geometry: GeometryRef`, `estimated_mass: Force`, `center_of_mass` (a `Pose6D` position), `max_contact_force: Force?` (fragility), `features: set<FeatureRef>`, and `uncertainty: UncertaintyBound` — with the let-reference form accepted wherever an ObjectTarget is expected. The remaining compound type (ContactConfig) stays the open-object floor, a deeper increment; `Pose6D` / `GeometryRef` / `FeatureRef` stay the representation floor by design (their representation is `02`'s / perception's). The `force.*` surface targets (`push` / `press_button` `target`, `wipe` / `scrub` `surface`), verified against their own `§ 6` tables as `SurfaceTarget`, are typed to `SurfaceTarget` as well.

`examples/01-cable-insertion/driver-messages/{execute,telemetry,status,clearance-request,clearance-response}.yaml` validate against `driver-interface.schema.json`. They pin the runtime message contract by demonstration on the cable-insertion task. The `execute` goal carries one retargeted `02` `CanonicalAction` (its representation-owned fields — `Pose6D`, `Envelope`, `TactileTarget`, `Monitor` — floored, so the schema does not re-encode a representation it does not own). `telemetry` is stamped on the `04` manifold timebase and carries the realized pose, wrench, securing force, manifold feature readings (closed-core feature keys, anti-drift C4), and fidelity tier the `05` envelope classes interval-sample. `status` carries the three-valued outcome, the ownership-split failure classification (a `03`-owned protocol `failure_class` enum vs. an optional `01`-owned `failure_detail` token whose vocabulary `03` does not re-enumerate), and the `05` audit record (`Verdict`, fidelity tier, `momentary_release` / freed-part disposition). The clearance request / response formalize the `03` § Collision model query as a service. A message is exactly one of the five, discriminated by `message` (top-level `oneOf`).

`schemas/tactile-manifold/{ft,array,visuotactile}.yaml` are the canonical per-sensor-class adapters, validated against `schemas/tactile-manifold/adapter.schema.json`. Each declares the raw → manifold-feature mapping contract for its class — `raw_signal` (the native channels), `feature_production` (which closed-core features each channel produces, by `direct` / `derived` / `aggregated` kind, never the numeric algorithm), `site_model` (how the raw layout resolves into abstract sites and relational roles), and an optional `resolution_envelope` (`04` § The adapter mapping). An embodiment binds to its adapter through the descriptor's `tactile[frame].sensor_class`: the LEAP descriptor binds `ft`, the Allegro `array`. Anti-drift C5 checks the adapter's feature core is identical to `skill-isa`'s, and C6 checks each bound frame's declared features are a subset of its adapter's produced features.

## Validating

`validate.py` is the committed conformance-test-class-1 runner: it checks the
four schemas (Draft 2020-12), validates every reference instance against them,
and asserts the cross-schema consistency invariants (the descriptor's capability
enum is exactly `skill-isa`'s `PrimitiveId` set minus `reach.*` plus the four
category gates; the extension-key pattern is shared; the closed-core tactile
feature set is identical across `skill-isa`, the descriptor, the
`driver-interface` telemetry feature, and the `tactile-manifold` adapter
feature, with the extension-feature pattern identical across all four; and each
reference embodiment's declared tactile features are a subset of its bound
adapter's produced features). It exits non-zero on any failure.

```bash
# Ephemeral environment, no project pollution:
uv run --with jsonschema --with pyyaml python schemas/validate.py
```

The equivalent inline form, for reference:

```bash
uv run --with jsonschema --with pyyaml python - <<'PY'
import json, yaml, glob
from jsonschema import Draft202012Validator

# Embodiment descriptors
schema = json.load(open("schemas/embodiment-descriptor.schema.json"))
Draft202012Validator.check_schema(schema)
v = Draft202012Validator(schema)
for f in glob.glob("examples/01-cable-insertion/embodiments/*.yaml"):
    v.validate(yaml.safe_load(open(f)))
    print("OK", f)

# Skill ISA composition
sk = json.load(open("schemas/skill-isa.schema.json"))
Draft202012Validator.check_schema(sk)
Draft202012Validator(sk).validate(yaml.safe_load(open("examples/01-cable-insertion/skill.yaml")))
print("OK", "examples/01-cable-insertion/skill.yaml")
PY
```
