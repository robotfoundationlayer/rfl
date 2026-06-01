# Authoring a Skill ISA composition

A Skill ISA file describes a manipulation task **embodiment-agnostically**: no
frame, joint, or sensor of any specific robot is named. Retargeting resolves the
task against a concrete embodiment descriptor at run time. This guide walks
through writing one; the normative reference is
[`spec/01-skill-isa.md`](../spec/01-skill-isa.md), and the machine-checkable
form is [`schemas/skill-isa.schema.json`](../schemas/skill-isa.schema.json).

The worked example throughout is
[`examples/01-cable-insertion/skill.yaml`](../examples/01-cable-insertion/skill.yaml).

----

## File shape

```yaml
skill: cable-insertion          # the skill id (kebab-case)
description: >                  # human-readable intent
  Grasp a connector and insert it under force control …

objects:                        # task object references (see § Objects)
  connector:  { ref: connector, estimated_mass: 1.45 N }
  receptacle: { ref: receptacle }

body:
  sequence:                     # the compositional algebra (see § The algebra)
    - <statement>
    - <statement>
```

`skill`, `description`, and `body` are required; `objects` is conventional for
any task that references task objects.

## The algebra

`body` is one composition node. The five forms (`spec/01` § Composition algebra):

| Form | Shape | Use |
|---|---|---|
| `sequence` | `sequence: [stmt, …]` | run statements in order |
| `parallel` | `parallel: [stmt, …]` | run concurrently (joined) |
| `reactive` | `reactive: { … }` | guarded / interrupt-driven behavior |
| `repeat` | `repeat: { … }` | bounded repetition |
| `branch` | `branch: { … }` | conditional on a predicate |

A statement is either a **primitive call** (`category.primitive: { params }`) or
a **let-binding** that carries a result downstream:

```yaml
- let: connector_t              # bind name
  from:
    sense.locate:               # …to the result of a primitive
      target_ref: connector
      modality: auto
```

A bound name is then usable anywhere a parameter of that type is expected
(`target: connector_t` below). This is the observe→act (L1) pattern.

## Primitive calls

Every call key is one of the **50 ISA primitive ids** across seven categories
(`reach`, `grasp`, `in_hand`, `transport`, `place`, `force`, `sense`). The
schema types each primitive's parameters precisely — names, units, enums — so a
typo or a wrong unit is rejected at validation, not at run time:

```yaml
- grasp.pinch:
    target: connector_t           # an ObjectTarget (here a let-reference)
    force_budget: 8 N             # quantities are unit-suffixed strings
    tactile_target: auto          # `auto` is accepted only where the spec types T | auto
    slip_response: retighten      # an enum member
```

Three rules worth internalizing:

- **Quantities are unit-suffixed strings** (`"8 N"`, `"30 mm"`, `"0.05 m/s"`),
  the JSON/YAML form of the spec's `value` + `unit`. A `[min, max]` range is a
  two-element array of quantities.
- **`auto`** is permitted only where `spec/01` types the parameter as `T | auto`
  (e.g. `tactile_target`, a `sense.locate` `modality`). Elsewhere it is rejected.
- **Task-level magnitudes**, not robot limits. `force_budget: 8 N` is the task
  ceiling; retarget clamps it to the embodiment's `grip_force_max` and the
  object's `max_contact_force`. You never write a robot's numbers here.

## Stop / completion conditions

Force and motion primitives end on a **stop condition** (`spec/01` § Stop /
completion conditions). Each primitive accepts only the condition family the
spec allows it, and the schema enforces that:

```yaml
- force.insert_fit:
    target_fit: receptacle_t
    force_budget: 15 N
    compliance: active
    stop_condition:
      all_of:
        - effort_rise: 12 N        # effort rises…
        - reached: { depth: 8 mm } # …at the expected depth ⇒ seated (vs jam)
```

`pull.stop_condition` rejects `detent`; `screw.completion` requires a torque
(not force) effort; `press_button.actuation` is `detent | effort_rise(force)`
only. Reach for the example matching your primitive rather than guessing.

## Targets

Three target types recur (`spec/01` § Target types), each precisely typed:

- **ObjectTarget** — a grasp/sense target: `pose` (required) plus optional
  `geometry`, `estimated_mass`, `center_of_mass`, `max_contact_force`
  (fragility), `features`, `uncertainty`. Usually supplied as a let-reference
  from a `sense.locate`.
- **SurfaceTarget** — a `reach.approach`/`hover` or `place.put_down` target:
  `point` + outward `normal` + `frame` (+ optional `uncertainty`).
- **ObjectTarget**/**SurfaceTarget** also accept `auto` and let-references where
  the spec allows.

## Validate before you ship

```bash
cargo run -p rfl-cli -- validate examples/01-cable-insertion/skill.yaml
# VALID: skill 'cable-insertion' — 8 statement(s) (6 primitive call(s), 2 let-bind(s))
```

`validate` runs both the schema-level parameter typing **and** the Class-1
composition-validity checks (`spec/01` § Composition validity):

- **DOF-admissibility** — an `in_hand` move on a `form_held` /
  `rotation_constrained` grasp is rejected.
- **`no_active_grasp` lifecycle** — you cannot grasp while already holding
  without an intervening release / handoff.
- **STB3 stability-class successor** — a support-closure grasp must be lowered
  to a controlled safe state, never open-released mid-air.
- **GraspRef-supersession** — a stale grasp handle after a regrasp / handoff is
  rejected.

A composition that parses but violates any of these exits `2` with the offending
rule named. Fix the composition, not the checker.

## Retarget to see it lower

```bash
cargo run -p rfl-cli -- retarget examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml
```

prints one canonical `execute` goal per primitive — the embodiment-resolved
form a driver consumes. If your skill uses a capability the embodiment does not
declare, retarget reports `capability_absent` (not a parse error): the skill is
valid, the pairing is not. See
[Authoring an embodiment descriptor](authoring-embodiments.md) for the other
half, and the [CLI reference](cli-reference.md) for exit codes.
