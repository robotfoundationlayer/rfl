# Getting started

> **Status**: pre-release skeleton. Full documentation site (with examples, tutorials, and API reference) targeted for v0.1 (2027 Q1).

## Quickstart — the whole pipeline, no hardware

One script runs the entire loop on the committed cable-insertion example —
validate → retarget → certify (live, against a mock driver) → verify → sign →
verify-signed:

```bash
scripts/demo.sh
```

The "mock driver" drains the canonical execute goals and replays a recorded,
conforming telemetry session; the certificate it produces is byte-identical to
the `--report` replay path, so the live and replay certify paths demonstrably
agree. Every command it uses is documented in the [CLI reference](cli-reference.md).

To write your own skill or embodiment instead of running the example, see
[Authoring a Skill ISA composition](authoring-skills.md) and
[Authoring an embodiment descriptor](authoring-embodiments.md).

## Audience

This page is for engineers who want to understand what RFL is and how it would fit into their existing robot-AI stack.

- **VLA model developers** — RFL is the target layer your model emits actions to
- **Embodiment (hand / arm / humanoid) manufacturers** — RFL is what your driver implements to be addressable by any compliant VLA
- **Application developers** — RFL lets you swap VLAs and hands without rewriting integration code

## Reading order

1. [README](../README.md) — what RFL is in one screen
2. [Whitepaper](../whitepaper/README.md) — the full argument (~25,000 words)
3. [Specification overview](../spec/00-overview.md) — the three layers + five constitutional principles
4. [Example 01 — cable insertion](../examples/01-cable-insertion/README.md) — the same skill across three hand classes
5. [CONTRIBUTING.md](../CONTRIBUTING.md) — how to participate in spec design

## When to use RFL (and when not to)

**Use RFL when:**

- You target multiple robot embodiments and want to write integration code once
- You want your VLA or your hand to be addressable by the broadest possible ecosystem
- You need conformance certification for industrial deployment (ISO 10218 / 13482 flow-through, see [spec/05-conformance.md](../spec/05-conformance.md))

**Don't use RFL when:**

- You're building a research prototype that only ever runs on one hand
- You need bit-exact reproducibility of a specific VLA's internal trajectories — RFL canonical actions are deterministic but VLA-internal sampling is not RFL's concern
- The embodiment is so far from anthropomorphic / industrial-collaborative norms that no Skill ISA category applies cleanly (rare — but possible, e.g., highly redundant soft-body robots without distinct end-effectors)
