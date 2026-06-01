// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl certify` orchestration: derive the canonical goals via `retarget` (the unforgeable
//! contract side), ingest the vendor report via `replay`, run the `battery`, and seal a
//! `certificate`. The scope is Class 3 driver-protocol; the certificate discloses what it does
//! not cover.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use rfl_core::canonical::ExecuteGoal;

use crate::battery::{self, ActionVerdict, NamedCheck};
use crate::certificate::{self, ActionEntry, Certificate, CertificateBody, CheckEntry, FileRef};
use crate::{replay, CheckOutcome};

/// The overall certified result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertResult {
    /// Every obligation passed.
    Pass,
    /// At least one obligation failed (a certificate is still emitted).
    Fail,
}

/// The output of a certify run: the sealed certificate and its overall result.
pub struct CertifyOutcome {
    /// The sealed certificate.
    pub certificate: Certificate,
    /// The overall result (mirrors `certificate.body.result`).
    pub result: CertResult,
}

fn check_entry(c: &NamedCheck) -> CheckEntry {
    match &c.outcome {
        CheckOutcome::Pass => CheckEntry { name: c.name, result: "pass", reason: None },
        CheckOutcome::Fail(r) => {
            CheckEntry { name: c.name, result: "fail", reason: Some(r.clone()) }
        }
    }
}

fn action_entry(v: &ActionVerdict) -> ActionEntry {
    ActionEntry {
        action_id: v.action_id.clone(),
        suffix: v.suffix.clone(),
        envelope_class: v.envelope_class.map(certificate::envelope_class_str),
        checks: v.checks.iter().map(check_entry).collect(),
        passed: v.passed,
    }
}

/// Run a certify pass over the three input files.
///
/// # Errors
/// An unparseable skill / embodiment, an invalid report stream, or a correlation mismatch
/// (a missing or orphan action) — each makes the run invalid.
pub fn run(skill_path: &Path, embodiment_path: &Path, report_path: &Path) -> Result<CertifyOutcome> {
    let skill_bytes = std::fs::read(skill_path).with_context(|| format!("read {skill_path:?}"))?;
    let emb_bytes =
        std::fs::read(embodiment_path).with_context(|| format!("read {embodiment_path:?}"))?;
    let report_bytes =
        std::fs::read(report_path).with_context(|| format!("read {report_path:?}"))?;

    let skill = rfl_core::skill_isa::Skill::parse_yaml(
        std::str::from_utf8(&skill_bytes).context("skill is not UTF-8")?,
    )?;
    let emb = rfl_core::embodiment::Embodiment::parse_yaml(
        std::str::from_utf8(&emb_bytes).context("embodiment is not UTF-8")?,
    )?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;

    let reports = replay::replay_report(
        std::str::from_utf8(&report_bytes).context("report is not UTF-8")?,
    )?;

    // Correlate the expected goal set against the replayed reports.
    let mut pairs = Vec::new();
    for (i, (action, suffix)) in out.actions.iter().zip(&out.suffixes).enumerate() {
        let id = format!("{}/{}/{:04}-{}", skill.skill, emb.id, i + 1, suffix);
        let report = reports
            .get(&id)
            .ok_or_else(|| anyhow!("no driver report for expected action {id}"))?
            .clone();
        pairs.push((ExecuteGoal::wrap(id, action.clone()), report));
    }
    let expected: BTreeSet<&str> = pairs.iter().map(|(g, _)| g.action_id.as_str()).collect();
    for k in reports.keys() {
        if !expected.contains(k.as_str()) {
            bail!("driver report contains an unexpected action_id {k}");
        }
    }

    // Run the battery.
    let mut action_entries = Vec::new();
    let mut all_passed = true;
    for (g, r) in &pairs {
        let v = battery::verify_action(g, r);
        if !v.passed {
            all_passed = false;
        }
        action_entries.push(action_entry(&v));
    }
    let seq_entry = check_entry(&battery::verify_sequence(&pairs));
    if seq_entry.result == "fail" {
        all_passed = false;
    }

    let body = CertificateBody {
        certificate_schema_version: "0.1",
        spec_version: rfl_core::SPEC_VERSION,
        tool_version: env!("CARGO_PKG_VERSION"),
        skill: FileRef { id: skill.skill.clone(), sha256: certificate::sha256_hex(&skill_bytes) },
        embodiment: FileRef { id: emb.id.clone(), sha256: certificate::sha256_hex(&emb_bytes) },
        report_sha256: certificate::sha256_hex(&report_bytes),
        result: if all_passed { "pass" } else { "fail" },
        covered: vec!["class3_driver_protocol"],
        excluded: vec![
            "class4_physical",
            "class2_loose_epsilon",
            "env3_disturbance",
            "fidelity_tier_physical_truth",
        ],
        actions: action_entries,
        sequence_checks: vec![seq_entry],
    };
    let result = if all_passed { CertResult::Pass } else { CertResult::Fail };
    Ok(CertifyOutcome { certificate: certificate::seal(body), result })
}
