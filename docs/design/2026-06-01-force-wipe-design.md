# Design: force.wipe (contact-maintenance band) — the force *lower* bound

Status: approved design, pre-implementation (2026-06-01)

This is the seventeenth reference-implementation increment. It implements `force.wipe`
(`spec/01` § 6.8) in its core v0: the **two-sided normal-force band**. Its leverage is a real
gap in the force-trajectory class — the class bounds force only from *above* (`≤ budget`), so
no conformance check enforces a force *lower* bound on contact wrench. `force.wipe` is "the
maintained-invariant class of `reach.hover` applied to contact force" (`spec/05` § 6.8): its
invariant is a band `normal_force ± tolerance`, where **loss of contact (force → 0) is a
violation** just as much as excess force. The spec is authoritative; this realizes its band.

## 1. The gap

The `ForceTrajectory` `check_envelope` arm enforces `wrench.force ≤ force_budget` and
`wrench.torque ≤ force_profile.torque` — both *upper* bounds. The grasp-continuity class
enforces a `securing_force ≥ min_holding_force` *floor*, but on the held-grasp securing force,
not on contact normal force during a path. So a "wipe" that loses contact entirely (normal
force → 0) would pass every check today. `force.wipe`'s § 6.8 safety envelope:

> Loss of contact (normal force → 0) or excessive normal force (> budget) is an envelope
> violation. (The maintained-invariant class of `reach.hover`, applied to a contact force
> along a path.)

This increment adds the missing *lower* edge — making the force class two-sided.

## 2. No schema fight

Like `reach.hover` and `force.press_button`: `force.wipe` is in the skill-isa PrimitiveId
enum, `ForceWipeParams` is fully typed (`surface` + `wipe_path` + `normal_force` required;
`normal_force_tolerance` is `Force | auto`), and `force.wipe` is in the embodiment capability
enum. The band reuses the existing `wrench` telemetry. So **no skill-isa / capability /
driver-interface schema change** — pure rfl-core + rfl-conformance + an example.

## 3. The contract (v0)

- **Reuse the class.** `envelope_class_for("wipe") => ForceTrajectory` — wipe *is* a
  force-trajectory bound; the band is a richer (two-sided) form of it, not a new class.
- **New band leg (the gap-closure).** The lowering emits `force_profile = {"normal_force": S,
  "normal_force_tolerance": T}` **only when `T` is an explicit `Force`** (an `auto` / absent
  tolerance defers — no band, § 7). The `ForceTrajectory` arm gains a **vacuous-when-absent
  band leg**: when `force_profile.normal_force` is present, every `wrench` sample's `|force|`
  must lie in `[S − T, S + T]` — below ⇒ a `contact_lost`-shaped failure, above ⇒ a
  `normal_force_exceeded`-shaped failure. Absent (insert_fit's `axial`, screw's `torque`,
  press_button's `actuation`) ⇒ skipped, exactly the securing-floor / station-keeping
  vacuous-leg pattern. The class stays one envelope per primitive; the divergence is driven by
  what the lowering emitted.
- **Position-tracking deferred.** `wipe_path` (a `{trajectory}` Compound) carries opaque; the
  tangential tracking + contour-following leg is a named follow-up (it needs `wipe_path`
  resolved plus a position signal, the wipe analogue of hover's `max_travel`).

## 4. Surface

**rfl-core (one commit — exhaustive-match):**
- `struct ForceWipe` (`src/skill_isa.rs`) mirroring `ForceScrew`: `surface` (FrameRef),
  `wipe_path` (`serde_yaml::Value`, carried), `normal_force` (Quantity), `normal_force_tolerance`
  (`Option<serde_yaml::Value>`), `compliance` (`Option<Compliance>`).
- `Primitive::ForceWipe(ForceWipe)` + `check_capability` arm
  `Primitive::ForceWipe(_) => "force.wipe"` + the lower dispatch arm
  `Primitive::ForceWipe(p) => (lower_force_wipe(p, e), "wipe")` — all this commit.
- `lower_force_wipe` (`src/translation.rs`): `force_budget: None` (the band owns both bounds);
  `env.force_profile = {"normal_force": S, "normal_force_tolerance": T}` when `T` parses to an
  explicit Force (else no band — auto deferred); `target_pose: PoseExpr::Ref { ref: surface }`;
  `compliance` mapped like screw/insert_fit; `wipe_path` carried symbolic (not lowered v0).

**rfl-conformance (`src/lib.rs`):**
- Extend the `ReferenceDriver` wrench echo: `force_mag` falls back from `ca.force_budget` to
  `force_profile.normal_force` — so a nominal wipe emits `wrench.force = [0, 0, S]`, in band.
- Strengthen the `ForceTrajectory` arm with the band leg: read `force_profile.normal_force`
  (+ `normal_force_tolerance`); if present, every `wrench` sample's `|force|` must be in
  `[S − T, S + T]` (the failure reason names which edge). Vacuous without `normal_force`.
- Add `Fault::LoseContact` (wrench.force → `[0, 0, 0]`, mirroring `Fault::UnderSecure`).

**examples + goldens:**
- `examples/03-screw-fasten/skill-wipe.yaml` (a `force.wipe` with `surface`, `wipe_path`,
  `normal_force`, an explicit `normal_force_tolerance`, `compliance`).
- `force.wipe` added to the `force` capability list of the three 03 descriptors.
- New `crates/rfl-conformance/tests/wipe.rs` golden (3 stems, like `press_button.rs`).

**tests:**
- translation unit: `wipe_lowers_normal_force_band`, `wipe_capability_absent_when_not_declared`.
- golden: 3 stems + byte-identical + every-line-schema-valid.
- envelope_conformance: `nominal_wipe_holds_the_contact_band` (ForceTrajectory Pass),
  `wipe_loss_of_contact_fails` (`Fault::LoseContact` → Fail), `wipe_over_force_fails`
  (`Fault::OverForce` → Fail, reused).
- lib unit: the band leg on hand-built reports (in-band Pass; below-band Fail; above-band Fail).

## 5. Non-vacuity

`Fault::LoseContact` (force → 0, below band) is the new bite — it proves the *lower* bound is
real (an upper-bound-only check passes a zero-force "wipe"). `Fault::OverForce` (reused)
catches the upper edge. The conformant nominal sits in-band. The band leg is the mirror of the
securing floor, applied to contact wrench rather than securing force.

## 6. Commit shape (executing-plans, inline)

Three commits, each red→green; full `cargo test` + `validate.py` read in a batch *separate*
from the commit; each ff-pushed with a `git show --stat` self-check:
1. **rfl-core primitive** — struct + variant + gate + `lower_force_wipe` + `skill-wipe.yaml` +
   3 descriptors + `wipe.rs` golden + translation tests.
2. **conformance** — `envelope_class_for` + wrench-echo fallback + band leg + `Fault::LoseContact`
   + the 3 integration tests + the lib unit test.
3. **README** status line (spec already states the hybrid invariant, so no spec change).

## 7. Faithfulness, determinism, and what stays deferred

- **Faithful:** the band is `spec/01` § 6.8's `normal_force ± normal_force_tolerance`, sampled
  over the interval (the maintained-invariant discipline). `Fault::LoseContact` is the
  `contact_lost` failure mode; `Fault::OverForce` the `normal_force_exceeded` mode. The wrench
  echo mirrors the existing `force_budget` echo.
- **Deterministic:** the bench mutates a deterministic nominal report by fixed rules; new
  `wipe` goldens are added (not mutations of existing ones); existing goldens are byte-identical
  (no other action emits a `normal_force` band, so the band leg is vacuous for them).
- **Deferred behind named prerequisites:** `normal_force_tolerance: auto` derivation; the
  tangential **position-tracking + contour-following** leg (needs `wipe_path` resolved + a
  position telemetry signal); `feed_rate`; held-tool grasp-reaction during the wipe. `force.scrub`
  (the oscillating cousin, § 6.9) can later reuse this band leg. The standing deferred list
  (force.cut/scrub/snap_engage, in_hand.flip AUD1, hover over-envelope max_excursion, Σ
  arc/path/volume, full held-interval GC1, GC2-6, per-skill ε-table, ROS 2, Class 4,
  {trajectory} MoveSpec, the various `auto` derivations, physics injection) stays parked.
