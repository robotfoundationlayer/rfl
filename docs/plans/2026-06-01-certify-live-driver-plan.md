# Certify Live `--driver` Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rfl certify --driver ./my_driver` — spawn a driver, write canonical execute goals to its stdin, read telemetry+status from its stdout, and run the existing certify pipeline live.

**Architecture:** Extract a behavior-preserving `certify_core(skill_bytes, emb_bytes, report_text)` shared by `run` (replay) and a new `run_live`. `run_live` retargets → `to_jsonl` goals → `drive_subprocess` (concurrent stdin-write/stdout-read threads + poll/kill timeout, EPIPE-tolerant) → `certify_core`. CLI gains `--driver`/`--timeout` (XOR `--report`); the redundant `conformance` stub is removed.

**Tech Stack:** Rust (edition 2024), `std::process`/`std::thread`/`std::time` (no new deps), `clap` 4; `sh` for `#[cfg(unix)]` mock drivers (CI is ubuntu+macos).

**Design doc:** `docs/design/2026-06-01-certify-live-driver-design.md` (committed `de85773`).

**Discipline:** explicit `git add <paths>` (never `-A`); Conventional Commits ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after; full `cargo test --workspace` + `validate.py` read separate from each commit.

----

## File Structure

- `crates/rfl-conformance/src/certify.rs` — MODIFY. Extract `certify_core`; rewrite `run` to call it; add `drive_subprocess` + `run_live`.
- `crates/rfl-conformance/tests/certify_live.rs` — NEW (`#[cfg(unix)]`). Live-mode integration tests via shell-script mock drivers.
- `crates/rfl-cli/src/main.rs` — MODIFY. `certify --driver`/`--timeout` (XOR `--report`); remove the `Conformance` variant + arm + doc line.
- `crates/rfl-cli/tests/certify_driver_cli.rs` — NEW (`#[cfg(unix)]`). CLI smoke for `certify --driver`.
- `docs/certifying-a-driver.md` — MODIFY. Add a "Live mode" subsection.

----

## Task 1: Extract `certify_core` (behavior-preserving)

**Files:**
- Modify: `crates/rfl-conformance/src/certify.rs`

- [ ] **Step 1: Add `certify_core` and rewrite `run` to call it**

In `crates/rfl-conformance/src/certify.rs`, replace the entire `pub fn run(...)` body (currently lines ~61-131, ending `Ok(CertifyOutcome { certificate: certificate::seal(body), result })`) with a thin `run` plus the extracted core. Replace:

```rust
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
```

... through the end of `run` ...

```rust
    let result = if all_passed { CertResult::Pass } else { CertResult::Fail };
    Ok(CertifyOutcome { certificate: certificate::seal(body), result })
}
```

with:

```rust
pub fn run(skill_path: &Path, embodiment_path: &Path, report_path: &Path) -> Result<CertifyOutcome> {
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
        skill: FileRef { id: skill.skill.clone(), sha256: certificate::sha256_hex(skill_bytes) },
        embodiment: FileRef { id: emb.id.clone(), sha256: certificate::sha256_hex(emb_bytes) },
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
    let result = if all_passed { CertResult::Pass } else { CertResult::Fail };
    Ok(CertifyOutcome { certificate: certificate::seal(body), result })
}
```

Note `report_sha256: certificate::sha256_hex(report_text.as_bytes())` equals the old `sha256_hex(&report_bytes)` because `report_text = from_utf8(&report_bytes)` does not change the bytes — so committed certs / fixtures are unaffected.

- [ ] **Step 2: Run the existing certify tests (must stay green — pure refactor)**

Run: `cargo test -p rfl-conformance --test certify`
Expected: all 7 PASS.

- [ ] **Step 3: Run the fixture guard (committed certs unchanged)**

Run: `cargo test -p rfl-conformance --test certificate_fixtures`
Expected: PASS (3 cases current; `report_sha256` and `content_hash` unchanged).

- [ ] **Step 4: Commit**

```bash
git add crates/rfl-conformance/src/certify.rs
git commit -m "refactor(certify): extract certify_core shared by run

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 2: `drive_subprocess` + `run_live`

**Files:**
- Modify: `crates/rfl-conformance/src/certify.rs`
- Create: `crates/rfl-conformance/tests/certify_live.rs`

- [ ] **Step 1: Add imports**

At the top of `crates/rfl-conformance/src/certify.rs`, after the existing `use std::collections::BTreeSet;` / `use std::path::Path;` lines, add:

```rust
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
```

- [ ] **Step 2: Add `drive_subprocess` and `run_live`**

Append to `crates/rfl-conformance/src/certify.rs` (after `certify_core`):

```rust
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
    let out = reader.join().map_err(|_| anyhow!("driver stdout reader panicked"))?;
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
```

- [ ] **Step 3: Write the live integration tests**

Create `crates/rfl-conformance/tests/certify_live.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors
#![cfg(unix)]

//! Live-mode (`certify::run_live`) tests via shell-script mock drivers. Unix-only; CI is
//! ubuntu + macos.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rfl_conformance::certify::{self, CertResult};

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

/// Write an executable `/bin/sh` mock driver that drains stdin (the execute goals) and then runs
/// `body`. Returns its path.
fn mock_driver(tag: &str, body: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rfl-mock-driver-{}-{}.sh", std::process::id(), tag));
    std::fs::write(&p, format!("#!/bin/sh\ncat >/dev/null\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

#[test]
fn live_driver_emitting_committed_report_certifies_pass() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let report = dir.join("driver-report.jsonl");
    let mock = mock_driver("ok", &format!("cat '{}'", report.display()));

    let outcome =
        certify::run_live(&skill, &emb, &mock, Duration::from_secs(30)).expect("valid live run");
    assert_eq!(outcome.result, CertResult::Pass);
    // live == replay for the same report bytes: the content_hash matches the committed cert.
    let committed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("certificate.json")).unwrap())
            .unwrap();
    assert_eq!(
        committed.get("content_hash").and_then(serde_json::Value::as_str),
        Some(outcome.certificate.content_hash.as_str())
    );
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_nonzero_exit_is_invalid_run() {
    let dir = example_dir();
    let mock = mock_driver("fail", "exit 3");
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(30),
    );
    assert!(r.is_err());
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_malformed_stdout_is_invalid_run() {
    let dir = example_dir();
    let mock = mock_driver("garbage", "echo 'not a json line'");
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(30),
    );
    assert!(r.is_err());
    std::fs::remove_file(mock).ok();
}

#[test]
fn live_driver_timeout_is_invalid_run_and_returns_promptly() {
    let dir = example_dir();
    let mock = mock_driver("slow", "sleep 30");
    let start = std::time::Instant::now();
    let r = certify::run_live(
        &dir.join("skill.yaml"),
        &dir.join("embodiments/allegro.yaml"),
        &mock,
        Duration::from_secs(1),
    );
    assert!(r.is_err());
    assert!(start.elapsed() < Duration::from_secs(10), "driver should have been killed promptly");
    std::fs::remove_file(mock).ok();
}
```

- [ ] **Step 4: Run the live tests**

Run: `cargo test -p rfl-conformance --test certify_live`
Expected: all 4 PASS (the timeout test finishes in ~1 s).

- [ ] **Step 5: Clippy on the crate**

Run: `cargo clippy -p rfl-conformance --lib --all-features 2>&1 | rg -c "certify\.rs" || echo "0 certify.rs warnings"`
Expected: `0 certify.rs warnings`.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/src/certify.rs crates/rfl-conformance/tests/certify_live.rs
git commit -m "feat(certify): live --driver mode (spawn, exchange over stdio)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 3: CLI `certify --driver` + remove the `conformance` stub

**Files:**
- Modify: `crates/rfl-cli/src/main.rs`
- Create: `crates/rfl-cli/tests/certify_driver_cli.rs`

- [ ] **Step 1: Update the `Certify` variant (report optional, add driver + timeout)**

In `crates/rfl-cli/src/main.rs`, replace the `Certify { … }` variant's `report` arg and add `driver` + `timeout`. The variant becomes:

```rust
    /// Certify a vendor driver report (replay via --report, or live via --driver) against the
    /// Class 3 driver-protocol obligations and emit a deterministic certificate.
    Certify {
        /// Path to the skill YAML file.
        #[arg(long)]
        skill: std::path::PathBuf,
        /// Path to the embodiment descriptor YAML file.
        #[arg(long)]
        embodiment: std::path::PathBuf,
        /// Path to a captured driver-report JSONL (replay mode). Mutually exclusive with --driver.
        #[arg(long)]
        report: Option<std::path::PathBuf>,
        /// Path to a driver binary to spawn (live mode). Mutually exclusive with --report.
        #[arg(long)]
        driver: Option<std::path::PathBuf>,
        /// Timeout in seconds for live (--driver) mode.
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        /// Optional path to write the canonical certificate JSON.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
```

- [ ] **Step 2: Remove the `Conformance` variant**

Delete the `Conformance { … }` variant from the `Command` enum (the `/// Run the conformance test suite against a driver implementation.` doc + the `Conformance { #[arg(long)] driver: std::path::PathBuf }` block) and the matching doc-comment line in the module header (`//! - \`rfl conformance --driver <binary>\` — run the conformance test suite`).

- [ ] **Step 3: Update the `Certify` handler arm to select the input mode**

In `fn main`, replace the start of the `Command::Certify { … } => { … }` arm. The current arm begins:

```rust
        Command::Certify { skill, embodiment, report, out } => {
            let outcome = match rfl_conformance::certify::run(&skill, &embodiment, &report) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("certify: invalid run: {e:#}");
                    std::process::exit(2);
                }
            };
```

Replace that opening with the mode selection (keep everything after `let cert = &outcome.certificate;` unchanged):

```rust
        Command::Certify { skill, embodiment, report, driver, timeout, out } => {
            let result = match (report, driver) {
                (Some(r), None) => rfl_conformance::certify::run(&skill, &embodiment, &r),
                (None, Some(d)) => rfl_conformance::certify::run_live(
                    &skill,
                    &embodiment,
                    &d,
                    std::time::Duration::from_secs(timeout),
                ),
                _ => {
                    eprintln!("certify: exactly one of --report or --driver is required");
                    std::process::exit(2);
                }
            };
            let outcome = match result {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("certify: invalid run: {e:#}");
                    std::process::exit(2);
                }
            };
```

- [ ] **Step 4: Remove the `Conformance` handler arm**

Delete the `Command::Conformance { driver } => { anyhow::bail!(…) }` arm from the `match cli.command` block.

- [ ] **Step 5: Build**

Run: `cargo build -p rfl-cli`
Expected: builds clean (no remaining references to `Conformance`).

- [ ] **Step 6: Write the CLI smoke test**

Create `crates/rfl-cli/tests/certify_driver_cli.rs`:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors
#![cfg(unix)]

//! Smoke test for `rfl certify --driver`: a mock driver emitting the committed report certifies
//! pass through the live path. Unix-only (CI is ubuntu + macos).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn example_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion")
}

#[test]
fn certify_driver_live_run_exits_zero() {
    let dir = example_dir();
    let report = dir.join("driver-report.jsonl");
    let mock = std::env::temp_dir().join(format!("rfl-cli-mock-{}.sh", std::process::id()));
    std::fs::write(&mock, format!("#!/bin/sh\ncat >/dev/null\ncat '{}'\n", report.display()))
        .unwrap();
    std::fs::set_permissions(&mock, std::fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_rfl"))
        .args(["certify", "--skill"])
        .arg(dir.join("skill.yaml"))
        .arg("--embodiment")
        .arg(dir.join("embodiments/allegro.yaml"))
        .arg("--driver")
        .arg(&mock)
        .output()
        .expect("run rfl certify --driver");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "exit {:?}, stdout: {stdout}", out.status.code());
    assert!(stdout.contains("RESULT: PASS"), "stdout: {stdout}");
    std::fs::remove_file(mock).ok();
}
```

- [ ] **Step 7: Run the CLI smoke**

Run: `cargo test -p rfl-cli --test certify_driver_cli`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/rfl-cli/src/main.rs crates/rfl-cli/tests/certify_driver_cli.rs
git commit -m "feat(cli): certify --driver live mode; remove conformance stub

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 4: Document live mode

**Files:**
- Modify: `docs/certifying-a-driver.md`

- [ ] **Step 1: Add a Live-mode subsection**

In `docs/certifying-a-driver.md`, inside `## Running rfl certify`, after the exit-code table, add:

```markdown
### Live mode (`--driver`)

Instead of capturing a report to a file, point `rfl certify` at your driver binary and it runs the
exchange for you:

```bash
rfl certify \
  --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml \
  --driver ./my_driver \
  --out certificate.json
```

The tool writes the canonical **execute** goals — exactly the output of `rfl retarget` — to your
driver's **stdin** (one JSON object per line, then EOF), and reads the `telemetry` + `status`
lines your driver writes to its **stdout**. Your driver should read goals from stdin, execute them,
emit its report on stdout, and exit `0`.

`--report` and `--driver` are mutually exclusive; supply exactly one. `--timeout <secs>` (default
`30`) bounds how long the driver may run; a driver that exits non-zero, overruns the timeout, or
emits a malformed report is an invalid run (exit `2`).
```

(Use a fenced ```bash and a nested ```markdown carefully — when editing, the inner code fence in the doc is a real ```bash block; the outer here is just this plan's quoting.)

- [ ] **Step 2: Verify the documented command works**

Run:
```bash
cd ~/Documents/GitHub/rfl
printf '#!/bin/sh\ncat >/dev/null\ncat examples/01-cable-insertion/driver-report.jsonl\n' > /tmp/md.sh && chmod +x /tmp/md.sh
cargo run -q -p rfl-cli -- certify --skill examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --driver /tmp/md.sh ; echo "exit=$?"
rm -f /tmp/md.sh
```
Expected: the certify summary + `RESULT: PASS`, `exit=0`.

- [ ] **Step 3: Commit**

```bash
git add docs/certifying-a-driver.md
git commit -m "docs: document certify --driver live mode

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 5: Full-suite verification (READ)

- [ ] **Step 1: Full workspace test (READ)**

Run: `cargo test --workspace`
Expected: all green, including `certify_live` (4) + `certify_driver_cli` (1) + the unchanged 7 certify + 3-case fixture guard; the removed `conformance` subcommand breaks nothing.

- [ ] **Step 2: Clippy (READ — my crates only; rfl-core pre-existing drift is out of scope)**

Run: `cargo clippy -p rfl-conformance -p rfl-cli --all-targets --all-features 2>&1 | rg -n "certify\.rs|main\.rs|certify_live\.rs|certify_driver_cli\.rs" -A2 | head`
Expected: no warnings attributed to the changed files.

- [ ] **Step 3: validate.py (READ)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | tail -3`
Expected: ends `PASS`.

- [ ] **Step 4: Manual replay + live equivalence smoke (READ)**

```bash
cd ~/Documents/GitHub/rfl
printf '#!/bin/sh\ncat >/dev/null\ncat examples/01-cable-insertion/driver-report.jsonl\n' > /tmp/md.sh && chmod +x /tmp/md.sh
cargo run -q -p rfl-cli -- certify --skill examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --driver /tmp/md.sh --out /tmp/live.json
cargo run -q -p rfl-cli -- certify --skill examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --report examples/01-cable-insertion/driver-report.jsonl --out /tmp/replay.json
diff <(jq -S . /tmp/live.json) <(jq -S . /tmp/replay.json) && echo "live == replay" ; rm -f /tmp/md.sh /tmp/live.json /tmp/replay.json
```
Expected: `live == replay` (the live and replay certificates are identical for the same report).

----

## Self-Review

**Spec coverage** (design doc → tasks):
- `certify_core` behavior-preserving refactor → Task 1. ✓
- `drive_subprocess` (concurrent I/O, EPIPE tolerance, poll/kill timeout) → Task 2. ✓
- `run_live` (retarget → to_jsonl → subprocess → core) → Task 2. ✓
- CLI `--driver`/`--timeout` XOR `--report`; remove `conformance` stub → Task 3. ✓
- live tests (pass / non-zero / malformed / timeout) + CLI smoke → Tasks 2–3. ✓
- docs live-mode subsection → Task 4. ✓
- exit-code taxonomy unchanged (invalid run → 2) → Task 3 handler. ✓

**Placeholder scan:** no TBD/TODO; every code step shows complete code; every command has an expected result. ✓

**Type consistency:** `certify_core(&[u8], &[u8], &str) -> Result<CertifyOutcome>`; `drive_subprocess(&Path, &str, Duration) -> Result<String>`; `run_live(&Path, &Path, &Path, Duration) -> Result<CertifyOutcome>`; CLI passes `Duration::from_secs(timeout)`; tests use `certify::{run_live, CertResult}` (both `pub`). The `report_sha256` over `report_text.as_bytes()` equals the prior `&report_bytes` (UTF-8 validation is byte-preserving), so committed certs are unaffected. ✓

**Behavior-preservation invariant:** Task 1 must leave the 7 certify tests + the 3-case fixture guard green before any new feature lands (Step 2-3 gate). ✓
