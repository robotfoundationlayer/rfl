# Schemas

JSON Schema definitions for the machine-readable RFL artifacts. They formalize the file formats the spec chapters describe and back conformance **test class 1** (parser conformance, `05-conformance.md`).

| Schema | Validates | Spec owner | Status |
|---|---|---|---|
| `embodiment-descriptor.schema.json` | the embodiment descriptor (frame model + capability manifest + limits + tactile / sensor descriptors) | `02` (format) · `03` · `04` | present |
| `skill-isa.schema.json` | a Skill ISA composition file | `01` | planned |
| `driver-interface/` | the canonical driver messages | `03` | planned |
| `tactile-manifold/` | per-sensor-class adapter mappings | `04` | planned |

## Conventions

- **Draft**: JSON Schema 2020-12.
- **Quantities** are unit-suffixed strings (`"20 N"`, `"0.05 m/s"`, `"60 deg"`, `"1.5 m/s^2"`) — the JSON / YAML equivalent of the spec's XML `value` + `unit` attributes. A `[min, max]` range is a two-element array of quantities.
- **Signed axes** are `"+z"` / `"-x"` etc.

## Reference instances

`examples/01-cable-insertion/embodiments/{allegro,leap,pneumatic-6f}.yaml` validate against `embodiment-descriptor.schema.json` (verified, Draft 2020-12). They pin the format by demonstration across three structurally distinct hands, including the no-tactile pneumatic case — where the descriptor's `tactile` block and the `aux.tactile_sensing` key are simply absent, which is exactly what routes confirmation to the force/position proxy (`04`).

## Validating

```bash
# Ephemeral environment, no project pollution:
uv run --with jsonschema --with pyyaml python - <<'PY'
import json, yaml, glob
from jsonschema import Draft202012Validator
schema = json.load(open("schemas/embodiment-descriptor.schema.json"))
Draft202012Validator.check_schema(schema)
v = Draft202012Validator(schema)
for f in glob.glob("examples/01-cable-insertion/embodiments/*.yaml"):
    v.validate(yaml.safe_load(open(f)))
    print("OK", f)
PY
```
