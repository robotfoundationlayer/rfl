// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! A stochastic reference simulator: the deterministic `ReferenceDriver` perturbed
//! by a DECLARED per-quantity noise model, so `rfl measure` over N seeds yields a
//! PROVISIONAL (sim-derived, never normative) ε. The σ live in the pending
//! `simulator-declaration.yaml` `variation_model`; for ε only σ matters (the
//! nominal cancels out of the run-to-run deviation), so the candidate ε is purely a
//! function of the declared σ — it can never be mistaken for a physical tolerance.
//! See `docs/design/2026-06-02-stochastic-reference-sim-design.md`.

use std::collections::BTreeMap;

use anyhow::Result;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rfl_core::canonical::ExecuteGoal;
use rfl_core::driver::{Driver, DriverReport, RealizedPose};
use rfl_core::embodiment::Embodiment;
use rfl_core::quantity::Quantity;
use rfl_core::skill_isa::Skill;

use crate::ReferenceDriver;
use crate::measure::{CommittedTable, action_primitives, committed_table};

/// One quantity's declared variation: nominal + run-to-run σ (+ unit).
#[derive(Debug, Clone)]
pub struct QuantityNoise {
    /// Declared nominal (cancels out of the run-to-run deviation; for plausible values).
    pub nominal: f64,
    /// Declared run-to-run standard deviation in the quantity's unit.
    pub sigma: f64,
    /// The quantity's unit.
    pub unit: String,
}

/// A declared per-quantity noise model (the simulator-declaration `variation_model`).
pub type NoiseModel = BTreeMap<String, QuantityNoise>;

/// Parse the `variation_model` out of a simulator-declaration YAML (empty if absent).
///
/// # Errors
/// A malformed declaration YAML.
pub fn parse_variation_model(declaration_yaml: &str) -> Result<NoiseModel> {
    #[derive(serde::Deserialize)]
    struct Decl {
        #[serde(default)]
        variation_model: BTreeMap<String, RawNoise>,
    }
    #[derive(serde::Deserialize)]
    struct RawNoise {
        nominal: f64,
        sigma: f64,
        #[serde(default)]
        unit: Option<String>,
    }
    let decl: Decl = serde_yaml::from_str(declaration_yaml)?;
    Ok(decl
        .variation_model
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                QuantityNoise {
                    nominal: v.nominal,
                    sigma: v.sigma,
                    unit: v.unit.unwrap_or_default(),
                },
            )
        })
        .collect())
}

/// A Gaussian sample `N(0, sigma)` via Box-Muller over two uniforms (no extra dep).
fn gaussian(rng: &mut StdRng, sigma: f64) -> f64 {
    if sigma == 0.0 {
        return 0.0;
    }
    let u1 = rng.gen_range(0.0_f64..1.0).max(f64::MIN_POSITIVE); // avoid ln(0)
    let u2 = rng.gen_range(0.0_f64..1.0);
    let mag = (-2.0 * u1.ln()).sqrt();
    sigma * mag * (2.0 * std::f64::consts::PI * u2).cos()
}

/// The quantity names perturbed via existing report fields (the kinematic / wrench /
/// securing-force channels); every other committed quantity rides `measured_quantities`.
const FIELD_QUANTITIES: &[&str] = &[
    "realized_position",
    "realized_orientation",
    "final_orientation",
    "realized_wrench",
    "securing_force",
];

fn sigma_of(model: &NoiseModel, name: &str) -> f64 {
    model.get(name).map_or(0.0, |q| q.sigma)
}

/// Perturb a quaternion `[x, y, z, w]` by a rotation of `angle` about +z (Hamilton
/// product of the z-axis delta quaternion with `q`).
fn perturb_quat(q: [f64; 4], angle: f64) -> [f64; 4] {
    let (s, c) = ((angle / 2.0).sin(), (angle / 2.0).cos());
    let [x, y, z, w] = q;
    [c * x - s * y, c * y + s * x, c * z + s * w, c * w - s * z]
}

fn perturb_pose(p: &mut RealizedPose, model: &NoiseModel, rng: &mut StdRng) {
    let sp = sigma_of(model, "realized_position");
    for axis in &mut p.position {
        *axis += gaussian(rng, sp);
    }
    // realized_orientation (force.*) and final_orientation (in_hand.pivot) both read
    // status.final_pose.orientation; use whichever σ the model declares.
    let so = sigma_of(model, "realized_orientation").max(sigma_of(model, "final_orientation"));
    p.orientation = perturb_quat(p.orientation, gaussian(rng, so));
}

/// Perturb one nominal report in place for the current run, and emit the primitive's
/// domain scalars into the last telemetry sample's `measured_quantities`.
fn perturb_report(
    report: &mut DriverReport,
    primitive: Option<&str>,
    committed: &CommittedTable,
    model: &NoiseModel,
    rng: &mut StdRng,
) {
    for t in &mut report.telemetry {
        if let Some(pose) = &mut t.realized_pose {
            perturb_pose(pose, model, rng);
        }
        if let Some(w) = &mut t.wrench {
            let s = sigma_of(model, "realized_wrench");
            for f in &mut w.force {
                *f += gaussian(rng, s);
            }
        }
        if let Some((v, u)) = t
            .securing_force
            .as_ref()
            .and_then(|q| q.parse().map(|(v, u)| (v, u.to_string())))
        {
            let nv = v + gaussian(rng, sigma_of(model, "securing_force"));
            t.securing_force = Some(Quantity::from_si(nv, &u));
        }
    }
    if let Some(pose) = &mut report.status.final_pose {
        perturb_pose(pose, model, rng);
    }

    // Domain scalars: emit measured_quantities for the primitive's category-C quantities.
    let Some(primitive) = primitive else {
        return;
    };
    let Some(quantities) = committed.get(primitive) else {
        return;
    };
    let Some(last) = report.telemetry.last_mut() else {
        return;
    };
    for name in quantities.keys() {
        if FIELD_QUANTITIES.contains(&name.as_str()) {
            continue;
        }
        if let Some(noise) = model.get(name) {
            let value = noise.nominal + gaussian(rng, noise.sigma);
            last.measured_quantities
                .insert(name.clone(), Quantity::from_si(value, &noise.unit));
        }
    }
}

/// Generate a stochastic driver-report stream for one run, seeded by `seed`. The
/// deterministic nominal reference driver's output is perturbed by `model`. Same
/// seed → identical output (reproducible); different seeds → varied realized values.
/// The result is a PROVISIONAL source for `rfl measure`, never normative.
///
/// # Errors
/// A retarget error (e.g. `capability_absent`).
pub fn stochastic_run(
    skill: &Skill,
    embodiment: &Embodiment,
    model: &NoiseModel,
    seed: u64,
) -> Result<Vec<DriverReport>> {
    let out = rfl_core::translation::retarget(skill, embodiment)?;
    let prims = action_primitives(skill, embodiment, &out);
    let committed = committed_table();
    let mut driver = ReferenceDriver::default();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut reports = Vec::with_capacity(out.actions.len());
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let action_id = format!("{}/{}/{:04}-{}", skill.skill, embodiment.id, i + 1, suffix);
        let goal = ExecuteGoal::wrap(action_id.clone(), action.clone());
        let mut report = driver.execute(&goal);
        perturb_report(
            &mut report,
            prims.get(&action_id).map(String::as_str),
            &committed,
            model,
            &mut rng,
        );
        reports.push(report);
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports_to_jsonl;
    use std::path::Path;

    fn cable() -> (Skill, Embodiment) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion");
        let skill = Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill.yaml")).unwrap())
            .expect("skill");
        let emb = Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .expect("emb");
        (skill, emb)
    }

    fn model() -> NoiseModel {
        let decl = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/simulator-declaration.yaml"),
        )
        .unwrap();
        parse_variation_model(&decl).expect("model")
    }

    #[test]
    fn gaussian_is_reproducible_and_roughly_calibrated() {
        let mut a = StdRng::seed_from_u64(42);
        let mut b = StdRng::seed_from_u64(42);
        assert_eq!(gaussian(&mut a, 1.0), gaussian(&mut b, 1.0)); // reproducible
        let mut rng = StdRng::seed_from_u64(7);
        let n = 20_000;
        let xs: Vec<f64> = (0..n).map(|_| gaussian(&mut rng, 2.0)).collect();
        let mean = xs.iter().sum::<f64>() / f64::from(n);
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / f64::from(n);
        assert!(mean.abs() < 0.1, "mean {mean}"); // ~0
        assert!((var.sqrt() - 2.0).abs() < 0.2, "stdev {}", var.sqrt()); // ~σ=2
    }

    #[test]
    fn variation_model_parses_the_committed_declaration() {
        let m = model();
        assert!(m.contains_key("seating_depth"));
        assert!((m["realized_position"].sigma - 0.001).abs() < 1e-12);
    }

    #[test]
    fn same_seed_is_reproducible_different_seeds_differ() {
        let (s, e) = cable();
        let m = model();
        let a = reports_to_jsonl(&stochastic_run(&s, &e, &m, 3).unwrap());
        let b = reports_to_jsonl(&stochastic_run(&s, &e, &m, 3).unwrap());
        let c = reports_to_jsonl(&stochastic_run(&s, &e, &m, 4).unwrap());
        assert_eq!(a, b); // same seed -> identical
        assert_ne!(a, c); // different seed -> varied
    }

    #[test]
    fn emits_measured_quantities_for_a_contact_primitive() {
        let (s, e) = cable();
        let reports = stochastic_run(&s, &e, &model(), 1).unwrap();
        // The cable skill's force.insert_fit action emits seating_depth.
        let has_seating = reports.iter().any(|r| {
            r.telemetry
                .iter()
                .any(|t| t.measured_quantities.contains_key("seating_depth"))
        });
        assert!(has_seating, "expected a seating_depth measured_quantity");
    }
}
