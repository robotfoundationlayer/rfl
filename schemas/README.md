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

`examples/01-cable-insertion/skill.yaml` validates against `skill-isa.schema.json` (verified, Draft 2020-12). It exercises the compositional algebra: a `sequence` of primitive calls with two `let` bindings carrying `sense.locate` results downstream. The schema is **structural with incremental per-primitive typing** — it enforces the algebra shape (sequence / parallel / reactive / repeat / branch / let-bind), that every primitive call key is one of the 50 ISA ids, and the `ext.<ns>.<name>` form for extensions. Each primitive's parameter block is an open object by default; primitives precisely typed so far constrain their parameter names / units / enums against the `01` parameter table. Typed so far: the whole of category 1 `reach` except the deferred `reach.retract` (`reach.to_pose` § 1.1, `reach.approach` § 1.2, `reach.align` § 1.3, `reach.hover` § 1.5, `reach.scan` § 1.6), the whole of category 2 `grasp` (`grasp.pinch` § 2.1 through `grasp.release` § 2.10 — all ten), the whole of category 3 `in_hand` (`in_hand.rotate` § 3.1, `in_hand.translate` § 3.2, `in_hand.regrasp` § 3.3, `in_hand.roll` § 3.4, `in_hand.pivot` § 3.5, `in_hand.slide` § 3.6, `in_hand.flip` § 3.7 — all seven), the whole of category 5 `place` (`place.put_down` § 5.1, `place.stack` § 5.2, `place.insert_loose` § 5.3, `place.orient` § 5.4, `place.hand_to` § 5.5, `place.discard` § 5.6 — all six), the whole of category 4 `transport` (`transport.move_to_pose` § 4.1, `transport.follow_trajectory` § 4.2, `transport.handoff` § 4.3, `transport.carry` § 4.4, `transport.lift` § 4.5, `transport.lower` § 4.6 — all six), nine of category 6 `force` (`force.push` § 6.2 through `force.snap_engage` § 6.10 — all except `force.insert_fit`; their StopCondition-family parameters, e.g. `pull.stop_condition` and `screw.completion`, are kept as a deferred `StopSpec`), and the whole of category 7 `sense` (`sense.probe` § 7.1, `sense.inspect` § 7.2, `sense.weigh` § 7.3, `sense.locate` § 7.4, `sense.verify` § 7.5) — 48 primitives in all. The only primitives left open are `force.insert_fit` § 6.1 and `reach.retract` § 1.4, both pending the pre-freeze `01` § 6 formatting pass that aligns their parameter spelling (the `StopCondition`/`SeatingSpec` family and `retract_axis`) with the reference instance. This covers every primitive the reference instance exercises except the two whose `01` tables still diverge from the reference's task-level spelling, `reach.retract` and `force.insert_fit`, which stay open pending the pre-freeze formatting pass `01` § 6 flags. The typing is checked against the spec by a cross-check that asserts each typed primitive's schema property set equals its `01` parameter-table column exactly (catching a dropped or fabricated parameter); the parameter table is the source of truth, not the reference instance. Each new primitive is tightened by adding one `if/then` clause to `PrimitiveCall.allOf` plus its `$defs` entry, leaving the already-typed primitives untouched. A value may be a let-reference anywhere a typed parameter is expected, and `auto` is accepted only where the spec types the parameter as `T | auto`. The `StopCondition` / `Predicate` families and the remaining primitives are the continuing follow-up.

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
