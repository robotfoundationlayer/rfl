# Design — freed-part disposition disclosure (TM21c; `safety_flags` first use)

**Date:** 2026-06-01
**Increment:** 24
**Status:** approved
**Scope:** rfl-core (a `Status.safety_flags` struct that catches the Rust model up to the
already-declared schema) + rfl-conformance (`ReferenceDriver` population, a new
`check_freed_part_disposition`, a `FreeingDriver` with two adversaries). No schema change, no
golden change, no spec change.

## 0. The gap

`spec/04` § Freed-part handling (TM21c) requires: on a freeing event the freed part's disposition
MUST be `retained` or `safe_zone_release(zone)` — an uncontrolled drop is forbidden, even under
degraded detection. The driver-interface schema already declares the disclosure channel —
`StatusResult.safety_flags.freed_part_disposition` (`{disposition: retained|safe_zone_release,
zone}`) — **but no driver populates it and no conformance check reads it.** It is the 5th
declared-but-honesty-unchecked signal (after `events`, `verdict.evidence` ×2, `fidelity_tier`),
and the most safety-critical: an undisclosed freeing is, by TM21c, an uncontrolled drop.

The entire `safety_flags` object is in fact dead in the Rust model: increment 21 routed
`momentary_release` through `verdict.evidence`, so the Rust `Status` struct never gained a
`safety_flags` field at all. This increment gives `safety_flags` its first real use.

## 1. The contract (v0)

A **freeing operation** — an action whose lowered `force_profile.on_disengagement` is present
(today `force.unscrew`; `lower_force_unscrew` already emits `"retain"` by default or the authored
`"drop_safe"`) — must, on success, disclose `status.safety_flags.freed_part_disposition` such that:

1. it is **present** (absence on a freeing op = an uncontrolled drop = TM21c violation), and
2. its `disposition` **matches** the authored intent: `retain` → `retained`,
   `drop_safe` → `safe_zone_release`.

The check is **vacuous** for any action without `on_disengagement` (every non-freeing primitive
passes untouched — the established vacuous-when-absent leg pattern).

`force.unscrew` is the v0 demonstrator because it carries the authored `on_disengagement`, giving a
clean expected-vs-realized honesty check (the same shape as AUD3 fidelity-tier honesty, pinned to
the lowered truth, not the claim).

## 2. Schema delta: NONE (the Rust model catches up to the schema)

The schema already declares `safety_flags.freed_part_disposition`. This increment adds the Rust
`Status.safety_flags` struct to MATCH it — exactly the posture of increment 16's
`events: Vec<String> → Vec<serde_json::Value>` fix (the Rust type was wrong vs the schema object;
here the Rust type is simply absent). `skip_serializing_if = None` ⇒ existing reports (no
`safety_flags`) serialize byte-identically. `additionalProperties: false` on `safety_flags` is
respected: the Rust struct only ever emits `freed_part_disposition` (a declared key). `validate.py`
C1–C7 are skill-isa/embodiment invariants — untouched.

No retarget golden changes (`on_disengagement` was already emitted in the unscrew retarget output
since increment 11). No driver-report golden changes (`driver_protocol` goldens are cable-only —
no `on_disengagement` → `safety_flags` stays `None`).

## 3. Implementation points

- **`crates/rfl-core/src/driver.rs`**
  - `Status.safety_flags: Option<SafetyFlags>` (`skip_serializing_if = "Option::is_none"`).
  - `struct SafetyFlags { #[serde(skip_serializing_if = "Option::is_none")] freed_part_disposition:
    Option<FreedPartDisposition> }` (the `momentary_release` schema field is intentionally NOT
    added here — increment 21's check reads it from `verdict.evidence`; reconciling the two onto
    `safety_flags` is deferred).
  - `struct FreedPartDisposition { disposition: String, #[serde(skip_serializing_if =
    "Option::is_none")] zone: Option<serde_json::Value> }`.
  - The struct addition makes every `Status { … }` literal require `safety_flags: None`
    (compiler-enumerated, as `stop_latency` was this session).
- **`crates/rfl-conformance/src/lib.rs`**
  - `ReferenceDriver`: when the action carries `force_profile.on_disengagement`, set
    `status.safety_flags = Some(SafetyFlags { freed_part_disposition: Some(...) })` with
    `retain → {disposition: "retained"}` / `drop_safe → {disposition: "safe_zone_release", zone:
    <placeholder>}`.
  - `check_freed_part_disposition(goal, report) -> CheckOutcome`: read
    `goal.canonical_action.safety_envelope.force_profile.on_disengagement`; if absent → `Pass`
    (vacuous). Else compute the expected disposition; require
    `report.status.safety_flags.freed_part_disposition` present (else
    `Fail("uncontrolled drop: no freed_part_disposition")`) and equal to expected (else
    `Fail("disposition mismatch")`).

## 4. Adversarial drivers (non-circularity)

`FreeingDriver { inner: ReferenceDriver, response: FreeingResponse }` with
`FreeingResponse { Discloses, DropsUncontrolled, FalseDisposition }`:

- **`Discloses`** (conformant) — the nominal populated disclosure → `Pass`.
- **`DropsUncontrolled`** — freeing op succeeds but `safety_flags` is `None` → the
  uncontrolled-drop `Fail`. This is the proof `safety_flags` is load-bearing: a check that ignored
  it could not distinguish this from a conformant freeing.
- **`FalseDisposition`** — reports the opposite disposition (authored `retain`, claims
  `safe_zone_release`: a part that should have stayed held was released) → the mismatch `Fail`.

Both adversaries trip distinct clauses; the nominal `ReferenceDriver` passes.

## 5. Tests

- `check_freed_part_disposition` unit (lib.rs): present+matching → Pass; absent → Fail;
  mismatch → Fail; non-freeing action → vacuous Pass.
- `envelope_conformance.rs` on `examples/03-screw-fasten/skill-unscrew.yaml` (the unscrew action):
  `Discloses` → Pass; `DropsUncontrolled` → Fail; `FalseDisposition` → Fail.
- Optionally a `driver_protocol` unit asserting `safety_flags` serializes against the schema
  (parallel to `telemetry_with_station_error_is_schema_valid`).

**Expected deltas:** rfl-core +0–1; conformance `lib` +1, `envelope_conformance` +2–3. All 12
golden binaries byte-identical. `validate.py` C1–C7 PASS.

## 6. Deferred (each behind a named prerequisite)

- **`force.cut` freeing disclosure** — cut frees a piece (cut-completion, TM20c) but has no
  authored `on_disengagement`; extending requires a freeing marker on the cut lowering and a
  presence-only check (no expected disposition).
- **`grasp.release` freeing** — release frees the held object; a controlled-release disposition.
- **TM20c freeing detection** — the constrained → free transition reported from the freeing
  `ForceEvent` (breakaway / cut-completion).
- **TM22c freed-part tracking** — the freed part entering the collision augment set + world-state.
- **`safety_flags.momentary_release` reconciliation** — increment 21 reads it from
  `verdict.evidence`; both should route through `safety_flags` (latent drift).
- **`zone` geometry verification** — the safe-release region is a placeholder in v0.
