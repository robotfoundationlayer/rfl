// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! `rfl certify` orchestration: derive the canonical goals via `retarget` (the unforgeable
//! contract side), ingest the vendor report via `replay`, run the `battery`, and seal a
//! `certificate`. The scope is Class 3 driver-protocol; the certificate discloses what it does
//! not cover.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use rfl_core::canonical::ExecuteGoal;

use crate::battery::{self, ActionVerdict, NamedCheck};
use crate::certificate::{self, ActionEntry, Certificate, CertificateBody, CheckEntry, FileRef};
use crate::{CheckOutcome, replay};

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
        CheckOutcome::Pass => CheckEntry {
            name: c.name,
            result: "pass",
            reason: None,
        },
        CheckOutcome::Fail(r) => CheckEntry {
            name: c.name,
            result: "fail",
            reason: Some(r.clone()),
        },
    }
}

fn action_entry(v: &ActionVerdict) -> ActionEntry {
    ActionEntry {
        action_id: v.action_id.clone(),
        suffix: v.suffix.clone(),
        envelope_class: v.envelope_class.map(certificate::envelope_class_str),
        fidelity_tier: v.fidelity_tier.clone(),
        checks: v.checks.iter().map(check_entry).collect(),
        passed: v.passed,
    }
}

/// Run a certify pass over the three input files.
///
/// # Errors
/// An unparseable skill / embodiment, an invalid report stream, or a correlation mismatch
/// (a missing or orphan action) — each makes the run invalid.
pub fn run(
    skill_path: &Path,
    embodiment_path: &Path,
    report_path: &Path,
) -> Result<CertifyOutcome> {
    let skill_bytes = std::fs::read(skill_path).with_context(|| format!("read {skill_path:?}"))?;
    let emb_bytes =
        std::fs::read(embodiment_path).with_context(|| format!("read {embodiment_path:?}"))?;
    let report_bytes =
        std::fs::read(report_path).with_context(|| format!("read {report_path:?}"))?;
    certify_core(
        &skill_bytes,
        &emb_bytes,
        std::str::from_utf8(&report_bytes).context("report is not UTF-8")?,
    )
}

/// The certify pipeline over already-read inputs: parse + retarget + replay the report text +
/// correlate + battery + seal. Shared by `run` (replay) and `run_live` (subprocess).
fn certify_core(skill_bytes: &[u8], emb_bytes: &[u8], report_text: &str) -> Result<CertifyOutcome> {
    let skill = rfl_core::skill_isa::Skill::parse_yaml(
        std::str::from_utf8(skill_bytes).context("skill is not UTF-8")?,
    )?;
    let emb = rfl_core::embodiment::Embodiment::parse_yaml(
        std::str::from_utf8(emb_bytes).context("embodiment is not UTF-8")?,
    )?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;

    let reports = replay::replay_report(report_text)?;

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
        skill: FileRef {
            id: skill.skill.clone(),
            sha256: certificate::sha256_hex(skill_bytes),
        },
        embodiment: FileRef {
            id: emb.id.clone(),
            sha256: certificate::sha256_hex(emb_bytes),
        },
        report_sha256: certificate::sha256_hex(report_text.as_bytes()),
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
    let result = if all_passed {
        CertResult::Pass
    } else {
        CertResult::Fail
    };
    Ok(CertifyOutcome {
        certificate: certificate::seal(body),
        result,
    })
}

/// Spawn `driver`, write the execute goals to its stdin (EOF on completion), and return its
/// stdout. The stdin write runs on its own thread alongside the stdout reader so a driver that
/// fills its stdout pipe while we are still writing stdin does not deadlock; a broken-pipe on
/// stdin is ignored (a driver may emit a full report without consuming all its input). A driver
/// that does not finish within `timeout` is killed.
///
/// # Errors
/// The driver fails to spawn, exits non-zero, or overruns `timeout`.
fn drive_subprocess(driver: &Path, goals: &str, timeout: Duration) -> Result<String> {
    let mut child = Command::new(driver)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("spawn driver {driver:?}: {e}"))?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    let goals_owned = goals.to_string();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(goals_owned.as_bytes()); // EPIPE tolerated; stdin dropped -> EOF
    });

    let mut stdout = child.stdout.take().expect("piped stdout");
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let mut stderr = child.stderr.take().expect("piped stderr");
    let ereader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let start = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| anyhow!("wait driver: {e}"))? {
            break st;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            bail!("driver did not finish within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let _ = writer.join();
    let out = reader
        .join()
        .map_err(|_| anyhow!("driver stdout reader panicked"))?;
    let err = ereader.join().unwrap_or_default();
    if !status.success() {
        let tail = if err.trim().is_empty() {
            String::new()
        } else {
            format!(" (stderr: {})", err.trim())
        };
        bail!("driver exited unsuccessfully ({status}){tail}");
    }
    Ok(out)
}

/// Certify by spawning a driver (live mode): retarget the skill onto the embodiment, write the
/// canonical execute goals to the driver's stdin, read its telemetry+status from stdout, and run
/// the certify pipeline on that report. The driver's stdin is exactly the `rfl retarget` output.
///
/// # Errors
/// The skill / embodiment is unparseable, the driver fails to spawn / exits non-zero / times out,
/// or its stdout is an invalid or uncorrelated report.
pub fn run_live(
    skill_path: &Path,
    embodiment_path: &Path,
    driver_path: &Path,
    timeout: Duration,
) -> Result<CertifyOutcome> {
    let skill_bytes = std::fs::read(skill_path).with_context(|| format!("read {skill_path:?}"))?;
    let emb_bytes =
        std::fs::read(embodiment_path).with_context(|| format!("read {embodiment_path:?}"))?;
    let skill = rfl_core::skill_isa::Skill::parse_yaml(
        std::str::from_utf8(&skill_bytes).context("skill is not UTF-8")?,
    )?;
    let emb = rfl_core::embodiment::Embodiment::parse_yaml(
        std::str::from_utf8(&emb_bytes).context("embodiment is not UTF-8")?,
    )?;
    let out = rfl_core::translation::retarget(&skill, &emb)?;
    let goals = rfl_core::canonical::to_jsonl(&skill.skill, &emb.id, &out.actions, &out.suffixes);
    let report_text = drive_subprocess(driver_path, &goals, timeout)?;
    certify_core(&skill_bytes, &emb_bytes, &report_text)
}
