// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The ε measurement harness (`spec/02` RD2c, `spec/05` § Determinism floor).
//!
//! Class 2-loose conformance holds a contact-dynamics primitive's *realized*
//! execution to semantic equivalence within a per-skill tolerance ε. The ε is an
//! **empirical** quantity: the realized-execution deviation a conformant
//! implementation exhibits run to run (and across embodiments). This module is
//! the tool that *produces* those values — given the realized traces of the same
//! quantity across N conformant runs, it computes the per-quantity deviation
//! under the ε-table metric and the candidate ε (a high percentile + safety
//! factor) that fills `schemas/epsilon-tolerances.yaml` (currently `null`).
//!
//! The harness is ready now; the **values** stay pending real multi-run traces
//! from a reference driver or a (declared-conformant) simulator — a single
//! nominal report cannot exhibit run-to-run variation. See
//! `docs/design/2026-06-01-epsilon-tolerance-table-design.md` for the harness
//! protocol.

use std::collections::BTreeMap;

use nalgebra::{Quaternion, UnitQuaternion};
use rfl_core::driver::DriverReport;
use rfl_core::pose::{Pose6D, theta_orient};

/// The comparison metric for a measured quantity (mirrors the ε-table schema's
/// `metric` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    /// SE(3): position L2 (m) + orientation geodesic (rad), reported separately.
    GeodesicSe3,
    /// SO(3): orientation geodesic angle (rad).
    GeodesicSo3,
    /// L2 norm of a vector difference (e.g. wrench force, N).
    L2Norm,
    /// Absolute difference of a scalar.
    Abs,
}

/// Absolute deviation of two scalars (`Abs`).
#[must_use]
pub fn abs_dev(a: f64, b: f64) -> f64 {
    (a - b).abs()
}

/// L2 deviation of two equal-length vectors (`L2Norm`); `f64::NAN` on a length
/// mismatch (an ill-formed comparison).
#[must_use]
pub fn l2_dev(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::NAN;
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// SE(3) deviation of two poses (`GeodesicSe3`): `(translation_m, rotation_rad)`
/// — kept separate because the two have different units (the ε-table carries one
/// tolerance per quantity, so pose splits into a position and an orientation row).
#[must_use]
pub fn se3_dev(a: &Pose6D, b: &Pose6D) -> (f64, f64) {
    let t = l2_dev(&a.position, &b.position);
    let r = theta_orient(&a.orientation, &b.orientation);
    (t, r)
}

/// SO(3) deviation of two orientations (`GeodesicSo3`), in radians.
#[must_use]
pub fn so3_dev(a: &UnitQuaternion<f64>, b: &UnitQuaternion<f64>) -> f64 {
    theta_orient(a, b)
}

/// The candidate ε for a quantity from a sample of run-to-run deviations: the
/// `percentile` (nearest-rank, deterministic) scaled by `safety_factor`.
/// `percentile` is in `[0, 1]`; `None` for an empty sample (not yet gradeable).
#[must_use]
pub fn epsilon_candidate(deviations: &[f64], percentile: f64, safety_factor: f64) -> Option<f64> {
    if deviations.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = deviations
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p = percentile.clamp(0.0, 1.0);
    // Nearest-rank: rank = ceil(p * n), clamped to [1, n].
    let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1] * safety_factor)
}

/// Run-to-run deviations of a scalar quantity across `runs`, each compared to the
/// first run (the reference). Empty / single-run input yields no deviations.
#[must_use]
pub fn run_to_run_abs(runs: &[f64]) -> Vec<f64> {
    match runs.split_first() {
        Some((reference, rest)) => rest.iter().map(|r| abs_dev(*reference, *r)).collect(),
        None => vec![],
    }
}

impl Metric {
    /// The ε-table schema's `metric` token for this metric.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Metric::GeodesicSe3 => "geodesic_se3",
            Metric::GeodesicSo3 => "geodesic_so3",
            Metric::L2Norm => "l2_norm",
            Metric::Abs => "abs",
        }
    }
}

/// A representative realized value extracted from one run's action, in the form
/// the deviation metric consumes.
#[derive(Debug, Clone)]
enum Repr {
    /// A scalar reading (e.g. securing force, station error).
    Scalar(f64),
    /// A 3-vector (position, force, torque).
    Vec3([f64; 3]),
    /// An orientation quaternion `[x, y, z, w]`.
    Quat([f64; 4]),
}

/// The deviation between a reference and a run value under `metric`; `None` when
/// the metric and the representations do not match (an ill-formed comparison).
fn deviation(metric: Metric, reference: &Repr, run: &Repr) -> Option<f64> {
    match (metric, reference, run) {
        (Metric::Abs, Repr::Scalar(a), Repr::Scalar(b)) => Some(abs_dev(*a, *b)),
        (Metric::L2Norm, Repr::Vec3(a), Repr::Vec3(b)) => Some(l2_dev(a, b)),
        (Metric::GeodesicSo3, Repr::Quat(a), Repr::Quat(b)) => {
            Some(so3_dev(&unit_quat(a), &unit_quat(b)))
        }
        _ => None,
    }
}

/// `[x, y, z, w]` -> a (renormalized) unit quaternion.
fn unit_quat(q: &[f64; 4]) -> UnitQuaternion<f64> {
    UnitQuaternion::from_quaternion(Quaternion::new(q[3], q[0], q[1], q[2]))
}

/// One measured quantity: how to pull its representative value out of a run's
/// terminal report, and the ε-table metric / unit it records under.
struct QuantitySpec {
    /// The ε-table quantity key.
    key: &'static str,
    /// The deviation metric.
    metric: Metric,
    /// The SI unit of the tolerance.
    unit: &'static str,
    /// Extractor: the terminal representative value, if the action reports it.
    extract: fn(&DriverReport) -> Option<Repr>,
}

/// The last telemetry sample carrying a wrench (the terminal contact reading).
fn last_wrench(r: &DriverReport) -> Option<&rfl_core::driver::Wrench> {
    r.telemetry.iter().rev().find_map(|t| t.wrench.as_ref())
}

/// The terminal-value quantity set (v0): one representative per action per run,
/// taken from `status.final_pose` and the last telemetry sample — no time-series
/// alignment (which would need a cross-run sample-matching rule).
const QUANTITIES: &[QuantitySpec] = &[
    QuantitySpec {
        key: "final_position",
        metric: Metric::L2Norm,
        unit: "m",
        extract: |r| r.status.final_pose.as_ref().map(|p| Repr::Vec3(p.position)),
    },
    QuantitySpec {
        key: "final_orientation",
        metric: Metric::GeodesicSo3,
        unit: "rad",
        extract: |r| {
            r.status
                .final_pose
                .as_ref()
                .map(|p| Repr::Quat(p.orientation))
        },
    },
    QuantitySpec {
        key: "wrench_force",
        metric: Metric::L2Norm,
        unit: "N",
        extract: |r| last_wrench(r).map(|w| Repr::Vec3(w.force)),
    },
    QuantitySpec {
        key: "wrench_torque",
        metric: Metric::L2Norm,
        unit: "N*m",
        extract: |r| last_wrench(r).map(|w| Repr::Vec3(w.torque)),
    },
    QuantitySpec {
        key: "securing_force",
        metric: Metric::Abs,
        unit: "N",
        extract: |r| {
            r.telemetry
                .iter()
                .rev()
                .find_map(|t| t.securing_force.as_ref())
                .and_then(|q| q.parse().map(|(v, _)| Repr::Scalar(v)))
        },
    },
    QuantitySpec {
        key: "station_error",
        metric: Metric::Abs,
        unit: "m",
        extract: |r| {
            r.telemetry
                .iter()
                .rev()
                .find_map(|t| t.station_error.as_ref())
                .and_then(|q| q.parse().map(|(v, _)| Repr::Scalar(v)))
        },
    },
];

/// One `(primitive, quantity)` provisional tolerance.
#[derive(Debug, Clone)]
pub struct ProvisionalEntry {
    /// The deviation metric.
    pub metric: Metric,
    /// The candidate ε (percentile × safety); `None` when fewer than one
    /// run-to-run sample was observed (not yet gradeable — never a fabricated 0).
    pub tolerance: Option<f64>,
    /// The SI unit.
    pub unit: &'static str,
    /// The number of runs supplied.
    pub n_runs: usize,
    /// The number of run-to-run deviation samples observed for this quantity.
    pub n_samples: usize,
}

/// A provisional ε-tolerance table measured from real traces. Distinct from the
/// committed `schemas/epsilon-tolerances.yaml` (all `null`): this carries
/// measured values + provenance and is **never** the normative table.
#[derive(Debug, Clone)]
pub struct ProvisionalTable {
    /// The percentile used for `epsilon_candidate`.
    pub percentile: f64,
    /// The safety factor used for `epsilon_candidate`.
    pub safety_factor: f64,
    /// `primitive -> quantity -> entry`.
    pub tolerances: BTreeMap<String, BTreeMap<String, ProvisionalEntry>>,
}

/// Aggregate run-to-run deviations into a provisional ε table.
///
/// `action_primitives` maps each `action_id` to its primitive name (an unmapped
/// id falls under `__unmapped__`, never dropped). `runs` are the per-run
/// `replay_report` outputs; `runs[0]` is the reference each later run is compared
/// to. For each action's primitive, every terminal quantity present in the
/// reference yields a `(primitive, quantity)` entry; its samples pool across all
/// actions sharing the primitive. A quantity present in the reference but in no
/// later run gets an entry with `tolerance: None` (honest "not yet gradeable").
#[must_use]
pub fn aggregate_epsilon(
    action_primitives: &BTreeMap<String, String>,
    runs: &[BTreeMap<String, DriverReport>],
    percentile: f64,
    safety: f64,
) -> ProvisionalTable {
    // The accumulating deviation samples for one (primitive, quantity); created
    // when the reference reports the quantity, so its mere presence == applicable.
    struct Bucket {
        metric: Metric,
        unit: &'static str,
        samples: Vec<f64>,
    }
    // (primitive, quantity-key) -> bucket.
    let mut buckets: BTreeMap<(String, &'static str), Bucket> = BTreeMap::new();
    let n_runs = runs.len();
    let Some(reference) = runs.first() else {
        return ProvisionalTable {
            percentile,
            safety_factor: safety,
            tolerances: BTreeMap::new(),
        };
    };

    for (action_id, ref_report) in reference {
        let primitive = action_primitives
            .get(action_id)
            .map_or("__unmapped__", String::as_str);
        for spec in QUANTITIES {
            let Some(ref_val) = (spec.extract)(ref_report) else {
                continue;
            };
            let entry = buckets
                .entry((primitive.to_string(), spec.key))
                .or_insert(Bucket {
                    metric: spec.metric,
                    unit: spec.unit,
                    samples: Vec::new(),
                });
            for run in &runs[1..] {
                if let Some(run_report) = run.get(action_id) {
                    if let Some(run_val) = (spec.extract)(run_report) {
                        if let Some(d) = deviation(spec.metric, &ref_val, &run_val) {
                            entry.samples.push(d);
                        }
                    }
                }
            }
        }
    }

    let mut tolerances: BTreeMap<String, BTreeMap<String, ProvisionalEntry>> = BTreeMap::new();
    for ((primitive, quantity), bucket) in buckets {
        let tolerance = epsilon_candidate(&bucket.samples, percentile, safety);
        tolerances.entry(primitive).or_default().insert(
            quantity.to_string(),
            ProvisionalEntry {
                metric: bucket.metric,
                tolerance,
                unit: bucket.unit,
                n_runs,
                n_samples: bucket.samples.len(),
            },
        );
    }

    ProvisionalTable {
        percentile,
        safety_factor: safety,
        tolerances,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use super::*;

    #[test]
    fn scalar_and_vector_deviations() {
        assert_eq!(abs_dev(5.0, 5.0), 0.0);
        assert_eq!(abs_dev(5.0, 3.5), 1.5);
        assert_eq!(l2_dev(&[3.0, 4.0, 0.0], &[0.0, 0.0, 0.0]), 5.0);
        assert!(l2_dev(&[1.0], &[1.0, 2.0]).is_nan()); // length mismatch
    }

    #[test]
    fn se3_deviation_splits_translation_and_rotation() {
        let a = Pose6D {
            position: [0.0, 0.0, 0.0],
            orientation: UnitQuaternion::identity(),
        };
        let b = Pose6D {
            position: [0.0, 0.0, 0.2],
            orientation: UnitQuaternion::identity(),
        };
        let (t, r) = se3_dev(&a, &b);
        assert!((t - 0.2).abs() < 1e-12);
        assert!(r.abs() < 1e-12);
        // A pure 90-degree rotation: zero translation, pi/2 geodesic.
        let c = Pose6D {
            position: [0.0, 0.0, 0.0],
            orientation: UnitQuaternion::from_axis_angle(
                &nalgebra::Vector3::z_axis(),
                std::f64::consts::FRAC_PI_2,
            ),
        };
        let (t2, r2) = se3_dev(&a, &c);
        assert!(t2.abs() < 1e-12);
        assert!((r2 - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn epsilon_candidate_is_a_scaled_percentile() {
        // Deviations 0.0..0.9 (10 samples); p95 nearest-rank = ceil(0.95*10)=10th
        // = 0.9; with a 1.2 safety factor -> 1.08.
        let devs: Vec<f64> = (0..10).map(|i| f64::from(i) / 10.0).collect();
        let eps = epsilon_candidate(&devs, 0.95, 1.2).unwrap();
        assert!((eps - 0.9 * 1.2).abs() < 1e-12);
        // Identical runs -> zero deviation -> zero epsilon.
        assert_eq!(epsilon_candidate(&[0.0, 0.0, 0.0], 0.95, 1.5), Some(0.0));
        // Empty sample -> not yet gradeable.
        assert_eq!(epsilon_candidate(&[], 0.95, 1.0), None);
    }

    // ---- aggregation ----

    use rfl_core::driver::{DriverReport, Outcome, RealizedPose, Status, Telemetry, Wrench};

    fn status_with_pose(position: [f64; 3], orientation: [f64; 4]) -> Status {
        Status {
            message: "status",
            action_id: String::new(),
            outcome: Outcome::Succeeded,
            verdict: None,
            fidelity_tier: None,
            final_pose: Some(RealizedPose {
                position,
                orientation,
            }),
            failure_class: None,
            failure_detail: None,
            stop_latency: None,
            safety_flags: None,
        }
    }

    fn telemetry_force(force: [f64; 3]) -> Telemetry {
        Telemetry {
            message: "telemetry",
            action_id: String::new(),
            t: 0.0,
            realized_pose: None,
            wrench: Some(Wrench {
                force,
                torque: [0.0, 0.0, 0.0],
            }),
            securing_force: None,
            station_error: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: None,
            contact_geometry: None,
        }
    }

    fn report(telemetry: Vec<Telemetry>, status: Status) -> DriverReport {
        DriverReport { telemetry, status }
    }

    fn run(action_id: &str, r: DriverReport) -> BTreeMap<String, DriverReport> {
        let mut m = BTreeMap::new();
        m.insert(action_id.to_string(), r);
        m
    }

    fn prim_map(action_id: &str, primitive: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(action_id.to_string(), primitive.to_string());
        m
    }

    #[test]
    fn identical_runs_yield_zero_tolerance() {
        let pose = || {
            report(
                vec![],
                status_with_pose([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0]),
            )
        };
        let runs = vec![run("a/b/0001-x", pose()), run("a/b/0001-x", pose())];
        let table = aggregate_epsilon(&prim_map("a/b/0001-x", "force.screw"), &runs, 0.95, 1.2);
        let entry = &table.tolerances["force.screw"]["final_position"];
        assert_eq!(entry.tolerance, Some(0.0));
        assert_eq!(entry.n_runs, 2);
        assert_eq!(entry.n_samples, 1);
    }

    #[test]
    fn known_deviation_is_the_scaled_percentile() {
        // Reference position at origin; the run is 0.5 m away in z.
        let r0 = report(
            vec![],
            status_with_pose([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
        );
        let r1 = report(
            vec![],
            status_with_pose([0.0, 0.0, 0.5], [0.0, 0.0, 0.0, 1.0]),
        );
        let runs = vec![run("a/b/0001-x", r0), run("a/b/0001-x", r1)];
        let table = aggregate_epsilon(&prim_map("a/b/0001-x", "force.screw"), &runs, 0.95, 1.2);
        let eps = table.tolerances["force.screw"]["final_position"]
            .tolerance
            .unwrap();
        assert!((eps - 0.5 * 1.2).abs() < 1e-12);
    }

    #[test]
    fn quantity_present_in_one_run_only_is_not_gradeable() {
        // Reference has a wrench; the second run does not -> no deviation sample.
        let r0 = report(
            vec![telemetry_force([10.0, 0.0, 0.0])],
            status_with_pose([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
        );
        let r1 = report(
            vec![],
            status_with_pose([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
        );
        let runs = vec![run("a/b/0001-x", r0), run("a/b/0001-x", r1)];
        let table = aggregate_epsilon(&prim_map("a/b/0001-x", "force.screw"), &runs, 0.95, 1.2);
        let force = &table.tolerances["force.screw"]["wrench_force"];
        assert_eq!(force.tolerance, None);
        assert_eq!(force.n_samples, 0);
    }

    #[test]
    fn two_actions_sharing_a_primitive_pool_samples() {
        // Two force.screw actions, each deviating in position; samples pool.
        let mut reference = BTreeMap::new();
        reference.insert(
            "a/b/0001-screw".to_string(),
            report(vec![], status_with_pose([0.0; 3], [0.0, 0.0, 0.0, 1.0])),
        );
        reference.insert(
            "a/b/0002-screw".to_string(),
            report(vec![], status_with_pose([0.0; 3], [0.0, 0.0, 0.0, 1.0])),
        );
        let mut run1 = BTreeMap::new();
        run1.insert(
            "a/b/0001-screw".to_string(),
            report(
                vec![],
                status_with_pose([0.0, 0.0, 0.2], [0.0, 0.0, 0.0, 1.0]),
            ),
        );
        run1.insert(
            "a/b/0002-screw".to_string(),
            report(
                vec![],
                status_with_pose([0.0, 0.0, 0.4], [0.0, 0.0, 0.0, 1.0]),
            ),
        );
        let mut prims = BTreeMap::new();
        prims.insert("a/b/0001-screw".to_string(), "force.screw".to_string());
        prims.insert("a/b/0002-screw".to_string(), "force.screw".to_string());
        let table = aggregate_epsilon(&prims, &[reference, run1], 0.95, 1.0);
        let entry = &table.tolerances["force.screw"]["final_position"];
        assert_eq!(entry.n_samples, 2); // both actions pooled
        // p95 nearest-rank over {0.2, 0.4} = 0.4; safety 1.0.
        assert!((entry.tolerance.unwrap() - 0.4).abs() < 1e-12);
    }

    #[test]
    fn unmapped_action_id_falls_under_the_unmapped_bucket() {
        let pose = || {
            report(
                vec![],
                status_with_pose([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
            )
        };
        let runs = vec![run("a/b/0001-x", pose()), run("a/b/0001-x", pose())];
        // Empty primitive map -> the action is unmapped, not dropped.
        let table = aggregate_epsilon(&BTreeMap::new(), &runs, 0.95, 1.2);
        assert!(table.tolerances.contains_key("__unmapped__"));
    }

    #[test]
    fn run_to_run_compares_against_the_reference_run() {
        // Three runs of a scalar; deviations vs the first.
        let devs = run_to_run_abs(&[10.0, 10.2, 9.7]);
        assert_eq!(devs.len(), 2);
        assert!((devs[0] - 0.2).abs() < 1e-9);
        assert!((devs[1] - 0.3).abs() < 1e-9);
        assert!(run_to_run_abs(&[10.0]).is_empty()); // a single run has no variation
    }
}
