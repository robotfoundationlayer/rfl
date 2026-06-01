# Design: AUD3 fidelity-tier honesty — degradation disclosure

Status: approved design, pre-implementation (2026-06-01)

This is the twentieth reference-implementation increment. It opens the **audit / transparency
class** (`spec/05` AUD1-AUD3), the last major unexercised conformance dimension, at its
lightest entry: AUD3 fidelity-tier honesty. Like the ENV3 disturbance increment it is **pure
`rfl-conformance`** — no rfl-core / schema / example / golden change. The spec is authoritative;
this realizes it.

## 1. The gap

`spec/05` AUD3: "a degraded confirmation propagates its fidelity tier (`proxy` /
`proxy_reactive`) … a result reported at full `manifold` tier when it was degraded is
malformed." The pieces already exist: the lowering degrades `tactile_target` to
`TactileTargetOut::Proxy` on a no-tactile embodiment (`spec/04` graceful degradation), and the
`ReferenceDriver` echoes `fidelity_tier` (`manifold` / `proxy`) in its status. But **no checker
verifies the claimed tier is honest** — a driver could claim `manifold` on a proxy-only
embodiment and nothing catches it. `fidelity_tier` is the fourth declared-but-honesty-unchecked
channel (after `events`, `verdict.evidence` twice).

## 2. Pure conformance, no schema change

Reuses `tactile_target` (in the `CanonicalAction`: `Auto` ⇒ manifold, `Proxy` ⇒ proxy) +
`fidelity_tier` (in the status `StatusResult`). Both pre-exist. **No rfl-core / schema /
example / golden change** — everything lives in `rfl-conformance`, exactly like the ENV3
disturbance increment (14).

## 3. The contract

A new `check_audit_honesty(goal, report)`:
- The **expected** tier comes from the action's `tactile_target`: `TactileTargetOut::Auto` ⇒
  `manifold`; `TactileTargetOut::Proxy` ⇒ `proxy`; anything else (`Explicit` / `None`) ⇒
  vacuous (no auto-confirmation to disclose).
- The **claimed** tier is `report.status.fidelity_tier`.
- The violation is **over-claiming**: `claimed == "manifold"` while `expected == "proxy"` (an
  undisclosed degradation, malformed per AUD3). Under-claiming (proxy when manifold was
  available) is conservative, not a violation — faithful to AUD3's "manifold-when-degraded is
  malformed", which targets over-statement only.

## 4. Surface (rfl-conformance only)

- `src/lib.rs`: add `Fault::FalseTier` (sets `report.status.fidelity_tier = Some("manifold")` —
  over-claims) + its `FaultyDriver` match arm; add `pub fn check_audit_honesty`.
- `tests/envelope_conformance.rs`: drive the cable-insertion `grasp.pinch` (`tactile_target`
  auto, action index 1) — on **pneumatic-6f** it degrades to proxy, on **allegro** it stays
  manifold:
  - `nominal_proxy_tier_is_disclosed` (`ReferenceDriver`, pneumatic-6f, pinch) →
    `check_audit_honesty` Pass (honest "proxy").
  - `false_manifold_claim_on_proxy_fails` (`Fault::FalseTier`, pneumatic-6f, pinch) → Fail
    (claims manifold when degraded — the bite).
  - `manifold_tier_passes` (`ReferenceDriver`, allegro, pinch) → Pass (honest "manifold").
- A lib unit test for `check_audit_honesty` on hand-built reports (Proxy action + manifold claim
  → Fail; Proxy + proxy → Pass; Auto + manifold → Pass).

## 5. Non-vacuity

`Fault::FalseTier` proves the check bites — but only on a *degraded* action. On allegro the
truth is manifold, so claiming manifold is honest and the same fault is vacuous there; on
pneumatic-6f it is a violation. That asymmetry is the proof the check reads the *actual*
degradation (the lowered `tactile_target`), not merely the claim — it cannot be fooled by a
driver that always claims manifold *or* always claims proxy.

## 6. Commit shape (executing-plans, inline)

Two commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate* from
the commit; each ff-pushed with a `git show --stat` self-check:
1. **conformance** — `Fault::FalseTier` + `check_audit_honesty` + the 3 integration tests + the
   lib unit.
2. **README** status line (spec/05 AUD3 already states the obligation; no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the expected tier is the lowering's own `tactile_target` degradation decision
  (`spec/04`); the claimed tier is the driver's reported `fidelity_tier`. The rule targets
  over-statement, matching AUD3's "manifold-when-degraded is malformed."
- **Deterministic:** the bench mutates a deterministic nominal report by a fixed rule; pass/fail
  asserts, no goldens (like the ENV3 increment).
- **Deferred behind named prerequisites:** **AUD2** `momentary_release` propagation (needs
  `in_hand.flip` + cross-action audit state); **AUD3 freed-part disposition** disclosure (needs
  a realized `on_separation` / freed-part signal); **AUD1** the broad persisted-audit-record
  obligation; the `proxy_reactive` tier (the lowering produces only Auto/Proxy in v0); full
  three-valued `Verdict` honesty above `confidence_threshold`. The standing deferred list
  (`tool_safety` regime, force.scrub / push / pull, hover over-envelope max_excursion, Σ
  arc/path/volume, full held-interval GC1, GC2-6, per-skill ε-table, ROS 2, Class 4,
  {trajectory} MoveSpec, the various `auto` derivations, physics injection) stays parked.
