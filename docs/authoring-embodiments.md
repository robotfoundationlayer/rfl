# Authoring an embodiment descriptor

An embodiment descriptor tells RFL what a robot **can do** and **within what
limits**, in roles rather than body parts, so a skill can retarget onto it
without naming any of its hardware. This guide walks through writing one; the
normative references are [`spec/03-driver-interface.md`](../spec/03-driver-interface.md)
(frame model + capability manifest + limits + sensors) and
[`spec/04-tactile-manifold.md`](../spec/04-tactile-manifold.md) (the
contact-sensor descriptor), and the machine-checkable form is
[`schemas/embodiment-descriptor.schema.json`](../schemas/embodiment-descriptor.schema.json).

The worked example is
[`examples/01-cable-insertion/embodiments/allegro.yaml`](../examples/01-cable-insertion/embodiments/allegro.yaml);
two structurally distinct hands (`leap.yaml`, `pneumatic-6f.yaml`) sit beside it.

----

## File shape

```yaml
embodiment:
  id: wonik-allegro-v4
  class: tendon-driven-4finger-anthropomorphic
  frames:        { … }   # § Frame model
  capabilities:  { … }   # § Capability manifest
  limits:        { … }   # § Limits
  tactile:       { … }   # § Contact sensors (optional)
  sensors:       { … }   # § Non-contact sensors (optional)
```

`id`, `class`, `frames`, `capabilities`, and `limits` are required.

## Frame model — roles, never body parts

```yaml
frames:
  control_frames: [tcp_index, tcp_middle, tcp_ring, tcp_thumb, palm]
  role_defaults:
    grasp:   tcp_thumb         # which frame plays each abstract role
    sensor:  palm_cam
    tactile: tcp_thumb
    support: palm
  tool_axis: { tcp_thumb: +z }
```

A skill says "grasp"; the descriptor binds the grasp **role** to a concrete
control frame. This is the Principle-1 seam: skills are written against roles,
descriptors map roles to hardware. Signed axes are `+z` / `-x` etc.

## Capability manifest — what the embodiment asserts

```yaml
capabilities:
  skills: [grasp.pinch, grasp.lateral, grasp.envelope, transport, transport.carry, force.insert_fit, sense.locate]
  aux:
    tactile_sensing: true      # distributed tactile ⇒ manifold-tier confirmation
    compliance: active
```

- Each entry of `skills` is an exact Skill ISA capability key — a core primitive
  id, a **category gate** (`transport` / `in_hand` / `force` / `sense`, asserting
  the whole category baseline), or a registered `ext.<ns>.<name>`. An unknown id
  is rejected (anti-drift invariant C1: the valid set is derived from
  `skill-isa.schema.json` at validation time).
- `aux` types each known auxiliary: `compliance` is a set or scalar over
  `{passive, active, virtual}`; `tactile_sensing` / `coordination_channel` are
  booleans; `tool_safety` / `human_collaboration_safety` carry their `spec/03`
  structures. Namespace-prefixed unknown aux keys are permitted.

A skill retargets onto this embodiment only if every capability it uses is
declared; otherwise retarget yields `capability_absent`.

## Limits — and the completeness rule (M2)

Limits are a flat `embodiment.limits.*` map. The **M2 limit-completeness** rule
(`spec/03` § Capability manifest) is the one most likely to bite a new author:
*every limit that an asserted capability references must be declared.*

```yaml
limits:
  # reach baseline — REQUIRED of every embodiment (§ Reach baseline limits)
  v_cartesian_max:        0.4 m/s
  w_cartesian_max:        2.0 rad/s
  a_cartesian_max:        1.5 m/s^2
  joint_velocity_ceiling: 3.0 rad/s
  stop_time:              0.1 s
  tracking_bandwidth:     5 Hz
  # asserted-capability limits
  stability_margin:       0.5          # because `transport.carry` is asserted
  grip_force_max:         20 N         # force-closure grasps
  v_grasp:                0.05 m/s
  payload_grasp_pinch:    3 N          # because `grasp.pinch` is asserted
  payload_grasp_lateral:  2 N
  lateral_grasp_max_thickness: 15 mm
  payload_grasp_envelope: 4 N
  enclosure_range:        [10 mm, 80 mm]
```

If you assert `grasp.envelope`, you must declare `payload_grasp_envelope` and the
`enclosure_range` range; assert `transport.carry`, declare `stability_margin`;
assert `force.scrub`, declare `normal_compliance_range` and `oscillation_max`.
The reach baseline is mandatory unconditionally. `validate.py` derives the
required-limit set per capability from the `spec/03` tables and checks your
descriptor declares exactly them — a dropped limit fails Class-1.

## Contact sensors (optional) — and graceful degradation

```yaml
tactile:
  tcp_thumb: { sensor_class: array, sites: 1, features: [contact, normal_force, shear],
               resolution: { spatial: 2 mm, temporal: 100 Hz, value: 0.05 N } }
  tcp_index: { sensor_class: array, sites: 1, features: [contact, normal_force, shear],
               resolution: { spatial: 2 mm, temporal: 100 Hz, value: 0.05 N } }
```

- `sensor_class` binds the frame to a per-class **adapter** (`ft` / `array` /
  `visuotactile`; see [`schemas/tactile-manifold/`](../schemas/tactile-manifold/)).
- Each declared `feature` must be one the bound adapter **produces** (anti-drift
  invariant C6) and must be in the closed-core feature set (C3–C5).
- **Omit the whole `tactile` block (and `aux.tactile_sensing`)** for a hand with
  no contact sensing — exactly the `pneumatic-6f.yaml` case. A `tactile_target:
  auto` in a skill then degrades to the force/position proxy (`spec/04` §
  Graceful degradation), and certification records the lower fidelity tier
  honestly rather than failing.

## Non-contact sensors (optional)

```yaml
sensors:
  palm_cam: { bore_axis: +z, fov: { h_angle: 60 deg, v_angle: 45 deg },
              working_range: [0.05 m, 1.0 m], modalities: [presence, pose] }
```

## Validate

The descriptor has no standalone CLI subcommand; it is validated as part of a
retarget, and structurally by the Class-1 runner:

```bash
uv run --with jsonschema --with pyyaml python schemas/validate.py   # validates all reference descriptors + C1–C8
cargo run -p rfl-cli -- retarget <skill.yaml> --embodiment <your-descriptor.yaml>
```

A retarget that succeeds proves the descriptor declares every capability and
limit the skill needs. Start from the closest reference descriptor
(`allegro` = tactile anthropomorphic; `leap` = ft-class; `pneumatic-6f` =
no-tactile) and edit, rather than from a blank file. See
[Authoring a Skill ISA composition](authoring-skills.md) for the demand side.
