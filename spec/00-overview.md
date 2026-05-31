# RFL Specification — Overview

> **Status**: design-complete (2026-05-30) — the scope, structure, and five constitutional principles the subsequent chapters instantiate are fixed; the full specification draft v0.1 (frozen text) is targeted for **2027 Q1**.

## What this specification defines

The RFL specification defines a three-layer abstraction between Vision-Language-Action foundation models and robotic embodiments:

| Chapter | Layer | One-line definition |
|---|---|---|
| [01-skill-isa.md](01-skill-isa.md) | **Skill ISA** | Fifty manipulation primitives in seven categories + a compositional algebra |
| [02-translation-layer.md](02-translation-layer.md) | **Translation Layer** | Canonical embodiment-agnostic action representation + deterministic retargeting |
| [03-driver-interface.md](03-driver-interface.md) | **Driver Interface** | ROS 2-compatible protocol that an embodiment implementation must satisfy |

Plus three cross-cutting chapters:

| Chapter | Topic |
|---|---|
| [04-tactile-manifold.md](04-tactile-manifold.md) | Formal abstraction for cross-embodiment tactile feedback |
| [05-conformance.md](05-conformance.md) | Conformance test classes, badge regime, and trademark gate |
| [06-extension-registry.md](06-extension-registry.md) | Namespaced extension proposal + promotion process |

## What this specification deliberately does not define

- VLA model architectures, training procedures, or inference protocols
- Perception models (camera processing, point-cloud handling)
- World models or simulation environments
- Robot hardware design or actuation engineering
- Commercial pricing, licensing models, or vendor relationships

These belong above (VLA) or below (embodiment hardware) RFL, or in adjacent commercial layers. The layer-discipline commitment in § Principle 4 below is binding.

## Five constitutional principles

Every design choice in this specification can be traced back to these five principles. A proposed change that violates any principle without explicit re-derivation is out of scope.

### Principle 1 — Embodiment-agnostic

No primitive, action, or interface element may assume the kinematic class, actuation modality, sensor suite, or anthropomorphic structure of any specific embodiment. The operational test: a draft of the specification must be reviewable by someone whose only experience of robotics is a non-anthropomorphic platform, and they must find the semantics expressible in their world without translation through anthropomorphic intuition.

### Principle 2 — Compositional

Primitives combine via a defined algebra (sequence, parallel, reactive, repeat, branch). Any multi-step manipulation expressible at all must be expressible as a composition of primitives, with no escape-hatch to embodiment-specific encoding.

### Principle 3 — Verifiable

Every conformance claim is mechanically testable by the conformance suite (see `05-conformance.md`). No principle is enforced by maintainer opinion alone.

### Principle 4 — Provider-neutral

No requirement in the specification may be traceable to a single vendor's product roadmap or proprietary capability. Vendor-specific examples are permitted only as parenthetical illustrations and only when at least one structurally distinct alternative exists.

### Principle 5 — Forward-compatible

Once v1.0 ships, no breaking change may land in any v1.x release. New capability enters through namespaced extensions (`06-extension-registry.md`) or through a major-version bump that preserves a v1.x-compatible compatibility layer.

## Specification governance

Spec changes follow the workflow in [CONTRIBUTING.md § Spec-change discipline](../CONTRIBUTING.md). Major-version transitions and any deviation from the five constitutional principles require explicit Maintainer + Technical Steering Committee sign-off (TSC membership and rules are described in the project governance documents, not in the spec itself).
