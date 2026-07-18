# `rfl certify` v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `rfl certify --skill <s> --embodiment <e> --report <driver.jsonl>` — a JSONL-replay third-party self-certification tool that validates a vendor's captured driver report against the Class 3 driver-protocol obligations and emits a deterministic, hashable certificate.

**Architecture:** Four new focused modules in `rfl-conformance` (`replay` ingest → `battery` verify → `certificate` artifact → `certify` orchestrate), plus a thin `rfl certify` CLI arm. Every `check_*` is reused unchanged (each is already a pure function of `(ExecuteGoal, DriverReport)`). The goal/contract side is derived by `retarget` (unforgeable by the vendor); only the report side is ingested from the file. `rfl-core` wire types stay `Serialize`-only — ingestion gets its own strict `Deserialize` mirror structs in the verifier.

**Tech Stack:** Rust (edition 2024), `serde`/`serde_json`, `boon` 0.6 (JSON Schema validation, promoted dev-dep→dep), `sha2` 0.10 (content hashes), `clap` 4, `anyhow`.

**Design doc:** `docs/design/2026-06-01-conformance-certify-v0-design.md` (committed `27d1ee1`).

**Discipline (every task):** inline TDD red→green; commit each task with an explicit `git add <paths>` (never `-A`/`.`); Conventional Commits, English imperative ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` must read `0 0` before AND a ff `git push origin main` after (a parallel session shares this tree — re-verify `git log` has your commit each batch); run full `cargo test` (workspace) + `validate.py` in a batch **separate** from the commit and READ the output.

----

## File Structure

- `crates/rfl-conformance/Cargo.toml` — promote `boon` to `[dependencies]`, add `sha2`.
- `crates/rfl-conformance/src/lib.rs` — add `pub mod replay; pub mod battery; pub mod certificate; pub mod certify;`; change `fn suffix_of` → `pub(crate) fn suffix_of` (visibility only).
- `crates/rfl-conformance/src/replay.rs` — NEW. Ingest vendor JSONL → `BTreeMap<action_id, DriverReport>`: parse, boon schema-validate, strict-deserialize into mirror structs, convert into canonical types, correlate (one status per action).
- `crates/rfl-conformance/src/battery.rs` — NEW. `verify_action` (full per-action check battery) + `verify_sequence` (momentary_release). Pure.
- `crates/rfl-conformance/src/certificate.rs` — NEW. Certificate structs + `sha256` + `seal` (content_hash) + `to_json`. Pure.
- `crates/rfl-conformance/src/certify.rs` — NEW. `run(skill, embodiment, report) -> CertifyOutcome`: retarget → replay → correlate-expected → battery → seal.
- `crates/rfl-conformance/tests/certify.rs` — NEW. Integration tests for the library path (happy/fail/invalid-run/determinism).
- `crates/rfl-cli/Cargo.toml` — add `rfl-conformance` path dep.
- `crates/rfl-cli/src/main.rs` — add `Certify` subcommand + handler with exit-code semantics.
- `crates/rfl-cli/tests/certify_cli.rs` — NEW. CLI smoke test (exit code + summary).

----

## Task 0: Wire dependencies

**Files:**
- Modify: `crates/rfl-conformance/Cargo.toml`
- Modify: `crates/rfl-cli/Cargo.toml`

- [ ] **Step 1: Promote `boon`, add `sha2` in rfl-conformance**

Edit `crates/rfl-conformance/Cargo.toml`. Move `boon = "0.6"` out of `[dev-dependencies]` and add it + `sha2` to `[dependencies]`. Result:

```toml
[dependencies]
rfl-core = { path = "../rfl-core" }
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
boon = "0.6"
sha2 = "0.10"

[dev-dependencies]
insta.workspace = true
```

- [ ] **Step 2: Add rfl-conformance dep to rfl-cli**

Edit `crates/rfl-cli/Cargo.toml` `[dependencies]`, append:

```toml
rfl-conformance = { path = "../rfl-conformance" }
```

- [ ] **Step 3: Verify the workspace still builds**

Run: `cargo build --workspace`
Expected: builds clean (existing tests that used `boon` as a dev-dep still resolve it as a normal dep).

- [ ] **Step 4: Commit**

```bash
git add crates/rfl-conformance/Cargo.toml crates/rfl-cli/Cargo.toml Cargo.lock
git commit -m "build(conformance): add boon+sha2 deps, wire rfl-cli->rfl-conformance

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

(`Cargo.lock` updates with the new `sha2` resolution; stage it explicitly.)

----

## Task 1: `replay` — ingest the vendor JSONL

**Files:**
- Create: `crates/rfl-conformance/src/replay.rs`
- Modify: `crates/rfl-conformance/src/lib.rs` (add `pub mod replay;`)

### Behavior
`replay_report(jsonl: &str) -> Result<BTreeMap<String, DriverReport>>`: per non-blank line, parse JSON → boon-validate against the embedded `driver-interface.schema.json` → strict-deserialize into `TelemetryIn`/`StatusIn` (`deny_unknown_fields`) → convert into canonical `Telemetry`/`Status` → correlate by `action_id` (exactly one status per action; a telemetry line for an action that never gets a status is an error; a second status is an error).

Floored sub-objects: `realized_pose`/`final_pose` are **presence-only** (no check reads their contents — `Option<serde_json::Value>` → `RealizedPose::placeholder()` when present). `wrench`/`verdict` ARE consumed by checks, so they are concretely typed. `tactile` is schema-validated but not carried (no v0 check consumes readings). `safety_flags.momentary_release` is accepted (schema-declared) but the authoritative continuity-break signal in v0 is `verdict.evidence` containing `"momentary_release"` (matches the suite); document this.

- [ ] **Step 1: Add the module declaration**

In `crates/rfl-conformance/src/lib.rs`, after the existing `use` block near the top (before `pub fn retarget_example_to_jsonl`), add:

```rust
pub mod battery;
pub mod certificate;
pub mod certify;
pub mod replay;
```

Also change the existing private helper (around line 851) from:

```rust
fn suffix_of(action_id: &str) -> &str {
```

to:

```rust
pub(crate) fn suffix_of(action_id: &str) -> &str {
```

(visibility only — no behavior change.)

- [ ] **Step 2: Write the failing round-trip test**

Create `crates/rfl-conformance/src/replay.rs` with only the test module first (so it compiles to a failing test once the function exists; write the test now, the impl in Step 4):

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Vendor driver-report ingestion for `rfl certify` (`spec/05` Test Class 3, replay mode).
//!
//! `rfl-core` produces the wire protocol (`Serialize` only, RD1c-deterministic); the verifier
//! *consumes* arbitrary vendor protocol. So the strict `Deserialize` mirror structs +
//! schema-validation live here, in the verifier, not in `rfl-core`. Reconstructs the canonical
//! `DriverReport`s the unchanged `check_*` battery already operates on.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{reports_to_jsonl, run_reference_driver};
    use std::path::Path;

    fn example_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn round_trips_reference_driver_output() {
        let dir = example_dir();
        let reports = run_reference_driver(
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .expect("drive");
        let jsonl = reports_to_jsonl(&reports);
        let replayed = replay_report(&jsonl).expect("replay");
        // 8 cable-insertion actions, each reconstructed with its status + telemetry.
        assert_eq!(replayed.len(), 8);
        for r in &reports {
            let back = replayed.get(&r.status.action_id).expect("action present");
            assert_eq!(back.telemetry.len(), r.telemetry.len());
            assert_eq!(back.status.outcome, r.status.outcome);
            assert_eq!(back.status.action_id, r.status.action_id);
        }
    }
}
```

- [ ] **Step 3: Run it to verify it fails to compile**

Run: `cargo test -p rfl-conformance --lib replay::tests::round_trips_reference_driver_output`
Expected: compile error `cannot find function 'replay_report'`.

- [ ] **Step 4: Implement `replay.rs` (the module body above the test module)**

Insert this above the `#[cfg(test)]` block in `crates/rfl-conformance/src/replay.rs`:

```rust
use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use rfl_core::driver::{
    DriverReport, FreedPartDisposition, Outcome, RealizedPose, SafetyFlags, Status, Telemetry,
    Verdict, Wrench,
};
use rfl_core::quantity::Quantity;

/// The driver-interface wire schema, embedded so the validator is self-contained in the binary
/// (the installed `rfl` has no repo checkout to read `schemas/` from).
const DRIVER_SCHEMA: &str = include_str!("../../../schemas/driver-interface.schema.json");

/// Compile the embedded schema once per run.
fn compile_schema() -> Result<(boon::Schemas, usize)> {
    let value: serde_json::Value =
        serde_json::from_str(DRIVER_SCHEMA).context("parse embedded driver-interface schema")?;
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource("driver-interface.schema.json", value)
        .map_err(|e| anyhow!("add schema resource: {e}"))?;
    let idx = compiler
        .compile("driver-interface.schema.json", &mut schemas)
        .map_err(|e| anyhow!("compile schema: {e}"))?;
    Ok((schemas, idx))
}

// ---- strict input mirrors (deny_unknown_fields == the schema's additionalProperties:false) ----

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeIn {
    Succeeded,
    Failed,
    Indeterminate,
}

/// `wrench` IS consumed by the force-trajectory / contact-band checks, so it is concretely typed.
/// The schema floors it as an open object; certify requires the fields it reads.
#[derive(serde::Deserialize)]
struct WrenchIn {
    force: [f64; 3],
    torque: [f64; 3],
}

/// `verdict.evidence` IS consumed (engagement / irreversible / momentary_release / actuation);
/// `value` / `confidence` are not read by any check but are accepted.
#[derive(serde::Deserialize)]
struct VerdictIn {
    #[serde(default)]
    value: bool,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FreedPartDispositionIn {
    disposition: String,
    #[serde(default)]
    zone: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SafetyFlagsIn {
    #[serde(default)]
    momentary_release: Option<bool>,
    #[serde(default)]
    freed_part_disposition: Option<FreedPartDispositionIn>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TelemetryIn {
    message: String,
    action_id: String,
    t: f64,
    #[serde(default)]
    realized_pose: Option<serde_json::Value>, // presence-only (contents floored + unread)
    #[serde(default)]
    wrench: Option<WrenchIn>,
    #[serde(default)]
    securing_force: Option<String>,
    #[serde(default)]
    station_error: Option<String>,
    #[serde(default)]
    tactile: Vec<serde_json::Value>, // schema-validated upstream; readings not carried in v0
    #[serde(default)]
    events: Vec<serde_json::Value>,
    #[serde(default)]
    fidelity_tier: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusIn {
    message: String,
    action_id: String,
    outcome: OutcomeIn,
    #[serde(default)]
    verdict: Option<VerdictIn>,
    #[serde(default)]
    fidelity_tier: Option<String>,
    #[serde(default)]
    final_pose: Option<serde_json::Value>, // presence-only
    #[serde(default)]
    failure_class: Option<String>,
    #[serde(default)]
    failure_detail: Option<String>,
    #[serde(default)]
    stop_latency: Option<String>,
    #[serde(default)]
    safety_flags: Option<SafetyFlagsIn>,
}

fn to_telemetry(t: TelemetryIn) -> Telemetry {
    Telemetry {
        message: "telemetry",
        action_id: t.action_id,
        t: t.t,
        realized_pose: t.realized_pose.map(|_| RealizedPose::placeholder()),
        wrench: t.wrench.map(|w| Wrench { force: w.force, torque: w.torque }),
        securing_force: t.securing_force.map(Quantity),
        station_error: t.station_error.map(Quantity),
        tactile: vec![],
        events: t.events,
        fidelity_tier: t.fidelity_tier,
    }
}

fn to_status(s: StatusIn) -> Status {
    Status {
        message: "status",
        action_id: s.action_id,
        outcome: match s.outcome {
            OutcomeIn::Succeeded => Outcome::Succeeded,
            OutcomeIn::Failed => Outcome::Failed,
            OutcomeIn::Indeterminate => Outcome::Indeterminate,
        },
        verdict: s.verdict.map(|v| Verdict {
            value: v.value,
            confidence: v.confidence,
            evidence: v.evidence,
        }),
        fidelity_tier: s.fidelity_tier,
        final_pose: s.final_pose.map(|_| RealizedPose::placeholder()),
        failure_class: s.failure_class,
        failure_detail: s.failure_detail,
        stop_latency: s.stop_latency.map(Quantity),
        safety_flags: s.safety_flags.map(|sf| SafetyFlags {
            freed_part_disposition: sf.freed_part_disposition.map(|d| FreedPartDisposition {
                disposition: d.disposition,
                zone: d.zone,
            }),
        }),
    }
}

/// One action's accumulating report during correlation.
#[derive(Default)]
struct Partial {
    telemetry: Vec<Telemetry>,
    status: Option<Status>,
}

/// Ingest a vendor driver-report JSONL stream into per-action `DriverReport`s, keyed by
/// `action_id`. Each line must be a schema-valid `telemetry` or `status` message; every action
/// must have exactly one terminal `status`.
///
/// # Errors
/// A malformed / schema-invalid line, a non-report message kind, a duplicate status, or a
/// telemetry-only action (no status) — each is an invalid run.
pub fn replay_report(jsonl: &str) -> Result<BTreeMap<String, DriverReport>> {
    let (schemas, idx) = compile_schema()?;
    let mut acc: BTreeMap<String, Partial> = BTreeMap::new();

    for (i, line) in jsonl.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).with_context(|| format!("line {n}: invalid JSON"))?;
        schemas
            .validate(&v, idx)
            .map_err(|e| anyhow!("line {n}: schema violation: {e}"))?;
        let msg = v.get("message").and_then(serde_json::Value::as_str).map(str::to_owned);
        match msg.as_deref() {
            Some("telemetry") => {
                let t: TelemetryIn = serde_json::from_value(v)
                    .map_err(|e| anyhow!("line {n}: telemetry deserialize: {e}"))?;
                acc.entry(t.action_id.clone()).or_default().telemetry.push(to_telemetry(t));
            }
            Some("status") => {
                let s: StatusIn = serde_json::from_value(v)
                    .map_err(|e| anyhow!("line {n}: status deserialize: {e}"))?;
                let slot = acc.entry(s.action_id.clone()).or_default();
                if slot.status.is_some() {
                    bail!("line {n}: duplicate status for action {}", s.action_id);
                }
                slot.status = Some(to_status(s));
            }
            other => bail!("line {n}: report stream contains a non-report message {other:?}"),
        }
    }

    let mut out = BTreeMap::new();
    for (id, p) in acc {
        let status = p
            .status
            .ok_or_else(|| anyhow!("action {id} has telemetry but no terminal status"))?;
        out.insert(id, DriverReport { telemetry: p.telemetry, status });
    }
    Ok(out)
}
```

- [ ] **Step 5: Run the round-trip test to verify it passes**

Run: `cargo test -p rfl-conformance --lib replay::tests::round_trips_reference_driver_output`
Expected: PASS.

- [ ] **Step 6: Add the schema-rejection + correlation-error tests**

Append inside the `tests` module in `replay.rs`:

```rust
    #[test]
    fn rejects_unknown_field() {
        // an extra key the schema forbids (additionalProperties:false)
        let bad = r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded","bogus":1}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_bad_outcome_enum() {
        let bad = r#"{"message":"status","action_id":"a/b/0001-x","outcome":"maybe"}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_non_report_message() {
        // execute is an RFL->driver message, never present in a report stream
        let bad = r#"{"message":"execute","action_id":"a/b/0001-x","canonical_action":{}}"#;
        assert!(replay_report(bad).is_err());
    }

    #[test]
    fn rejects_telemetry_without_status() {
        let bad = r#"{"message":"telemetry","action_id":"a/b/0001-x","t":1.0}"#;
        let err = replay_report(bad).unwrap_err().to_string();
        assert!(err.contains("no terminal status"), "got: {err}");
    }

    #[test]
    fn rejects_duplicate_status() {
        let bad = concat!(
            r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded"}"#,
            "\n",
            r#"{"message":"status","action_id":"a/b/0001-x","outcome":"succeeded"}"#,
        );
        let err = replay_report(bad).unwrap_err().to_string();
        assert!(err.contains("duplicate status"), "got: {err}");
    }
```

- [ ] **Step 7: Run the whole replay test module**

Run: `cargo test -p rfl-conformance --lib replay::`
Expected: all PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/rfl-conformance/src/replay.rs crates/rfl-conformance/src/lib.rs
git commit -m "feat(certify): ingest vendor driver-report JSONL (replay)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 2: `battery` — run every obligation, structured verdict

**Files:**
- Create: `crates/rfl-conformance/src/battery.rs`
- (module already declared in lib.rs from Task 1)

### Behavior
`verify_action(goal, report) -> ActionVerdict` runs the full applicable per-action battery (each checker is vacuous-pass when its contract is absent): envelope (if the suffix maps to a class), actuation, engagement, irreversible, freed_part_disposition, audit_honesty. `verify_sequence(pairs) -> NamedCheck` runs `check_momentary_release`. ENV3 (`check_graceful_degradation` / `check_settling`) is **deliberately excluded** — a nominal replay can't inject disturbances (matches the `env3` disclaimer).

- [ ] **Step 1: Write the failing tests**

Create `crates/rfl-conformance/src/battery.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The per-action and per-sequence verification battery for `rfl certify`. Runs every applicable
//! Class 3 obligation (each unchanged `check_*` is vacuous-pass when its contract is absent) and
//! returns a structured verdict. The ENV3 disturbance checks (`check_graceful_degradation` /
//! `check_settling`) are excluded: a nominal vendor replay cannot inject perturbations (the
//! certificate discloses `env3` as not-covered).

use rfl_core::canonical::ExecuteGoal;
use rfl_core::driver::DriverReport;

use crate::{
    check_actuation, check_audit_honesty, check_engagement, check_envelope,
    check_freed_part_disposition, check_irreversible, check_momentary_release, envelope_class_for,
    suffix_of, CheckOutcome, EnvelopeClass,
};

/// A named check outcome.
pub struct NamedCheck {
    /// The obligation name (stable, used in the certificate).
    pub name: &'static str,
    /// Its outcome.
    pub outcome: CheckOutcome,
}

/// The full verdict for one action.
pub struct ActionVerdict {
    /// The correlated action id.
    pub action_id: String,
    /// The primitive suffix.
    pub suffix: String,
    /// The envelope class verified (None for perception primitives with no motion envelope).
    pub envelope_class: Option<EnvelopeClass>,
    /// Every obligation run for this action.
    pub checks: Vec<NamedCheck>,
    /// True iff every check passed.
    pub passed: bool,
}

/// Run the full per-action obligation battery.
#[must_use]
pub fn verify_action(goal: &ExecuteGoal, report: &DriverReport) -> ActionVerdict {
    let suffix = suffix_of(&goal.action_id).to_string();
    let envelope_class = envelope_class_for(&suffix);
    let mut checks = Vec::new();
    if let Some(class) = envelope_class {
        checks.push(NamedCheck { name: "envelope", outcome: check_envelope(class, goal, report) });
    }
    checks.push(NamedCheck { name: "actuation", outcome: check_actuation(goal, report) });
    checks.push(NamedCheck { name: "engagement", outcome: check_engagement(goal, report) });
    checks.push(NamedCheck { name: "irreversible", outcome: check_irreversible(goal, report) });
    checks.push(NamedCheck {
        name: "freed_part_disposition",
        outcome: check_freed_part_disposition(goal, report),
    });
    checks.push(NamedCheck { name: "audit_honesty", outcome: check_audit_honesty(goal, report) });
    let passed = checks.iter().all(|c| matches!(c.outcome, CheckOutcome::Pass));
    ActionVerdict { action_id: goal.action_id.clone(), suffix, envelope_class, checks, passed }
}

/// Run the sequence-level obligation (`momentary_release` propagation, AUD2).
#[must_use]
pub fn verify_sequence(pairs: &[(ExecuteGoal, DriverReport)]) -> NamedCheck {
    NamedCheck { name: "momentary_release", outcome: check_momentary_release(pairs) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{drive, FaultyDriver, Fault, ReferenceDriver};
    use std::path::Path;

    fn example_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
    }

    #[test]
    fn nominal_action_passes_full_battery() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // index 1 = grasp.pinch (grasp-continuity).
        let (g, r) = &pairs[1];
        let v = verify_action(g, r);
        assert_eq!(v.suffix, "pinch");
        assert_eq!(v.envelope_class, Some(EnvelopeClass::GraspContinuity));
        assert!(v.passed, "checks: {:?}", v.checks.iter().map(|c| (c.name, &c.outcome)).collect::<Vec<_>>());
    }

    #[test]
    fn under_secure_action_fails_envelope() {
        let dir = example_dir();
        let pairs = drive(
            FaultyDriver::new(Fault::UnderSecure),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        let (g, r) = &pairs[1]; // grasp.pinch
        let v = verify_action(g, r);
        assert!(!v.passed);
        let env = v.checks.iter().find(|c| c.name == "envelope").unwrap();
        assert!(matches!(env.outcome, CheckOutcome::Fail(_)));
    }

    #[test]
    fn sequence_check_runs_momentary_release() {
        let dir = example_dir();
        let pairs = drive(
            ReferenceDriver::default(),
            &dir.join("skill.yaml"),
            &dir.join("embodiments/allegro.yaml"),
        )
        .unwrap();
        // cable-insertion has no flip -> vacuously Pass.
        let seq = verify_sequence(&pairs);
        assert_eq!(seq.name, "momentary_release");
        assert!(matches!(seq.outcome, CheckOutcome::Pass));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test -p rfl-conformance --lib battery::`
Expected: compile error — `suffix_of` not yet `pub(crate)` would already be fixed in Task 1; if anything is missing, fix imports. The functions exist, so this should compile and pass once the file is saved. If `Fault` / `FaultyDriver` / `drive` / `ReferenceDriver` are not re-exported, they are already `pub` in lib.rs — import via `crate::`.

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cargo test -p rfl-conformance --lib battery::`
Expected: all PASS. (This task is implementation-and-test-in-one because `battery` is a pure composition of already-tested `check_*`; the tests are the verification.)

- [ ] **Step 4: Commit**

```bash
git add crates/rfl-conformance/src/battery.rs
git commit -m "feat(certify): per-action + sequence verification battery

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 3: `certificate` — the signable artifact

**Files:**
- Create: `crates/rfl-conformance/src/certificate.rs`

### Behavior
Deterministic canonical JSON. `seal(body)` computes `content_hash = "sha256:" + sha256(compact serialization of the body)` and returns the full `Certificate`. `to_json` pretty-prints the sealed certificate. No floats in the cert → byte-reproducible. Re-verification rule: recompute sha256 over the compact serialization of every field except `content_hash`.

- [ ] **Step 1: Write the failing determinism test**

Create `crates/rfl-conformance/src/certificate.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! The `rfl certify` certificate: a deterministic, content-hashed artifact. Inputs are identified
//! by RFL id + content sha256 (never a filesystem path), so the certificate is machine-independent.
//! `content_hash` covers the compact serialization of every field except itself; signing is
//! out-of-band (detached-sign the canonical bytes).

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::EnvelopeClass;

/// An input file reference: its RFL id + content hash.
#[derive(Serialize)]
pub struct FileRef {
    /// The RFL id (skill name / embodiment id).
    pub id: String,
    /// Lowercase hex sha256 of the file bytes.
    pub sha256: String,
}

/// One obligation's result in the certificate.
#[derive(Serialize)]
pub struct CheckEntry {
    /// The obligation name.
    pub name: &'static str,
    /// `"pass"` or `"fail"`.
    pub result: &'static str,
    /// The failure reason (present only on `fail`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One action's certified result.
#[derive(Serialize)]
pub struct ActionEntry {
    /// The correlated action id.
    pub action_id: String,
    /// The primitive suffix.
    pub suffix: String,
    /// The envelope class verified (absent for perception primitives).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope_class: Option<&'static str>,
    /// Every obligation run for this action.
    pub checks: Vec<CheckEntry>,
    /// True iff every check passed.
    pub passed: bool,
}

/// The certificate body (everything the `content_hash` covers).
#[derive(Serialize)]
pub struct CertificateBody {
    /// The certificate-format version.
    pub certificate_schema_version: &'static str,
    /// The RFL spec version the suite implements.
    pub spec_version: &'static str,
    /// The `rfl` tool version.
    pub tool_version: &'static str,
    /// The certified skill.
    pub skill: FileRef,
    /// The certified embodiment.
    pub embodiment: FileRef,
    /// sha256 of the vendor report bytes.
    pub report_sha256: String,
    /// `"pass"` or `"fail"`.
    pub result: &'static str,
    /// The test classes / dimensions this certificate covers.
    pub covered: Vec<&'static str>,
    /// What it explicitly does NOT cover (honesty boundary).
    pub excluded: Vec<&'static str>,
    /// Per-action results, in retarget order.
    pub actions: Vec<ActionEntry>,
    /// Sequence-level obligations (e.g. momentary_release propagation).
    pub sequence_checks: Vec<CheckEntry>,
}

/// A sealed certificate: the body plus its content hash.
#[derive(Serialize)]
pub struct Certificate {
    /// The hashed body.
    #[serde(flatten)]
    pub body: CertificateBody,
    /// `"sha256:<hex>"` over the compact serialization of `body`.
    pub content_hash: String,
}

/// Lowercase hex sha256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The stable string for an envelope class (certificate vocabulary).
#[must_use]
pub fn envelope_class_str(class: EnvelopeClass) -> &'static str {
    match class {
        EnvelopeClass::TerminalPostcondition => "terminal_postcondition",
        EnvelopeClass::GraspContinuity => "grasp_continuity",
        EnvelopeClass::ForceTrajectory => "force_trajectory",
        EnvelopeClass::IntervalInvariant => "interval_invariant",
    }
}

/// Seal a body: attach `content_hash` over its compact serialization.
#[must_use]
pub fn seal(body: CertificateBody) -> Certificate {
    let compact = serde_json::to_vec(&body).expect("serialize certificate body");
    let content_hash = format!("sha256:{}", sha256_hex(&compact));
    Certificate { body, content_hash }
}

/// Render a sealed certificate as pretty canonical JSON.
#[must_use]
pub fn to_json(cert: &Certificate) -> String {
    serde_json::to_string_pretty(cert).expect("serialize certificate")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_body(result: &'static str) -> CertificateBody {
        CertificateBody {
            certificate_schema_version: "0.1",
            spec_version: "v0.1-draft",
            tool_version: "0.0.1",
            skill: FileRef { id: "cable-insertion".into(), sha256: "aa".into() },
            embodiment: FileRef { id: "allegro".into(), sha256: "bb".into() },
            report_sha256: "cc".into(),
            result,
            covered: vec!["class3_driver_protocol"],
            excluded: vec!["class4_physical", "class2_loose_epsilon", "env3_disturbance"],
            actions: vec![ActionEntry {
                action_id: "cable-insertion/allegro/0002-pinch".into(),
                suffix: "pinch".into(),
                envelope_class: Some("grasp_continuity"),
                checks: vec![CheckEntry { name: "envelope", result: "pass", reason: None }],
                passed: true,
            }],
            sequence_checks: vec![CheckEntry {
                name: "momentary_release",
                result: "pass",
                reason: None,
            }],
        }
    }

    #[test]
    fn seal_is_deterministic_and_verifiable() {
        let a = to_json(&seal(sample_body("pass")));
        let b = to_json(&seal(sample_body("pass")));
        assert_eq!(a, b, "certificate must be byte-identical across runs");

        // recompute the hash over the compact body and confirm it matches content_hash.
        let cert = seal(sample_body("pass"));
        let compact = serde_json::to_vec(&cert.body).unwrap();
        assert_eq!(cert.content_hash, format!("sha256:{}", sha256_hex(&compact)));
        assert!(cert.content_hash.starts_with("sha256:"));
    }

    #[test]
    fn result_change_changes_hash() {
        let pass = seal(sample_body("pass")).content_hash;
        let fail = seal(sample_body("fail")).content_hash;
        assert_ne!(pass, fail);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail then build**

Run: `cargo test -p rfl-conformance --lib certificate::`
Expected: compiles and PASSES (the module is self-contained). If `seal_is_deterministic_and_verifiable` fails on the byte-equality assertion, it would reveal `#[serde(flatten)]` reordering — not expected, but if it occurs, replace `#[serde(flatten)]` with an explicit re-serialization (serialize body fields then append `content_hash`) and re-run.

- [ ] **Step 3: Commit**

```bash
git add crates/rfl-conformance/src/certificate.rs
git commit -m "feat(certify): deterministic content-hashed certificate artifact

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 4: `certify` — orchestration

**Files:**
- Create: `crates/rfl-conformance/src/certify.rs`
- Create: `crates/rfl-conformance/tests/certify.rs`

### Behavior
`run(skill_path, embodiment_path, report_path) -> Result<CertifyOutcome>`: read + hash each file; parse skill/embodiment; `retarget` → expected goals; `replay_report`; correlate (every expected action has a report, no orphan reports); per-pair `verify_action`; `verify_sequence`; assemble + `seal`. An `Err` is an **invalid run** (the CLI maps it to exit 2); a sealed `result: "fail"` is a valid non-conformant run (exit 1).

- [ ] **Step 1: Implement `certify.rs`**

Create `crates/rfl-conformance/src/certify.rs`:

```rust
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
use crate::certificate::{
    self, ActionEntry, Certificate, CertificateBody, CheckEntry, FileRef,
};
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
    let seq = battery::verify_sequence(&pairs);
    let seq_entry = check_entry(&seq);
    if seq_entry.result == "fail" {
        all_passed = false;
    }

    let result = if all_passed { CertResult::Pass } else { CertResult::Fail };
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
    Ok(CertifyOutcome { certificate: certificate::seal(body), result })
}
```

- [ ] **Step 2: Write the failing integration tests**

Create `crates/rfl-conformance/tests/certify.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! End-to-end `certify::run` tests: a nominal report certifies pass; a faulty report certifies
//! fail; broken correlation is an invalid run; the certificate is deterministic.

use std::path::{Path, PathBuf};

use rfl_conformance::certify::{self, CertResult};
use rfl_conformance::{drive, reports_to_jsonl, Fault, FaultyDriver, ReferenceDriver};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// Write `jsonl` to a unique temp file and return its path.
fn temp_report(tag: &str, jsonl: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rfl-certify-{}-{}.jsonl", std::process::id(), tag));
    std::fs::write(&p, jsonl).unwrap();
    p
}

#[test]
fn nominal_report_certifies_pass() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("nominal", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Pass);
    let body = &outcome.certificate.body;
    assert_eq!(body.result, "pass");
    assert_eq!(body.actions.len(), 8);
    assert_eq!(body.covered, vec!["class3_driver_protocol"]);
    assert!(body.excluded.contains(&"env3_disturbance"));
    assert!(body.excluded.contains(&"class4_physical"));
    std::fs::remove_file(report).ok();
}

#[test]
fn faulty_report_certifies_fail_with_reason() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    // generate a non-conformant report (UnderSecure lowers grasp.pinch securing_force).
    let pairs = drive(FaultyDriver::new(Fault::UnderSecure), &skill, &emb).unwrap();
    let reports: Vec<_> = pairs.into_iter().map(|(_, r)| r).collect();
    let report = temp_report("faulty", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Fail);
    let pinch = outcome
        .certificate
        .body
        .actions
        .iter()
        .find(|a| a.suffix == "pinch")
        .unwrap();
    assert!(!pinch.passed);
    let env = pinch.checks.iter().find(|c| c.name == "envelope").unwrap();
    assert_eq!(env.result, "fail");
    assert!(env.reason.as_ref().unwrap().contains("securing_force"));
    std::fs::remove_file(report).ok();
}

#[test]
fn missing_status_is_invalid_run() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    // drop all status lines for the last action -> correlation gap.
    let full = reports_to_jsonl(&reports);
    let last_id = &reports.last().unwrap().status.action_id;
    let pruned: String = full
        .lines()
        .filter(|l| !(l.contains("\"status\"") && l.contains(last_id.as_str())))
        .collect::<Vec<_>>()
        .join("\n");
    let report = temp_report("missing", &pruned);

    let err = certify::run(&skill, &emb, &report).unwrap_err().to_string();
    assert!(err.contains("no driver report") || err.contains("no terminal status"), "got: {err}");
    std::fs::remove_file(report).ok();
}

#[test]
fn certificate_is_deterministic() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("determinism", &reports_to_jsonl(&reports));

    let a = certify::run(&skill, &emb, &report).unwrap().certificate.content_hash;
    let b = certify::run(&skill, &emb, &report).unwrap().certificate.content_hash;
    assert_eq!(a, b);
    std::fs::remove_file(report).ok();
}
```

This test file uses `rfl_conformance::{drive, reports_to_jsonl, Fault, FaultyDriver, ReferenceDriver, run_reference_driver}` (all already `pub` in lib.rs) and `rfl_conformance::certify`.

- [ ] **Step 3: Run the certify tests to verify they pass**

Run: `cargo test -p rfl-conformance --test certify`
Expected: all 4 PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/rfl-conformance/src/certify.rs crates/rfl-conformance/tests/certify.rs
git commit -m "feat(certify): orchestrate retarget+replay+battery into a sealed certificate

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 5: `rfl certify` CLI subcommand

**Files:**
- Modify: `crates/rfl-cli/src/main.rs`
- Create: `crates/rfl-cli/tests/certify_cli.rs`

### Behavior
`rfl certify --skill <s> --embodiment <e> --report <j> [--out <path>]`. Prints a human PASS/FAIL summary + per-action table to stdout; `--out` writes the canonical certificate JSON. Exit codes: `0` valid+pass, `1` valid+fail (cert still written/printed), `2` invalid run (error to stderr, no certificate).

- [ ] **Step 1: Add the subcommand variant**

In `crates/rfl-cli/src/main.rs`, extend the doc comment block and the `Command` enum. After the `Conformance { … }` variant, add:

```rust
    /// Certify a vendor driver report (JSONL replay) against the Class 3 driver-protocol
    /// obligations and emit a deterministic certificate.
    Certify {
        /// Path to the skill YAML file.
        #[arg(long)]
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
        /// Path to the captured driver-report JSONL (telemetry + status lines).
        #[arg(long)]
        report: std::path::PathBuf,
        /// Optional path to write the canonical certificate JSON.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
```

- [ ] **Step 2: Add the handler arm**

In `fn main`, inside the `match cli.command` block, add this arm (it ends in `process::exit`, type `!`, so it satisfies the `Result<()>` arms):

```rust
        Command::Certify { skill, embodiment, report, out } => {
            let outcome = match rfl_conformance::certify::run(&skill, &embodiment, &report) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("certify: invalid run: {e:#}");
                    std::process::exit(2);
                }
            };
            let cert = &outcome.certificate;
            let body = &cert.body;
            println!(
                "RFL conformance certificate — {} on {} (spec {})",
                body.skill.id, body.embodiment.id, body.spec_version
            );
            for a in &body.actions {
                let mark = if a.passed { "PASS" } else { "FAIL" };
                let class = a.envelope_class.unwrap_or("perception");
                println!("  [{mark}] {} ({class})", a.action_id);
                for c in &a.checks {
                    if let Some(reason) = &c.reason {
                        println!("        - {} FAIL: {reason}", c.name);
                    }
                }
            }
            for c in &body.sequence_checks {
                let mark = if c.result == "pass" { "PASS" } else { "FAIL" };
                println!("  [{mark}] sequence:{}", c.name);
                if let Some(reason) = &c.reason {
                    println!("        - {reason}");
                }
            }
            println!(
                "RESULT: {} ({} covered, env3/class4 excluded) — {}",
                body.result.to_uppercase(),
                body.covered.join("+"),
                cert.content_hash
            );
            if let Some(path) = out {
                if let Err(e) = std::fs::write(&path, rfl_conformance::certificate::to_json(cert)) {
                    eprintln!("certify: write {path:?}: {e}");
                    std::process::exit(2);
                }
                println!("certificate -> {}", path.display());
            }
            match outcome.result {
                rfl_conformance::certify::CertResult::Pass => std::process::exit(0),
                rfl_conformance::certify::CertResult::Fail => std::process::exit(1),
            }
        }
```

Note: `body.result` is `&'static str` (`"pass"`/`"fail"`); `.to_uppercase()` works on `str`.

- [ ] **Step 3: Build to verify it compiles**

Run: `cargo build -p rfl-cli`
Expected: builds clean.

- [ ] **Step 4: Write the CLI smoke test**

Create `crates/rfl-cli/tests/certify_cli.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Smoke test for `rfl certify`: exit code + summary on the committed cable-insertion example.

use std::path::{Path, PathBuf};
use std::process::Command;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

#[test]
fn certify_nominal_run_exits_zero() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    // generate a nominal JSONL report with the reference driver.
    let reports = rfl_conformance::run_reference_driver(&skill, &emb).unwrap();
    let jsonl = rfl_conformance::reports_to_jsonl(&reports);
    let report = std::env::temp_dir().join(format!("rfl-cli-certify-{}.jsonl", std::process::id()));
    std::fs::write(&report, jsonl).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["certify", "--skill"])
        .arg(&skill)
        .arg("--embodiment")
        .arg(&emb)
        .arg("--report")
        .arg(&report)
        .output()
        .expect("run rfl certify");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "exit {:?}, stdout: {stdout}", out.status.code());
    assert!(stdout.contains("RESULT: PASS"), "stdout: {stdout}");
    std::fs::remove_file(report).ok();
}
```

This requires `rfl-cli` to depend on `rfl-conformance` (added in Task 0) so the test can generate the fixture. `CARGO_BIN_EXE_rfl` is provided by Cargo to integration tests.

- [ ] **Step 5: Run the CLI test**

Run: `cargo test -p rfl-cli --test certify_cli`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-cli/src/main.rs crates/rfl-cli/tests/certify_cli.rs
git commit -m "feat(cli): add rfl certify subcommand with exit-code semantics

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 6: Full-suite verification + README note

**Files:**
- Modify: `README.md` (only if a parallel session is NOT mid-edit; otherwise SKIP and note it)

- [ ] **Step 1: Run the full workspace test suite (READ, separate from any commit)**

Run: `cargo test --workspace`
Expected: all green, including the prior 223 + the new replay/battery/certificate/certify/CLI tests. READ the count.

- [ ] **Step 2: Run validate.py (READ)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py`
Expected: C1–C7 all `ok`.

- [ ] **Step 3: Manual end-to-end smoke (READ)**

```bash
cd ~/Documents/GitHub/rfl
cargo run -q -p rfl-cli -- certify \
  --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml \
  --report /tmp/does-not-exist.jsonl ; echo "exit=$?"
```
Expected: `certify: invalid run: read ...` on stderr, `exit=2` (the invalid-run path).

- [ ] **Step 4 (optional, only if README.md is clean): add a `certify` usage note**

If `git status --short README.md` shows no parallel modification, add a short `rfl certify` usage block to the CLI/usage section of `README.md`. If README is dirty from the parallel session, SKIP this step and note it in the milestone update — do not stage a file you did not author.

- [ ] **Step 5: Commit (only if Step 4 ran)**

```bash
git add README.md
git commit -m "docs(readme): note rfl certify (Class 3 self-certification)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Self-Review

**Spec coverage** (design doc → tasks):
- CLI `rfl certify` + exit codes → Task 5. ✓
- `replay` (parse + boon schema-validate + strict deserialize + correlate) → Task 1. ✓
- `battery` (full per-action + sequence, ENV3 excluded) → Task 2. ✓
- `certificate` (canonical JSON, sha256, content_hash, id+hash identity) → Task 3. ✓
- `certify` orchestration (retarget → replay → correlate-expected → battery → seal) → Task 4. ✓
- Deps (boon promote, sha2, rfl-cli→rfl-conformance) → Task 0. ✓
- Honest boundaries (`covered`/`excluded`) → Task 3 + Task 4 body. ✓
- "ran-and-failed vs couldn't-run" (exit 1 vs 2) → Task 4 (Err vs Fail) + Task 5 (exit map). ✓
- Testing matrix (round-trip, schema rejection, happy/fail/invalid-run, determinism, golden-ish) → Tasks 1–5. ✓ (insta golden deferred — the determinism + field assertions cover it; a true insta snapshot can be added later without new surface.)

**Placeholder scan:** no TBD/TODO; every code step shows complete code; every command has an expected result. ✓

**Type consistency:** `replay_report -> BTreeMap<String, DriverReport>`; `verify_action -> ActionVerdict`; `verify_sequence -> NamedCheck`; `certify::run -> Result<CertifyOutcome>`; `CertifyOutcome { certificate: Certificate, result: CertResult }`; `Certificate { body: CertificateBody, content_hash }`; CLI reads `cert.body.actions[].{passed,checks,envelope_class}` and `cert.body.sequence_checks` — all consistent across tasks. ✓

**Known v0 conventions (documented, intentional):**
- `realized_pose`/`final_pose` ingested presence-only (no check reads contents).
- `tactile` schema-validated but not carried (no check consumes readings).
- Continuity-break signal is `verdict.evidence == "momentary_release"` (matches the suite), not `safety_flags.momentary_release` (schema-permitted, accepted, but not authoritative in v0).
- insta golden of the full certificate deferred (covered by determinism + structural assertions).
