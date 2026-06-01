# Per-skill ε-tolerance table — format + measurement harness design

Status: format design-complete; **values BLOCKED on measurement data**

## What it is

Class 2-loose conformance (`spec/02` RD2c, `spec/05`) holds a contact-dynamics
primitive's *realized execution* to **semantic equivalence within a per-skill
tolerance ε**, never byte-reproducibility. The primitives in scope are the ones
whose realized execution runs against contact dynamics: every `force.*`
(compliant search) and the passive-drive `in_hand.pivot(drive = gravity |
external)`. Their canonical-action *generation* stays Class 2-strict; only the
realized-execution check is loose. The per-skill ε-table is the data that fixes
"how loose" per primitive — the single remaining data-dependent pre-freeze item.

## Why it is BLOCKED (not deferrable by design)

ε is an empirical quantity: it is the realized-execution deviation a *conformant*
reference implementation exhibits across runs and embodiments. It cannot be
invented — too tight and a conformant driver fails; too loose and a
non-conformant one passes. It is measured, then frozen. This is the explicit
data-dependency the spec, README, and `spec/05` § status all carry.

## Proposed format (ready, value cells empty)

A schema-backed table keyed by primitive, with per-quantity tolerances and the
comparison metric each uses:

```yaml
# schemas/epsilon-tolerance.schema.json validates this; values pending measurement.
epsilon_tolerances:
  force.insert_fit:
    realized_pose:     { metric: geodesic_se3, tolerance: null }   # m + rad
    realized_wrench:   { metric: l2_norm,      tolerance: null }   # N, N·m
    seating_depth:     { metric: abs,          tolerance: null }   # m
  force.screw:
    completion_torque: { metric: abs,          tolerance: null }   # N·m
    turns:             { metric: abs,          tolerance: null }
  in_hand.pivot:
    final_orientation: { metric: geodesic_so3, tolerance: null }   # rad
  # … one block per contact-dynamics primitive
```

Design choices:

- **Per-quantity, per-metric** — a primitive's tolerance is not one scalar; pose,
  wrench, and completion quantities each carry their own metric (geodesic for
  SE(3)/SO(3), L2 for wrench, absolute for scalars) and their own ε. This matches
  how the envelope-class checkers already compare realized vs canonical.
- **`null` until measured** — the structure is committed and schema-validated now;
  a `null` tolerance means "unmeasured", and conformance treats an unmeasured
  primitive's loose check as *not yet gradeable* (explicitly, never silently as
  pass).
- **Keyed by primitive id**, reusing the `skill-isa` `PrimitiveId` enum (anti-drift
  check: every key is a real contact-dynamics primitive; cross-checkable like C1).

## Measurement harness (spec, to run when hardware/sim is available)

1. For each contact-dynamics primitive, retarget its worked example onto each
   reference embodiment and execute it **N times** on the reference driver
   (sim-first, then hardware), capturing the realized telemetry trace each run.
2. For each measured quantity, compute the run-to-run and embodiment-to-embodiment
   deviation under the quantity's metric; take a high percentile (e.g. p95) as the
   candidate ε, with a documented safety factor.
3. Cross-validate: a known-conformant driver must pass at ε; a known-deviant
   adversarial driver (the conformance suite already ships these) must fail.
4. Freeze the value into the table; the schema flips `null → <quantity>`.

## Decision

Land the **format** (schema + empty table + the anti-drift key check) as a clean
future increment; the **values** are BLOCKED on the measurement harness output,
which needs the reference driver running on sim/hardware (the supply-side
existence-proof track). Recorded so the format can ship independently of the
data.
