# Certificate Fidelity Tier + Certify Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record the achieved fidelity tier per action in the certificate (spec/05 badge), and exercise the proxy-degradation and sequence-`momentary_release` certify paths end-to-end with committed reference certs.

**Architecture:** Thread `fidelity_tier` from `report.status.fidelity_tier` through `battery::ActionVerdict` → `certificate::ActionEntry` (one optional field at each layer); extend the cert schema; generalize the bless-pattern fixture guard to a table covering cable@allegro (manifold), cable@pneumatic-6f (proxy), and screw-fasten/skill-flip@allegro (sequence). Pure addition — no `check_*` or canonical-hash change.

**Tech Stack:** Rust (edition 2024), `serde`/`serde_json`, `boon` 0.6, `sha2` 0.10, `clap` 4; `jsonschema`/`pyyaml` for `validate.py`.

**Design doc:** `docs/design/2026-06-01-certificate-fidelity-tier-design.md` (committed `0595cf0`).

**Discipline (every task):** inline TDD red→green; explicit `git add <paths>` (never `-A`); Conventional Commits ≤72 imperative; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after (the parallel `cobel` session is finished but re-verify `git log` each batch); full `cargo test --workspace` + `validate.py` read separate from the commit.

----

## File Structure

- `crates/rfl-conformance/src/battery.rs` — MODIFY. `ActionVerdict` gains `fidelity_tier`; `verify_action` sets it from `report.status.fidelity_tier`.
- `crates/rfl-conformance/src/certificate.rs` — MODIFY. `ActionEntry` gains `fidelity_tier`; the `sample_body` test helper's `ActionEntry` literal updates.
- `crates/rfl-conformance/src/certify.rs` — MODIFY. `action_entry` maps `fidelity_tier` through.
- `schemas/certificate.schema.json` — MODIFY. `ActionEntry` gains an optional `fidelity_tier` enum.
- `schemas/validate.py` — MODIFY. Validate every `examples/*/certificate*.json` (glob).
- `crates/rfl-conformance/tests/certify.rs` — MODIFY. Add fidelity + proxy + flip coverage tests.
- `crates/rfl-conformance/tests/certificate_fixtures.rs` — MODIFY. Generalize to a table of cases.
- `examples/01-cable-insertion/certificate.json` — REGENERATE (now carries `fidelity_tier`).
- `examples/01-cable-insertion/driver-report-pneumatic.jsonl`, `certificate-pneumatic.json` — NEW (proxy).
- `examples/03-screw-fasten/driver-report-flip.jsonl`, `certificate-flip.json` — NEW (sequence).

----

## Task 1: `fidelity_tier` through the cert + schema + validate.py glob

**Files:**
- Modify: `crates/rfl-conformance/src/battery.rs`, `crates/rfl-conformance/src/certificate.rs`, `crates/rfl-conformance/src/certify.rs`, `schemas/certificate.schema.json`, `schemas/validate.py`
- Modify: `crates/rfl-conformance/tests/certify.rs`
- Regenerate: `examples/01-cable-insertion/certificate.json`

- [ ] **Step 1: Write the failing coverage test**

In `crates/rfl-conformance/tests/certify.rs`, append:

```rust
#[test]
fn certificate_records_per_action_fidelity_tier() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("fidelity", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    let actions = &outcome.certificate.body.actions;
    // grasp.pinch confirms at manifold tier on allegro (tactile sensing present).
    let pinch = actions.iter().find(|a| a.suffix == "pinch").unwrap();
    assert_eq!(pinch.fidelity_tier.as_deref(), Some("manifold"));
    // a pure reach has no confirmation tier.
    let reach = actions.iter().find(|a| a.suffix == "align" || a.suffix == "retract").unwrap();
    assert_eq!(reach.fidelity_tier, None);
    std::fs::remove_file(report).ok();
}
```

- [ ] **Step 2: Run it to verify it fails to compile**

Run: `cargo test -p rfl-conformance --test certify certificate_records_per_action_fidelity_tier`
Expected: compile error — `ActionEntry` has no field `fidelity_tier`.

- [ ] **Step 3: Add `fidelity_tier` to `ActionVerdict` (battery.rs)**

In `crates/rfl-conformance/src/battery.rs`, in `struct ActionVerdict`, add the field after `envelope_class`:

```rust
    /// The envelope class verified (None for perception primitives with no motion envelope).
    pub envelope_class: Option<EnvelopeClass>,
    /// The fidelity tier achieved (`report.status.fidelity_tier`); None for actions with no
    /// confirmation tier (reach / perception).
    pub fidelity_tier: Option<String>,
```

and in `verify_action`, set it in the returned struct:

```rust
    let passed = checks.iter().all(|c| matches!(c.outcome, CheckOutcome::Pass));
    let fidelity_tier = report.status.fidelity_tier.clone();
    ActionVerdict { action_id: goal.action_id.clone(), suffix, envelope_class, fidelity_tier, checks, passed }
```

- [ ] **Step 4: Add `fidelity_tier` to `ActionEntry` (certificate.rs)**

In `crates/rfl-conformance/src/certificate.rs`, in `struct ActionEntry`, add after `envelope_class`:

```rust
    /// The envelope class verified (absent for perception primitives).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope_class: Option<&'static str>,
    /// The fidelity tier achieved (`spec/05` badge); absent for actions with no confirmation tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_tier: Option<String>,
```

and in the `sample_body` test helper, add `fidelity_tier` to the `ActionEntry` literal:

```rust
            actions: vec![ActionEntry {
                action_id: "cable-insertion/allegro/0002-pinch".into(),
                suffix: "pinch".into(),
                envelope_class: Some("grasp_continuity"),
                fidelity_tier: Some("manifold".to_string()),
                checks: vec![CheckEntry { name: "envelope", result: "pass", reason: None }],
                passed: true,
            }],
```

- [ ] **Step 5: Map it through in `certify::action_entry` (certify.rs)**

In `crates/rfl-conformance/src/certify.rs`, in `fn action_entry`, add the field:

```rust
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
```

- [ ] **Step 6: Add `fidelity_tier` to the schema**

In `schemas/certificate.schema.json`, in `$defs.ActionEntry.properties`, add after `envelope_class`:

```json
        "envelope_class": {
          "enum": ["terminal_postcondition", "grasp_continuity", "force_trajectory", "interval_invariant"]
        },
        "fidelity_tier": { "enum": ["manifold", "proxy", "proxy_reactive"] },
```

- [ ] **Step 7: Validate every example cert in validate.py**

In `schemas/validate.py`, replace the single-cert validation block:

```python
    cert_validator = Draft202012Validator(certificate)
    cert_path = EXAMPLES / "certificate.json"
    errs = list(cert_validator.iter_errors(json.loads(cert_path.read_text())))
    check("certificate.json vs certificate-schema", not errs, errs[0].message if errs else "")
```

with a glob over all example certs:

```python
    cert_validator = Draft202012Validator(certificate)
    for cert_file in sorted((ROOT / "examples").glob("*/certificate*.json")):
        errs = list(cert_validator.iter_errors(json.loads(cert_file.read_text())))
        label = f"{cert_file.parent.name}/{cert_file.name}"
        check(f"{label} vs certificate-schema", not errs, errs[0].message if errs else "")
```

- [ ] **Step 8: Run the coverage test (green)**

Run: `cargo test -p rfl-conformance --test certify certificate_records_per_action_fidelity_tier`
Expected: PASS.

- [ ] **Step 9: Regenerate the allegro fixture (it now carries fidelity_tier)**

Run: `RFL_BLESS=1 cargo test -p rfl-conformance --test certificate_fixtures`
Then: `cargo test -p rfl-conformance --test certificate_fixtures`
Expected: PASS (the committed `examples/01-cable-insertion/certificate.json` now includes `fidelity_tier` and is current).

- [ ] **Step 10: Confirm validate.py still green**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | rg "certificate|PASS|FAIL"`
Expected: `01-cable-insertion/certificate.json vs certificate-schema` ok; ends `PASS`.

- [ ] **Step 11: Clippy + commit**

Run: `cargo clippy -p rfl-conformance --lib --all-features 2>&1 | rg -c "battery\.rs|certificate\.rs|certify\.rs" || echo 0`
Expected: `0`.

```bash
git add crates/rfl-conformance/src/battery.rs crates/rfl-conformance/src/certificate.rs crates/rfl-conformance/src/certify.rs schemas/certificate.schema.json schemas/validate.py crates/rfl-conformance/tests/certify.rs examples/01-cable-insertion/certificate.json
git commit -m "feat(certify): record achieved fidelity tier per action

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 2: Proxy-tier coverage (cable @ pneumatic-6f)

**Files:**
- Modify: `crates/rfl-conformance/tests/certify.rs`
- Modify: `crates/rfl-conformance/tests/certificate_fixtures.rs`
- Create: `examples/01-cable-insertion/driver-report-pneumatic.jsonl`, `examples/01-cable-insertion/certificate-pneumatic.json`

- [ ] **Step 1: Write the proxy coverage test**

In `crates/rfl-conformance/tests/certify.rs`, append:

```rust
#[test]
fn certifies_proxy_fidelity_on_pneumatic() {
    let dir = example_dir();
    let skill = dir.join("skill.yaml");
    let emb = dir.join("embodiments/pneumatic-6f.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("pneumatic", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Pass, "honest proxy must still pass");
    // pneumatic-6f has no tactile sensing -> grasp.pinch confirms at proxy tier.
    let pinch = outcome.certificate.body.actions.iter().find(|a| a.suffix == "pinch").unwrap();
    assert_eq!(pinch.fidelity_tier.as_deref(), Some("proxy"));
    // and the audit-honesty check passed for it (a proxy claim against a proxy lowering).
    let audit = pinch.checks.iter().find(|c| c.name == "audit_honesty").unwrap();
    assert_eq!(audit.result, "pass");
    std::fs::remove_file(report).ok();
}
```

- [ ] **Step 2: Run it (green — uses the field added in Task 1)**

Run: `cargo test -p rfl-conformance --test certify certifies_proxy_fidelity_on_pneumatic`
Expected: PASS.

- [ ] **Step 3: Generalize the fixture guard to a table**

Replace the entire body of `crates/rfl-conformance/tests/certificate_fixtures.rs` with:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Guards the committed example fixtures (driver-report*.jsonl + certificate*.json). Run with
//! `RFL_BLESS=1` to (re)generate them; otherwise asserts each is current and verifies.

use std::path::Path;

struct Case {
    dir: &'static str,
    skill: &'static str,
    embodiment: &'static str,
    report: &'static str,
    cert: &'static str,
}

const CASES: &[Case] = &[
    Case {
        dir: "01-cable-insertion",
        skill: "skill.yaml",
        embodiment: "allegro.yaml",
        report: "driver-report.jsonl",
        cert: "certificate.json",
    },
    Case {
        dir: "01-cable-insertion",
        skill: "skill.yaml",
        embodiment: "pneumatic-6f.yaml",
        report: "driver-report-pneumatic.jsonl",
        cert: "certificate-pneumatic.json",
    },
];

#[test]
fn example_fixtures_are_current() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let bless = std::env::var("RFL_BLESS").is_ok();
    for c in CASES {
        let dir = examples.join(c.dir);
        let skill = dir.join(c.skill);
        let emb = dir.join("embodiments").join(c.embodiment);
        let report = rfl_conformance::reports_to_jsonl(
            &rfl_conformance::run_reference_driver(&skill, &emb).unwrap(),
        );
        let report_path = dir.join(c.report);
        let cert_path = dir.join(c.cert);

        if bless {
            std::fs::write(&report_path, &report).unwrap();
            let cert =
                rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
            std::fs::write(&cert_path, rfl_conformance::certificate::to_json(&cert)).unwrap();
            continue;
        }

        let committed_report = std::fs::read_to_string(&report_path)
            .unwrap_or_else(|_| panic!("{} present (run RFL_BLESS=1 to generate)", c.report));
        assert_eq!(committed_report, report, "{} stale; run RFL_BLESS=1 cargo test", c.report);

        let fresh = rfl_conformance::certify::run(&skill, &emb, &report_path).unwrap().certificate;
        let committed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cert_path).unwrap()).unwrap();
        assert_eq!(
            committed.get("content_hash").and_then(serde_json::Value::as_str),
            Some(fresh.content_hash.as_str()),
            "{} stale; run RFL_BLESS=1 cargo test",
            c.cert
        );

        let verified = rfl_conformance::certificate::verify_certificate(
            &std::fs::read_to_string(&cert_path).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{} not schema-valid: {e}", c.cert));
        assert!(verified.matches, "{} fails its own content_hash", c.cert);
    }
}
```

- [ ] **Step 4: Generate the pneumatic fixtures**

Run: `RFL_BLESS=1 cargo test -p rfl-conformance --test certificate_fixtures`
Then: `cargo test -p rfl-conformance --test certificate_fixtures`
Expected: both PASS; `examples/01-cable-insertion/driver-report-pneumatic.jsonl` + `certificate-pneumatic.json` now exist.

- [ ] **Step 5: Validate the new cert + verify it**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | rg "pneumatic|PASS|FAIL"`
Expected: `01-cable-insertion/certificate-pneumatic.json vs certificate-schema` ok; ends `PASS`.

Run: `cargo run -q -p rfl-cli -- verify examples/01-cable-insertion/certificate-pneumatic.json ; echo "exit=$?"`
Expected: `VERIFIED: …`, `exit=0`.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/tests/certify.rs crates/rfl-conformance/tests/certificate_fixtures.rs examples/01-cable-insertion/driver-report-pneumatic.jsonl examples/01-cable-insertion/certificate-pneumatic.json
git commit -m "test(certify): proxy-tier reference cert (cable @ pneumatic-6f)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 3: Sequence coverage (screw-fasten/skill-flip)

**Files:**
- Modify: `crates/rfl-conformance/tests/certify.rs`
- Modify: `crates/rfl-conformance/tests/certificate_fixtures.rs`
- Create: `examples/03-screw-fasten/driver-report-flip.jsonl`, `examples/03-screw-fasten/certificate-flip.json`

- [ ] **Step 1: Write the flip sequence coverage test**

In `crates/rfl-conformance/tests/certify.rs`, append:

```rust
#[test]
fn certifies_flip_sequence_momentary_release() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
    let skill = dir.join("skill-flip.yaml");
    let emb = dir.join("embodiments/allegro.yaml");
    let reports = run_reference_driver(&skill, &emb).unwrap();
    let report = temp_report("flip", &reports_to_jsonl(&reports));

    let outcome = certify::run(&skill, &emb, &report).expect("valid run");
    assert_eq!(outcome.result, CertResult::Pass);
    // the skill contains an in_hand.flip -> a flip action is present (the sequence check has
    // something to trace), making momentary_release non-vacuous.
    assert!(
        outcome.certificate.body.actions.iter().any(|a| a.suffix == "flip"),
        "skill-flip should retarget to a flip action"
    );
    let seq = outcome
        .certificate
        .body
        .sequence_checks
        .iter()
        .find(|c| c.name == "momentary_release")
        .unwrap();
    assert_eq!(seq.result, "pass");
    std::fs::remove_file(report).ok();
}
```

- [ ] **Step 2: Run it (green)**

Run: `cargo test -p rfl-conformance --test certify certifies_flip_sequence_momentary_release`
Expected: PASS. (If `result != pass`, read the failing action's reason in `outcome.certificate.body.actions` — a nominal `ReferenceDriver` should conform; investigate before forcing.)

- [ ] **Step 3: Add the flip case to the fixture table**

In `crates/rfl-conformance/tests/certificate_fixtures.rs`, append to `CASES` (inside the array, after the pneumatic entry):

```rust
    Case {
        dir: "03-screw-fasten",
        skill: "skill-flip.yaml",
        embodiment: "allegro.yaml",
        report: "driver-report-flip.jsonl",
        cert: "certificate-flip.json",
    },
```

- [ ] **Step 4: Generate the flip fixtures**

Run: `RFL_BLESS=1 cargo test -p rfl-conformance --test certificate_fixtures`
Then: `cargo test -p rfl-conformance --test certificate_fixtures`
Expected: both PASS; `examples/03-screw-fasten/driver-report-flip.jsonl` + `certificate-flip.json` now exist.

- [ ] **Step 5: Validate + verify the new cert**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | rg "flip|PASS|FAIL"`
Expected: `03-screw-fasten/certificate-flip.json vs certificate-schema` ok; ends `PASS`.

Run: `cargo run -q -p rfl-cli -- verify examples/03-screw-fasten/certificate-flip.json ; echo "exit=$?"`
Expected: `VERIFIED: …`, `exit=0`.

- [ ] **Step 6: Commit**

```bash
git add crates/rfl-conformance/tests/certify.rs crates/rfl-conformance/tests/certificate_fixtures.rs examples/03-screw-fasten/driver-report-flip.jsonl examples/03-screw-fasten/certificate-flip.json
git commit -m "test(certify): sequence momentary_release reference cert (skill-flip)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

----

## Task 4: Full-suite verification (READ)

- [ ] **Step 1: Full workspace test (READ, separate from any commit)**

Run: `cargo test --workspace`
Expected: all green, including the 3 new certify coverage tests + the 3-case fixture guard.

- [ ] **Step 2: validate.py (READ)**

Run: `uv run --with jsonschema --with pyyaml python schemas/validate.py 2>&1 | rg "certificate|PASS"`
Expected: three `… vs certificate-schema` ok lines (allegro / pneumatic / flip) + `check_schema certificate` ok; ends `PASS`.

- [ ] **Step 3: Inspect a proxy cert's fidelity tier (READ)**

```bash
cd ~/Documents/GitHub/rfl
rg '"fidelity_tier"' examples/01-cable-insertion/certificate-pneumatic.json
```
Expected: at least one `"fidelity_tier": "proxy"` (the grasp.pinch action).

----

## Self-Review

**Spec coverage** (design doc → tasks):
- per-action `fidelity_tier` (ActionVerdict + ActionEntry + certify map + schema) → Task 1. ✓
- validate.py globs all example certs → Task 1 Step 7. ✓
- proxy coverage (cable @ pneumatic-6f) + reference cert → Task 2. ✓
- sequence coverage (skill-flip) + reference cert → Task 3. ✓
- generalized bless-pattern fixture guard → Tasks 2–3. ✓
- tests (manifold/proxy fidelity, flip sequence, verify each cert, validate each) → Tasks 1–4. ✓
- YAGNI (no top-level summary, no full matrix, proxy_reactive enum-only) → respected. ✓

**Placeholder scan:** no TBD/TODO; every code step shows complete code; every command has an expected result. ✓

**Type consistency:** `ActionVerdict.fidelity_tier: Option<String>` (battery) → `ActionEntry.fidelity_tier: Option<String>` (certificate) ← `v.fidelity_tier.clone()` (certify); schema enum `[manifold, proxy, proxy_reactive]`; tests read `a.fidelity_tier.as_deref()` and `a.suffix`/`c.name`/`c.result` consistent with existing cert types. The `Case` struct fields (dir/skill/embodiment/report/cert) are used uniformly across Tasks 2–3. ✓

**Ordering invariant:** Task 1 changes the cert body shape → the inc-2 allegro fixture goes stale → Task 1 Step 9 regenerates it before any commit asserts currency. Tasks 2–3 each add a case + generate its fixtures before asserting. ✓
