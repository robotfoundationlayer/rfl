# TM20c freeing detection and `grasp.release` disposition — design + scope finding

Status: design-complete; implementation **partially BLOCKED** (see § Finding)

## Goal (as framed)

Generalize the freed-part disposition mechanism (the `status.safety_flags`
channel, `spec/04` TM20c/TM21c) to a third consumer — `grasp.release` — with an
adversarial driver, mirroring the two existing consumers (`force.unscrew`
intent-ful, `force.cut` intent-less presence-only).

## Finding: `grasp.release` is NOT a `freed_part_disposition` consumer

Reading the spec closely, `grasp.release`'s "do not abandon the object" guarantee
is a **different mechanism** than the force-op freed-part channel:

- `spec/04` § Freed-part handling (lines 427–465) scopes TM20c/TM21c to the
  moment **a `force` operation frees a part** — `force.unscrew` thread
  disengagement (breakaway `effort_drop`) or `force.cut` cut-completion. The
  freeing event is a `ForceEvent`. `grasp.release` is named there only as the
  *dual* (its collision-set augmentation), not as a freed-part consumer.
- `spec/01` line 348 gives `grasp.release`'s actual no-drop guarantee: the
  **`supported(object)` predicate gated by `require_stable`** — "before opening
  the grasp, the primitive confirms `supported(object)`; if it fails and
  `require_stable` is set, the grasp is not released (the object is never
  abandoned in a pose from which it will fall)." `supported(object)` is the CoM
  projecting strictly inside the object's **contact polygon** by a margin — the
  `grasp.platform` CoM-over-polygon test applied to a resting object.

So wiring `freed_part_disposition` onto `grasp.release` would mechanize the wrong
contract. The correct `grasp.release` safety contract is `supported` /
`require_stable`, which is a **geometric predicate over contact-polygon
geometry** — and that geometry is the same un-reported contact geometry that
BLOCKS STB1 (per the standing backlog block-list). The numeric predicate is not
computable from the current wire (no contact-polygon field), so the geometric
check is **BLOCKED on contact geometry**, exactly like STB1.

## What is implementable (deferred, not done here)

The *audit-disclosure* layer is implementable without the geometry: a
`grasp.release` authored with `require_stable: true` must **disclose** in its
status that support was confirmed before opening (a verdict-evidence /
`safety_flags` attestation), and an adversarial driver that opens the grasp
without the attestation fails. This is the genuine "third consumer of the
disclosure discipline," and it is golden-safe (vacuous unless `require_stable`
is authored, so the existing cable-insertion seated release is unaffected).

It requires, as one exhaustive-match increment:

1. a `require_stable: bool` parameter on `grasp.release` (`spec/01` § 2.10 /
   line 348) — a new enum/param + its lowering to a `release_safety` marker;
2. a `check_release_safety` battery check reading that marker and requiring a
   support-confirmation attestation in `status`;
3. a `ReferenceDriver` extension + an adversarial driver that omits it;
4. an example skill variant exercising `require_stable` + its goldens + report.

This is a clean future increment. It is deferred here (not bundled) because it
needs a new wire field and a worked example, and because the *substantive* part
of the task (the `supported` geometric check) is BLOCKED on contact geometry —
so shipping only the disclosure shell would overstate coverage.

## Decision

Record the finding; do **not** mis-wire `freed_part_disposition` onto
`grasp.release`. TM20c/TM21c remain correctly scoped to the two force-op
consumers. The `grasp.release` disclosure increment is specified above for a
later session; its geometric core is **BLOCKED: contact-polygon geometry
(same block as STB1)**.
