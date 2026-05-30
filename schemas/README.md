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

`examples/01-cable-insertion/skill.yaml` validates against `skill-isa.schema.json` (verified, Draft 2020-12). It exercises the compositional algebra: a `sequence` of primitive calls with two `let` bindings carrying `sense.locate` results downstream. The schema is **structural with incremental per-primitive typing** — it enforces the algebra shape (sequence / parallel / reactive / repeat / branch / let-bind), that every primitive call key is one of the 50 ISA ids, and the `ext.<ns>.<name>` form for extensions. Each primitive's parameter block is an open object by default; primitives precisely typed so far constrain their parameter names / units / enums against the `01` parameter table (currently `grasp.pinch`, `01` § 2.1). Each new primitive is tightened by adding one `if/then` clause to `PrimitiveCall.allOf` plus its `$defs` entry, leaving the already-typed primitives untouched. A value may be a let-reference anywhere a typed parameter is expected, and `auto` is accepted only where the spec types the parameter as `T | auto`. The `StopCondition` / `Predicate` families and the remaining 49 primitives are the continuing follow-up.

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
