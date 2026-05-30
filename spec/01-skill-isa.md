# Skill ISA — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1). The structure below fixes the scope of the chapter; the body content will be filled iteratively before v0.1 freeze.

## Scope

This chapter defines:

1. The fifty primitive manipulations, organized into seven categories
2. The compositional algebra that combines primitives into multi-step manipulations
3. The JSON schema (`schemas/skill-isa.schema.json`) against which a Skill ISA file is validated

## Seven categories (provisional)

| Category | Intent | Approximate primitive count |
|---|---|---|
| `reach` | Approach, alignment, pre-grasp positioning | ~6 |
| `grasp` | Object acquisition (pinch, power, hook, ...) | ~10 |
| `in_hand` | In-hand manipulation, regrasp, rotation | ~7 |
| `transport` | Carry, hand-off, body-frame relocation | ~6 |
| `place` | Object placement and release | ~6 |
| `force` | Force-controlled interaction (insertion, screwing, ...) | ~10 |
| `sense` | Sensing-only primitives (probe, inspect, ...) | ~5 |

Exact counts and naming are part of the v0.1 freeze.

## Compositional algebra (provisional)

```bnf
Composition ::= Primitive
              | Sequence
              | Parallel
              | Reactive
              | Repeat
              | Branch

Sequence    ::= "sequence" "(" Composition { "," Composition } ")"
Parallel    ::= "parallel" "(" Composition { "," Composition } ")"
Reactive    ::= "reactive" "(" Composition "," "until" "(" Predicate ")" ")"
Repeat      ::= "repeat" "(" Composition "," Count ")"
Branch      ::= "branch" "(" Predicate "," Composition "," Composition ")"
```

The reactive operator has subtle semantics around concurrent envelope violation that will be specified rigorously in the v0.1 text.

## Open issues

- Final primitive list per category — pending external red-team review (see CONTRIBUTING.md spec-change discipline)
- Predicate language scope (closed enumeration vs. extensible vocabulary)
- Type system formality (gradually typed JSON schema vs. fully typed algebraic specification)
