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
use rfl_core::quantity::Quantity;

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

impl Metric {
    /// Parse the ε-table schema's `metric` token; `None` for an unknown token.
    fn from_token(s: &str) -> Option<Metric> {
        match s {
            "geodesic_se3" => Some(Metric::GeodesicSe3),
            "geodesic_so3" => Some(Metric::GeodesicSo3),
            "l2_norm" => Some(Metric::L2Norm),
            "abs" => Some(Metric::Abs),
            _ => None,
        }
    }
}

/// The last telemetry sample carrying a wrench (the terminal contact reading).
fn last_wrench(r: &DriverReport) -> Option<&rfl_core::driver::Wrench> {
    r.telemetry.iter().rev().find_map(|t| t.wrench.as_ref())
}

/// The last telemetry value of a scalar `Quantity` field, parsed to its magnitude.
fn last_scalar(
    r: &DriverReport,
    f: fn(&rfl_core::driver::Telemetry) -> Option<&Quantity>,
) -> Option<f64> {
    r.telemetry
        .iter()
        .rev()
        .find_map(f)
        .and_then(|q| q.parse().map(|(v, _)| v))
}

/// The terminal-value extractor for a committed ε-table quantity *name*, or `None`
/// for a domain quantity that is not a first-class field of the driver-report wire
/// (`seating_depth`, `completion_torque`, `turns`, …) — those are category C:
/// not measurable from today's wire, reported `null` + `not_wire_derivable`.
fn wire_extractor(quantity: &str) -> Option<fn(&DriverReport) -> Option<Repr>> {
    match quantity {
        "realized_position" => {
            Some(|r| r.status.final_pose.as_ref().map(|p| Repr::Vec3(p.position)))
        }
        // `final_orientation` (in_hand.pivot) and `realized_orientation` (the split
        // force-pose) both read the terminal orientation.
        "realized_orientation" | "final_orientation" => Some(|r| {
            r.status
                .final_pose
                .as_ref()
                .map(|p| Repr::Quat(p.orientation))
        }),
        // `realized_wrench` is the contact-force magnitude (unit N) — the last
        // telemetry wrench's force vector under the l2_norm metric.
        "realized_wrench" => Some(|r| last_wrench(r).map(|w| Repr::Vec3(w.force))),
        "securing_force" => {
            Some(|r| last_scalar(r, |t| t.securing_force.as_ref()).map(Repr::Scalar))
        }
        _ => None,
    }
}

/// The committed ε-table structure (`schemas/epsilon-tolerances.yaml`), embedded so
/// the tool is self-contained. The tolerances there are all `null`; this is read
/// only for the *structure* — which `(primitive, quantity, metric, unit)` exist —
/// so the measured table aligns key-for-key with the normative one.
const EPSILON_TABLE_YAML: &str = include_str!("../../../schemas/epsilon-tolerances.yaml");

/// `primitive -> quantity -> (metric, unit)` parsed from the committed table.
type CommittedTable = BTreeMap<String, BTreeMap<String, (Metric, Option<String>)>>;

/// Parse the embedded committed table's structure (panics only on a corrupt
/// embedded asset, which CI's schema-validate would already have caught).
fn committed_table() -> CommittedTable {
    #[derive(serde::Deserialize)]
    struct Raw {
        epsilon_tolerances: BTreeMap<String, BTreeMap<String, RawEntry>>,
    }
    #[derive(serde::Deserialize)]
    struct RawEntry {
        metric: String,
        #[serde(default)]
        unit: Option<String>,
    }
    let raw: Raw = serde_yaml::from_str(EPSILON_TABLE_YAML).expect("embedded ε-table parses");
    raw.epsilon_tolerances
        .into_iter()
        .map(|(prim, quantities)| {
            let q = quantities
                .into_iter()
                .filter_map(|(name, e)| Metric::from_token(&e.metric).map(|m| (name, (m, e.unit))))
                .collect();
            (prim, q)
        })
        .collect()
}

/// One `(primitive, quantity)` provisional tolerance.
#[derive(Debug, Clone)]
pub struct ProvisionalEntry {
    /// The deviation metric (from the committed table).
    pub metric: Metric,
    /// The candidate ε (percentile × safety); `None` when not gradeable — either
    /// the quantity is not wire-derivable or no run-to-run sample was observed.
    /// Never a fabricated 0.
    pub tolerance: Option<f64>,
    /// The unit (from the committed table), if any.
    pub unit: Option<String>,
    /// Why `tolerance` is `null`, when it is: `not_wire_derivable` (a category-C
    /// domain quantity absent from the wire) or `not_reported` (wire-derivable but
    /// no run reported it). `None` when a candidate was measured.
    pub reason: Option<&'static str>,
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

/// Aggregate run-to-run deviations into a provisional ε table, **keyed by the
/// committed table** so the result aligns key-for-key with the normative one.
///
/// `action_primitives` maps each `action_id` to its primitive name; `runs` are the
/// per-run `replay_report` outputs with `runs[0]` the reference. For each action
/// whose primitive the committed table covers (the contact-dynamics set), every
/// committed quantity is emitted: a wire-derivable one reported by the reference
/// yields run-to-run deviation samples (pooled across actions sharing the
/// primitive) → an `epsilon_candidate`; a wire-derivable one no run reports →
/// `null` + `not_reported`; a category-C domain quantity → `null` +
/// `not_wire_derivable`. Primitives outside the committed table get no ε row.
#[must_use]
pub fn aggregate_epsilon(
    action_primitives: &BTreeMap<String, String>,
    runs: &[BTreeMap<String, DriverReport>],
    percentile: f64,
    safety: f64,
) -> ProvisionalTable {
    struct Bucket {
        metric: Metric,
        unit: Option<String>,
        wire_derivable: bool,
        samples: Vec<f64>,
    }
    let committed = committed_table();
    let n_runs = runs.len();
    let mut buckets: BTreeMap<(String, String), Bucket> = BTreeMap::new();
    let Some(reference) = runs.first() else {
        return ProvisionalTable {
            percentile,
            safety_factor: safety,
            tolerances: BTreeMap::new(),
        };
    };

    for (action_id, ref_report) in reference {
        let Some(primitive) = action_primitives.get(action_id) else {
            continue;
        };
        // ε applies only to contact-dynamics primitives — the committed table set.
        let Some(quantities) = committed.get(primitive) else {
            continue;
        };
        for (quantity, (metric, unit)) in quantities {
            let extractor = wire_extractor(quantity);
            let bucket = buckets
                .entry((primitive.clone(), quantity.clone()))
                .or_insert(Bucket {
                    metric: *metric,
                    unit: unit.clone(),
                    wire_derivable: extractor.is_some(),
                    samples: Vec::new(),
                });
            if let Some(extract) = extractor {
                if let Some(ref_val) = extract(ref_report) {
                    for run in &runs[1..] {
                        if let Some(run_report) = run.get(action_id) {
                            if let Some(run_val) = extract(run_report) {
                                if let Some(d) = deviation(*metric, &ref_val, &run_val) {
                                    bucket.samples.push(d);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut tolerances: BTreeMap<String, BTreeMap<String, ProvisionalEntry>> = BTreeMap::new();
    for ((primitive, quantity), bucket) in buckets {
        let tolerance = epsilon_candidate(&bucket.samples, percentile, safety);
        let reason = if !bucket.wire_derivable {
            Some("not_wire_derivable")
        } else if bucket.samples.is_empty() {
            Some("not_reported")
        } else {
            None
        };
        tolerances.entry(primitive).or_default().insert(
            quantity,
            ProvisionalEntry {
                metric: bucket.metric,
                tolerance,
                unit: bucket.unit,
                reason,
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

/// Build the `action_id -> primitive name` map for a retargeted skill. The id
/// format mirrors `rfl_core::canonical::to_jsonl` (`{skill}/{emb}/{NNNN}-{suffix}`,
/// one per sequence statement, in order), so the ids match what a driver echoes
/// back in its report.
#[must_use]
pub fn action_primitives(
    skill: &rfl_core::skill_isa::Skill,
    embodiment: &rfl_core::embodiment::Embodiment,
    out: &rfl_core::translation::RetargetOutput,
) -> BTreeMap<String, String> {
    use rfl_core::skill_isa::Statement;
    let mut map = BTreeMap::new();
    for (i, stmt) in skill.body.sequence.iter().enumerate() {
        let prim = match stmt {
            Statement::Primitive(p) => p,
            Statement::LetBind(b) => &b.from,
        };
        let id = format!(
            "{}/{}/{:04}-{}",
            skill.skill,
            embodiment.id,
            i + 1,
            out.suffixes[i]
        );
        map.insert(id, prim.name().to_string());
    }
    map
}

/// Measure a provisional ε table from N captured trace JSONLs for one
/// skill+embodiment. Retargets to recover the `action_id -> primitive` map,
/// schema-parses each trace via `replay::replay_report`, and aggregates the
/// run-to-run deviations. `runs_jsonl[0]` is the reference.
///
/// # Errors
/// Propagates a retarget error (e.g. `capability_absent`) or a malformed-trace
/// parse error.
pub fn measure_traces(
    skill: &rfl_core::skill_isa::Skill,
    embodiment: &rfl_core::embodiment::Embodiment,
    runs_jsonl: &[String],
    percentile: f64,
    safety: f64,
) -> anyhow::Result<ProvisionalTable> {
    let out = rfl_core::translation::retarget(skill, embodiment)?;
    let primitives = action_primitives(skill, embodiment, &out);
    let runs: Vec<_> = runs_jsonl
        .iter()
        .map(|j| crate::replay::replay_report(j))
        .collect::<anyhow::Result<_>>()?;
    Ok(aggregate_epsilon(&primitives, &runs, percentile, safety))
}

impl ProvisionalTable {
    /// Render the table as a provisional ε-tolerance YAML document. Deliberately
    /// distinct from the committed `schemas/epsilon-tolerances.yaml`: a header
    /// banner plus `provisional: true` / per-entry `source: measured` + sample
    /// provenance so a measured table can never be mistaken for the normative one.
    /// Deterministic (the maps are `BTreeMap`s).
    #[must_use]
    pub fn to_provisional_yaml(&self) -> String {
        let mut s = String::new();
        s.push_str(
            "# PROVISIONAL: measured, NOT normative. Do not commit as schemas/epsilon-tolerances.yaml.\n",
        );
        s.push_str("provisional: true\n");
        s.push_str("source: measured\n");
        s.push_str(&format!("percentile: {}\n", self.percentile));
        s.push_str(&format!("safety_factor: {}\n", self.safety_factor));
        s.push_str("epsilon_tolerances:\n");
        for (primitive, quantities) in &self.tolerances {
            s.push_str(&format!("  {primitive}:\n"));
            for (quantity, e) in quantities {
                s.push_str(&format!("    {quantity}:\n"));
                s.push_str(&format!("      metric: {}\n", e.metric.as_str()));
                match e.tolerance {
                    Some(t) => s.push_str(&format!("      tolerance: {t}\n")),
                    None => s.push_str("      tolerance: null\n"),
                }
                if let Some(reason) = e.reason {
                    s.push_str(&format!("      reason: {reason}\n"));
                }
                if let Some(unit) = &e.unit {
                    s.push_str(&format!("      unit: {unit}\n"));
                }
                s.push_str(&format!("      n_runs: {}\n", e.n_runs));
                s.push_str(&format!("      n_samples: {}\n", e.n_samples));
            }
        }
        s
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

    // ---- aggregation (keyed by the committed table) ----

    use rfl_core::driver::{DriverReport, Outcome, RealizedPose, Status, Telemetry, Wrench};
    use rfl_core::quantity::Quantity;

    const IDENTITY: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
    // A 90-degree rotation about z as [x, y, z, w] (geodesic distance pi/2 from identity).
    const QUARTER_TURN_Z: [f64; 4] = [
        0.0,
        0.0,
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ];

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

    fn blank_telemetry() -> Telemetry {
        Telemetry {
            message: "telemetry",
            action_id: String::new(),
            t: 0.0,
            realized_pose: None,
            wrench: None,
            securing_force: None,
            station_error: None,
            tactile: vec![],
            events: vec![],
            fidelity_tier: None,
            contact_geometry: None,
            measured_quantities: BTreeMap::new(),
        }
    }

    fn telemetry_force(force: [f64; 3]) -> Telemetry {
        Telemetry {
            wrench: Some(Wrench {
                force,
                torque: [0.0, 0.0, 0.0],
            }),
            ..blank_telemetry()
        }
    }

    fn telemetry_securing(newtons: f64) -> Telemetry {
        Telemetry {
            securing_force: Some(Quantity::from_si(newtons, "N")),
            ..blank_telemetry()
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
    fn committed_table_covers_the_contact_dynamics_set() {
        let t = committed_table();
        assert_eq!(t.len(), 11); // 10 force.* + in_hand.pivot
        let insert = &t["force.insert_fit"];
        assert!(insert.contains_key("realized_position"));
        assert!(insert.contains_key("realized_orientation"));
        assert!(insert.contains_key("realized_wrench"));
        assert!(insert.contains_key("seating_depth"));
        assert!(t.contains_key("in_hand.pivot"));
    }

    #[test]
    fn in_hand_pivot_is_fully_wire_measurable() {
        // Orientation turns a quarter turn (pi/2) and securing force shifts 8 -> 8.5 N.
        let r0 = report(
            vec![telemetry_securing(8.0)],
            status_with_pose([0.0; 3], IDENTITY),
        );
        let r1 = report(
            vec![telemetry_securing(8.5)],
            status_with_pose([0.0; 3], QUARTER_TURN_Z),
        );
        let runs = vec![run("p/e/0001-pivot", r0), run("p/e/0001-pivot", r1)];
        let table = aggregate_epsilon(
            &prim_map("p/e/0001-pivot", "in_hand.pivot"),
            &runs,
            0.95,
            1.0,
        );
        let pivot = &table.tolerances["in_hand.pivot"];
        // Both committed quantities are wire-derivable and reported -> non-null, reason None.
        assert!(
            (pivot["final_orientation"].tolerance.unwrap() - std::f64::consts::FRAC_PI_2).abs()
                < 1e-9
        );
        assert!(pivot["final_orientation"].reason.is_none());
        assert!((pivot["securing_force"].tolerance.unwrap() - 0.5).abs() < 1e-12);
        assert!(pivot["securing_force"].reason.is_none());
    }

    #[test]
    fn insert_fit_fills_pose_and_wrench_and_nulls_seating_depth() {
        // Reference vs a run deviating 0.2 m in z and 1 N in force; orientation steady.
        let r0 = report(
            vec![telemetry_force([10.0, 0.0, 0.0])],
            status_with_pose([0.0; 3], IDENTITY),
        );
        let r1 = report(
            vec![telemetry_force([11.0, 0.0, 0.0])],
            status_with_pose([0.0, 0.0, 0.2], IDENTITY),
        );
        let runs = vec![run("c/e/0001-insert", r0), run("c/e/0001-insert", r1)];
        let table = aggregate_epsilon(
            &prim_map("c/e/0001-insert", "force.insert_fit"),
            &runs,
            0.95,
            1.0,
        );
        let f = &table.tolerances["force.insert_fit"];
        assert!((f["realized_position"].tolerance.unwrap() - 0.2).abs() < 1e-12);
        assert_eq!(f["realized_orientation"].tolerance, Some(0.0)); // steady orientation
        assert!((f["realized_wrench"].tolerance.unwrap() - 1.0).abs() < 1e-12);
        // seating_depth is not a wire field -> null + reason, never a fabricated value.
        assert_eq!(f["seating_depth"].tolerance, None);
        assert_eq!(f["seating_depth"].reason, Some("not_wire_derivable"));
    }

    #[test]
    fn screw_domain_quantities_are_not_wire_derivable() {
        let r = || report(vec![], status_with_pose([0.0; 3], IDENTITY));
        let runs = vec![run("s/e/0001-screw", r()), run("s/e/0001-screw", r())];
        let table = aggregate_epsilon(&prim_map("s/e/0001-screw", "force.screw"), &runs, 0.95, 1.0);
        let screw = &table.tolerances["force.screw"];
        // completion_torque and turns are not on the generic wire.
        for q in ["completion_torque", "turns"] {
            assert_eq!(screw[q].tolerance, None);
            assert_eq!(screw[q].reason, Some("not_wire_derivable"));
        }
    }

    #[test]
    fn non_contact_primitive_gets_no_epsilon_row() {
        // grasp.pinch is not in the committed (contact-dynamics) table -> no ε.
        let r = || report(vec![], status_with_pose([0.0; 3], IDENTITY));
        let runs = vec![run("g/e/0001-pinch", r()), run("g/e/0001-pinch", r())];
        let table = aggregate_epsilon(&prim_map("g/e/0001-pinch", "grasp.pinch"), &runs, 0.95, 1.0);
        assert!(table.tolerances.is_empty());
    }

    #[test]
    fn provisional_yaml_carries_the_honesty_firewall() {
        // Position deviates 0.5 m -> 0.6 candidate; wrench steady -> 0; seating_depth null.
        let r0 = report(
            vec![telemetry_force([10.0, 0.0, 0.0])],
            status_with_pose([0.0; 3], IDENTITY),
        );
        let r1 = report(
            vec![telemetry_force([10.0, 0.0, 0.0])],
            status_with_pose([0.0, 0.0, 0.5], IDENTITY),
        );
        let runs = vec![run("c/e/0001-insert", r0), run("c/e/0001-insert", r1)];
        let table = aggregate_epsilon(
            &prim_map("c/e/0001-insert", "force.insert_fit"),
            &runs,
            0.95,
            1.2,
        );
        let yaml = table.to_provisional_yaml();
        assert!(yaml.starts_with("# PROVISIONAL"));
        assert!(yaml.contains("provisional: true"));
        assert!(yaml.contains("source: measured"));
        assert!(yaml.contains("force.insert_fit"));
        assert!(yaml.contains("realized_position"));
        assert!(yaml.contains("tolerance: 0.6")); // 0.5 * 1.2
        // The category-C quantity renders null with its reason.
        assert!(yaml.contains("tolerance: null"));
        assert!(yaml.contains("reason: not_wire_derivable"));
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
