// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Grasp-force and stability derivations (`spec/02` § Grasp-force and stability
//! derivations, GF1c–GF3c).
//!
//! The capacity model is schematic and provider-neutral (Principle 4): RFL fixes
//! the *dependency* — capacity is a function of closure, grip, geometry, and
//! friction — not a vendor friction law. The per-mode factors below collapse the
//! `μ · geometry` term into one documented reference number per closure mode; they
//! are a v0 reference-implementation choice, pinned by golden, non-normative.

/// Standard gravity (m/s^2). `estimated_mass` is typed as weight under standard
/// gravity (`spec/01`), so only the dynamic clamp needs this as a scale.
pub const G0: f64 = 9.80665;

/// A held-grasp closure mode. v0 lowers `grasp.pinch` and `grasp.pin`; the enum gives
/// the derivations a forward-compatible key for the per-mode factors and payload limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraspMode {
    /// Antipodal force-closure pinch.
    Pinch,
    /// Extrinsic force-closure pin against an external surface (`spec/01` § 2.7).
    Pin,
    /// Support closure: an object borne in balance over a support polygon (`spec/01` § 2.6).
    Platform,
}

impl GraspMode {
    /// The schematic holding factor: required grip per unit held weight
    /// (`min_holding_force = weight · k_holding`). `≈ 1/(2μ)` at μ=0.25 for a
    /// 2-contact force-closure pinch; the pin's two friction interfaces (effector +
    /// surface) give the same schematic factor.
    #[must_use]
    pub fn k_holding(self) -> f64 {
        match self {
            GraspMode::Pinch | GraspMode::Pin => 2.0,
            // N/A for support closure: a borne object is not gripped, so there is no
            // grip-per-weight factor. Present for the exhaustive match; unused by
            // lowering (platform emits no min_holding_force).
            GraspMode::Platform => 1.0,
        }
    }

    /// The schematic reaction factor: a unit of grip resists `grip / k_reaction` of
    /// axial pull-out. Same μ basis as `k_holding`.
    #[must_use]
    pub fn k_reaction(self) -> f64 {
        match self {
            GraspMode::Pinch | GraspMode::Pin => 2.0,
            // N/A for support closure (see `k_holding`).
            GraspMode::Platform => 1.0,
        }
    }

    /// The descriptor limit key for this mode's rated payload (max holdable weight).
    #[must_use]
    pub fn payload_key(self) -> &'static str {
        match self {
            GraspMode::Pinch => "payload_grasp_pinch",
            GraspMode::Pin => "payload_grasp_pin",
            GraspMode::Platform => "payload_support",
        }
    }
}

/// GF1c — the static grip floor below which the held object falls under its own
/// weight. `weight_n` is the object weight in newtons (`target.estimated_mass`).
#[must_use]
pub fn min_holding_force(weight_n: f64, mode: GraspMode) -> f64 {
    weight_n * mode.k_holding()
}

/// GF2c — the dynamic-stability acceleration limit: the largest acceleration at
/// which inertial + gravity load (worst case collinear, `weight·(1 + a/g₀)`) stays
/// within the mode's rated holding capacity (`payload_n`, a weight). Returns the raw
/// dynamic limit in m/s^2; the caller takes the min with the kinematic ceiling.
/// Clamped at 0 — a weight above the rated payload (negative pre-clamp) is the
/// deferred payload-violation case, surfaced here as "cannot accelerate".
#[must_use]
pub fn dynamic_a_max(weight_n: f64, payload_n: f64) -> f64 {
    (G0 * (payload_n / weight_n - 1.0)).max(0.0)
}

/// The transport.carry acceleration clamp (`spec/01` § 4.4, `spec/02` § Dynamic stability,
/// line 137): the largest acceleration at which the GF2c worst-case collinear
/// inertial+gravity load `weight·(1 + a/g₀)` **plus** the `disturbance_budget` stays within
/// the mode's rated holding capacity (`payload_n`, a weight) with `stability_margin`
/// headroom, from § 4.4's precondition `payload ≥ (1+m)·(inertial_load + D)`:
///
/// ```text
/// a_max = g₀·( ( payload/(1+m) − D ) / weight − 1 ),  clamped ≥ 0.
/// ```
///
/// Reduces to [`dynamic_a_max`] at `m = 0, D = 0`, and is ≤ it for `m, D ≥ 0` — the "clamped
/// below the plain-transport limit" of `spec/02`:137. The pre-clamp going negative is the
/// § 4.4 `insufficient_stability_margin` case, surfaced as `a_max = 0` (carry quasi-statically
/// only), exactly as `dynamic_a_max` surfaces an over-payload weight. A v0 reference choice,
/// pinned by golden, non-normative (the same posture as `dynamic_a_max` / `k_holding`).
#[must_use]
pub fn carry_a_max(
    weight_n: f64,
    payload_n: f64,
    disturbance_n: f64,
    stability_margin: f64,
) -> f64 {
    let effective_payload = payload_n / (1.0 + stability_margin) - disturbance_n;
    (G0 * (effective_payload / weight_n - 1.0)).max(0.0)
}

/// GF3c — the reaction-load limit: a force primitive's reaction may not exceed the
/// grasp's axial capacity, bounded by the grip a `slip_response: retighten` grasp
/// can muster (`grip_force_max_n`) divided by the mode's reaction factor. Returns the
/// smaller of the requested budget and that capacity (both newtons).
#[must_use]
pub fn reaction_limit(force_budget_n: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    force_budget_n.min(grip_force_max_n / mode.k_reaction())
}

/// Schematic effective grip radius (m) for the tool-grasp rotational capacity — a v0
/// reference-implementation constant (pinned by golden, non-normative).
pub const R_GRIP: f64 = 0.02;

/// GF4c — the reaction-torque limit: a tool-mediated `force` primitive's reaction is a
/// torque about the tool axis; the held tool's grasp must resist it with rotational
/// holding capacity (≈ grip force × lever ÷ the reaction factor), or the tool spins
/// in-grasp. Returns the smaller of the requested torque budget and that capacity
/// (both N·m). The rotational counterpart of `reaction_limit` (GF3c).
#[must_use]
pub fn reaction_torque_limit(torque_budget_nm: f64, grip_force_max_n: f64, mode: GraspMode) -> f64 {
    torque_budget_nm.min(grip_force_max_n * R_GRIP / mode.k_reaction())
}

#[cfg(test)]
mod tests {
    // Deterministic grasp-force derivations: exact golden-value comparison is intended.
    #![allow(clippy::float_cmp, clippy::unreadable_literal)]

    use super::*;

    #[test]
    fn min_holding_force_is_weight_times_factor() {
        // 1.45 N connector, pinch k=2.0 -> 2.9 N.
        assert!((min_holding_force(1.45, GraspMode::Pinch) - 2.9).abs() < 1e-9);
    }

    #[test]
    fn dynamic_a_max_bites_below_pneumatic_ceiling() {
        // payload 1.5 N, held 1.45 N -> 9.80665 / 29 ≈ 0.33816 (< 0.8 ceiling).
        let a = dynamic_a_max(1.45, 1.5);
        assert!((a - 0.338160).abs() < 1e-5, "got {a}");
        assert!(a < 0.8);
    }

    #[test]
    fn dynamic_a_max_exceeds_strong_hand_ceiling() {
        // payload 3 N, held 1.45 N -> ≈10.5 (kinematic ceiling 1.5 wins downstream).
        assert!(dynamic_a_max(1.45, 3.0) > 1.5);
    }

    #[test]
    fn dynamic_a_max_clamps_at_zero_over_payload() {
        // held > payload -> pre-clamp negative -> 0.
        assert_eq!(dynamic_a_max(2.0, 1.5), 0.0);
    }

    #[test]
    fn reaction_limit_takes_the_smaller() {
        // 15 N budget vs 12/2 = 6 capacity -> 6.
        assert!((reaction_limit(15.0, 12.0, GraspMode::Pinch) - 6.0).abs() < 1e-9);
        // 5 N budget vs 20/2 = 10 capacity -> 5 (budget lower).
        assert!((reaction_limit(5.0, 20.0, GraspMode::Pinch) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn reaction_torque_limit_clamps_to_rotational_capacity() {
        // allegro grip 20 N: 20 * 0.02 / 2.0 = 0.2 N·m; the 2 N·m budget clamps to it.
        let a = reaction_torque_limit(2.0, 20.0, GraspMode::Pinch);
        assert!((a - 0.2).abs() < 1e-9, "got {a}");
        // pneumatic grip 12 N: 12 * 0.02 / 2.0 = 0.12.
        assert!((reaction_torque_limit(2.0, 12.0, GraspMode::Pinch) - 0.12).abs() < 1e-9);
    }

    #[test]
    fn reaction_torque_limit_keeps_budget_when_capacity_is_higher() {
        // a tiny 0.05 N·m budget under a 20 N grip (0.2 capacity) -> budget kept.
        assert!((reaction_torque_limit(0.05, 20.0, GraspMode::Pinch) - 0.05).abs() < 1e-9);
    }

    #[test]
    fn carry_a_max_reduces_to_dynamic_at_zero_disturbance_and_margin() {
        // m=0, D=0 -> identical to the plain dynamic-stability clamp.
        let c = carry_a_max(1.45, 3.0, 0.0, 0.0);
        let d = dynamic_a_max(1.45, 3.0);
        assert!((c - d).abs() < 1e-12, "carry {c} != dynamic {d}");
    }

    #[test]
    fn carry_a_max_is_below_dynamic_under_disturbance_and_margin() {
        // D, m > 0 strictly reserves capacity -> below the plain dynamic limit.
        assert!(carry_a_max(1.45, 3.0, 0.4, 0.5) < dynamic_a_max(1.45, 3.0));
    }

    #[test]
    fn carry_a_max_matches_worked_example_on_allegro() {
        // connector 1.45 N, pinch payload 3 N, D=0.4 N, m=0.5:
        // (3/1.5 - 0.4)/1.45 - 1 = 0.1034483 ; * g0 = 1.014481 m/s^2.
        let a = carry_a_max(1.45, 3.0, 0.4, 0.5);
        assert!((a - 1.014481).abs() < 1e-5, "got {a}");
    }

    #[test]
    fn carry_a_max_clamps_to_zero_when_headroom_is_insufficient() {
        // leap (payload 2) and pneumatic (payload 1.5) have no headroom for a 1.45 N
        // object + 0.4 N disturbance + 50% margin -> quasi-static (a_max = 0).
        assert_eq!(carry_a_max(1.45, 2.0, 0.4, 0.5), 0.0);
        assert_eq!(carry_a_max(1.45, 1.5, 0.4, 0.5), 0.0);
    }
}
