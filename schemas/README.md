# Schemas

JSON Schema definitions for the machine-readable RFL artifacts. They formalize the file formats the spec chapters describe and back conformance **test class 1** (parser conformance, `05-conformance.md`).

| Schema | Validates | Spec owner | Status |
|---|---|---|---|
| `embodiment-descriptor.schema.json` | the embodiment descriptor (frame model + capability manifest + limits + tactile / sensor descriptors) | `02` (format) · `03` · `04` | present |
| `skill-isa.schema.json` | a Skill ISA composition file | `01` | present |
| `driver-interface/` | the canonical driver messages | `03` | planned |
| `tactile-manifold/` | per-sensor-class adapter mappings | `04` | planned |

## Conventions

- **Draft**: JSON Schema 2020-12.
- **Quantities** are unit-suffixed strings (`"20 N"`, `"0.05 m/s"`, `"60 deg"`, `"1.5 m/s^2"`) — the JSON / YAML equivalent of the spec's XML `value` + `unit` attributes. A `[min, max]` range is a two-element array of quantities.
- **Signed axes** are `"+z"` / `"-x"` etc.

## Reference instances

`examples/01-cable-insertion/embodiments/{allegro,leap,pneumatic-6f}.yaml` validate against `embodiment-descriptor.schema.json` (verified, Draft 2020-12). They pin the format by demonstration across three structurally distinct hands, including the no-tactile pneumatic case — where the descriptor's `tactile` block and the `aux.tactile_sensing` key are simply absent, which is exactly what routes confirmation to the force/position proxy (`04`). The descriptor is not merely structural. The capability manifest's `skills` are validated against the exact set of Skill ISA capability keys (`skill-isa.schema.json`'s `PrimitiveId` enum minus the baseline `reach.*`, plus the four category gates `transport` / `in_hand` / `force` / `sense`, or an `ext.<ns>.<name>` key), so an unknown skill id is rejected. The `aux` block types each known auxiliary (`compliance` is a set or a scalar shorthand over `{passive, active, virtual}`, `tool_safety` and `human_collaboration_safety` carry their `03 §` structures, `coordination_channel` / `tactile_sensing` are booleans), with namespace-prefixed unknown keys still permitted. Limit-completeness (M2) is enforced as `if/then` clauses on the embodiment: the reach baseline (`v_cartesian_max`, `w_cartesian_max`, `a_cartesian_max`, `joint_velocity_ceiling`, `stop_time`, `tracking_bandwidth`) is required of every embodiment, and asserting a capability requires its `03 §` table limits (e.g. `grasp.envelope` needs `payload_grasp_envelope` and the `enclosure_range` range; `grasp.pin` needs no payload because the external surface bears the load; `transport.carry` needs `stability_margin`; `force.scrub` needs `normal_compliance_range` and `oscillation_max`; `in_hand.rotate` / `translate` carry the grasp-mode-keyed range maps `inhand_rotation_range` / `inhand_translation_range`). Place and sense add no new limit (PFS4c), so M2 spans all seven categories. The three reference descriptors declare every limit their capabilities require, and a cross-check derives the valid key set from `skill-isa.schema.json` and asserts the descriptor's capability enum and per-capability required-limit sets equal the `03` tables exactly.

`examples/01-cable-insertion/skill.yaml` validates against `skill-isa.schema.json` (verified, Draft 2020-12). It exercises the compositional algebra: a `sequence` of primitive calls with two `let` bindings carrying `sense.locate` results downstream. The schema is **structural with incremental per-primitive typing** — it enforces the algebra shape (sequence / parallel / reactive / repeat / branch / let-bind), that every primitive call key is one of the 50 ISA ids, and the `ext.<ns>.<name>` form for extensions. Every one of the 50 primitives across all seven categories is now precisely typed: each constrains its parameter names / units / enums against its `01` parameter table — category 1 `reach` (§ 1.1–1.6), category 2 `grasp` (§ 2.1–2.10), category 3 `in_hand` (§ 3.1–3.7), category 4 `transport` (§ 4.1–4.6), category 5 `place` (§ 5.1–5.6), category 6 `force` (§ 6.1–6.10), category 7 `sense` (§ 7.1–7.5). `reach.retract` (§ 1.4) and `force.insert_fit` (§ 6.1), whose `01` tables diverged from the reference's task-level spelling, were reconciled by a pre-freeze formatting pass (`retract_axis` → `direction`; `axial_force_budget` → `force_budget`; `seating_condition: SeatingSpec` → `stop_condition: StopCondition`). The `StopCondition` family (`01` § Stop / completion conditions) is typed precisely, not deferred: eleven shared member building blocks (`SC_reached_depth` / `SC_reached_distance` / `SC_reached_advance`, `SC_effort_rise_force` / `SC_effort_rise_torque`, `SC_effort_drop`, `SC_detent`, `SC_count`, `SC_elapsed`, `SC_landmark`, `SC_predicate`) compose into the per-primitive restriction subtypes of `01` § 315–321 — `SeatingSpec` (insert_fit), `PullStop` (pull), `ScrewStop` (screw / unscrew), `ActuationSpec` (press_button / snap_engage), `CutStop` (cut), `ScrubStop` (scrub), `SlideStop` (in_hand.slide) — so each stop / completion parameter accepts only the family members its primitive allows (e.g. `pull.stop_condition` rejects `detent`; `screw.completion` requires a Torque effort, not a Force; `press_button.actuation` is `detent | effort_rise(force)` only). Per a project decision the `unscrew` `effort_drop` variant noted in § 318 is not typed; screw and unscrew share one `ScrewStop`. Each primitive is tightened by one `if/then` clause in `PrimitiveCall.allOf` plus its `$defs` entry; a value may be a let-reference anywhere a typed parameter is expected, and `auto` is accepted only where the spec types the parameter as `T | auto`. The typing is checked against the spec by a cross-check that asserts each primitive's schema property set equals its `01` parameter-table column exactly (catching a dropped or fabricated parameter); the parameter table is the source of truth, not the reference instance.

## Validating

```bash
# Ephemeral environment, no project pollution:
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
