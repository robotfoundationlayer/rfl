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

`examples/01-cable-insertion/embodiments/{allegro,leap,pneumatic-6f}.yaml` validate against `embodiment-descriptor.schema.json` (verified, Draft 2020-12). They pin the format by demonstration across three structurally distinct hands, including the no-tactile pneumatic case — where the descriptor's `tactile` block and the `aux.tactile_sensing` key are simply absent, which is exactly what routes confirmation to the force/position proxy (`04`).

`examples/01-cable-insertion/skill.yaml` validates against `skill-isa.schema.json` (verified, Draft 2020-12). It exercises the compositional algebra: a `sequence` of primitive calls with two `let` bindings carrying `sense.locate` results downstream. The schema is **structural with incremental per-primitive typing** — it enforces the algebra shape (sequence / parallel / reactive / repeat / branch / let-bind), that every primitive call key is one of the 50 ISA ids, and the `ext.<ns>.<name>` form for extensions. Every one of the 50 primitives across all seven categories is now precisely typed: each constrains its parameter names / units / enums against its `01` parameter table — category 1 `reach` (§ 1.1–1.6), category 2 `grasp` (§ 2.1–2.10), category 3 `in_hand` (§ 3.1–3.7), category 4 `transport` (§ 4.1–4.6), category 5 `place` (§ 5.1–5.6), category 6 `force` (§ 6.1–6.10), category 7 `sense` (§ 7.1–7.5). The last two, `reach.retract` (§ 1.4) and `force.insert_fit` (§ 6.1), whose `01` tables diverged from the reference's task-level spelling, were reconciled by a pre-freeze formatting pass (`retract_axis` → `direction`; `axial_force_budget` → `force_budget`; `seating_condition: SeatingSpec` → `stop_condition: StopCondition`) and then typed; `force.insert_fit.stop_condition` uses a precise recursive `StopCondition` family def. The one piece of follow-up left is the other nine force primitives' StopCondition-family parameters (`pull.stop_condition`, `screw`/`unscrew.completion`, `press_button.actuation`, `cut.completion`, `scrub.completion`, `snap_engage.snap_signature`), still carried as a deferred `StopSpec` until their per-primitive § 6 prose spellings are normalized onto the family. Each primitive is tightened by one `if/then` clause in `PrimitiveCall.allOf` plus its `$defs` entry; a value may be a let-reference anywhere a typed parameter is expected, and `auto` is accepted only where the spec types the parameter as `T | auto`. The typing is checked against the spec by a cross-check that asserts each primitive's schema property set equals its `01` parameter-table column exactly (catching a dropped or fabricated parameter); the parameter table is the source of truth, not the reference instance.

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
