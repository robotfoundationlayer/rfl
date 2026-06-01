# Example 03 — Screw fastening, and the primitive-breadth showcase

> **Status**: worked example running. The base skill and its 21 variants retarget
> onto the three embodiment descriptors under `embodiments/` and are pinned by
> conformance test class 2 against committed golden snapshots. What began as a
> screw-fastening task became the **breadth showcase**: each variant exercises a
> different Skill ISA primitive, grasp mode, or conformance dimension, so
> together they cover the full 50-primitive ISA and every normative conformance
> obligation.

## Base task

`skill.yaml` — drive a threaded fastener with a held driver: locate the fastener,
grasp the driver, engage, and drive under coupled rotation + axial feed with a
torque stop. The base case for the `force.screw` primitive and the
tool-mediated torque-reaction grasp-force derivation (`02` GF4c).

## Variants by category

Each variant is the same `cargo run -p rfl-cli -- retarget <file> --embodiment
<…>` invocation; together they lower **all 50 primitives** across the seven
categories.

### Grasp modes (closure vocabulary, `01` § 2)

| File | Primitive | Closure / note |
|---|---|---|
| `skill-power.yaml` | `grasp.power` | whole-volume force closure (max contact) |
| `skill-lateral.yaml` | `grasp.lateral` | key-grip across a thin dimension |
| `skill-tripod.yaml` | `grasp.precision_tripod` | three-point, non-degenerate triangle |
| `skill-hook.yaml` | `grasp.hook` | the first **form** closure (hookable feature) |
| `skill-envelope.yaml` | `grasp.envelope` | caging form-closure for an imprecisely localized object |
| `skill-adjust.yaml` | `grasp.adjust` | modify an established grasp in place (raise grip force) |

### Grasp-continuity group (GC1–6, `05`)

| File | Primitive | Continuity mode |
|---|---|---|
| `skill-regrasp.yaml` | `in_hand.regrasp` | make-before-break (GC3) |
| `skill-pivot.yaml` | `in_hand.pivot` | controlled under-actuation, one DOF released + resecured (GC4) |
| `skill-flip.yaml` | `in_hand.flip` | bounded momentary release — the only continuity-suspending primitive (GC5, AUD2) |
| `skill-handoff.yaml` | `transport.handoff` | two-party co-grasp, combined-force ceiling (GC6) |

### The remaining categories

| File | Primitives | Note |
|---|---|---|
| `skill-inhand.yaml` | `in_hand.rotate` / `translate` / `roll` / `slide` | the plain continuity-preserving in-hand moves |
| `skill-transport.yaml` | `transport.lift` / `lower` / `follow_trajectory` | the plain held transports beyond `move_to_pose` |
| `skill-place.yaml` | `place.put_down` | controlled, stability-confirmed release |
| `skill-reach.yaml` | `reach.to_pose` / `reach.approach` | the remaining free-space reaches (move-to-absolute-pose + guarded approach) |
| `skill-sense.yaml` | `sense.probe` / `verify` / `weigh` | completes the sense category (5/5); `sense.weigh` produces `estimated_mass` |

### Force category (10/10) and its conformance dimensions

| File | Primitive | Dimension first exercised |
|---|---|---|
| `skill.yaml` | `force.screw` | tool-mediated torque reaction (GF4c) |
| `skill-unscrew.yaml` | `force.unscrew` | freed-part disposition (`04` TM21c, the `safety_flags` channel) |
| `skill-press.yaml` | `force.press_button` | event-gated actuation (the detent ForceEvent channel) |
| `skill-snap.yaml` | `force.snap_engage` | reversibility REV2 (semi-reversible, `confirm_held`) |
| `skill-cut.yaml` | `force.cut` | reversibility REV3 (irreversible) + the first conjunctive capability gate (`tool_safety`) |
| `skill-wipe.yaml` | `force.wipe` | the contact-maintenance band (a force *lower* bound) |
| `skill-force.yaml` | `force.push` / `pull` / `scrub` | completes the force category |

## Conformance exercised (`05`)

Across the variants: all four envelope classes (terminal-postcondition,
grasp-continuity GC1–6, force/torque-trajectory ENV4, interval-invariant), the
stability obligations (STB2 / STB3), the audit-and-transparency group (AUD1–3),
the reversibility spectrum (REV1–3), the force-event and contact-band checks, the
conjunctive capability gate, and the freed-part-disposition disclosure — each
verified against adversarial drivers.

## How to run

```bash
# From the repository root — the base task:
cargo run -p rfl-cli -- retarget examples/03-screw-fasten/skill.yaml \
    --embodiment examples/03-screw-fasten/embodiments/allegro.yaml

# Any variant, on any of the three embodiments:
cargo run -p rfl-cli -- retarget examples/03-screw-fasten/skill-cut.yaml \
    --embodiment examples/03-screw-fasten/embodiments/leap.yaml
```

A variant that uses a capability an embodiment does not declare reports
`capability_absent` (an honest negotiation outcome), not a parse error.
