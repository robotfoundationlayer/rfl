# Design — certificate records achieved fidelity tier + certify coverage across the spectrum

**Date:** 2026-06-01
**Track:** conformance-certify — increment 3 (fidelity tier + coverage)
**Status:** approved
**Scope:** `rfl-conformance` (`battery::ActionVerdict` + `certify::action_entry` + `certificate::ActionEntry` gain `fidelity_tier`), `schemas/certificate.schema.json` (ActionEntry enum), `schemas/validate.py` (glob all example certs), `crates/rfl-conformance/tests/` (certify coverage + generalized fixture guard), `examples/` (two new reference certs + their reports). **No `rfl-core` change**, **no change to any `check_*`**, **no change to the canonical-hash recipe** (the new field simply participates in the sorted-key body hash).

## 0. Why this increment

The certificate claims Class 3 conformance but omits *how well* each capability confirmed. `spec/05` § Fidelity tier and the badge says the badge records, per capability, the **fidelity tier achieved** (`manifold` > `proxy` > `proxy_reactive`). The report already carries `status.fidelity_tier`, and `check_audit_honesty` already guards it — but `ActionEntry` drops it, so an adopter reading a certificate cannot see the tier. This increment lands the fidelity dimension on the artifact.

It also closes a coverage gap: certify is exercised only on **cable-insertion / allegro**, which is all-`manifold` and has no flip. Two real behaviors the certify orchestration must handle are therefore only unit-tested, never run end-to-end through certify:

- **proxy-tier degradation** — `pneumatic-6f` has no tactile sensing, so `grasp.pinch` lowers to `proxy`; this is the only case where `check_audit_honesty`'s `expected = "proxy"` branch runs;
- **sequence-level `momentary_release`** — only a skill containing `in_hand.flip` makes `check_momentary_release` non-vacuous; `examples/03-screw-fasten/skill-flip.yaml` is exactly that.

Both inputs already exist in `examples/`; certifying them turns two dormant integration paths live and yields reference certs that show the proxy + sequence dimensions.

## 1. Per-action fidelity tier

The achieved tier is `report.status.fidelity_tier` (set by the driver from the lowered `tactile_target`: `Auto → manifold`, `Proxy → proxy`; absent for reach/perception actions with no confirmation).

- `battery::ActionVerdict` gains `pub fidelity_tier: Option<String>`, set in `verify_action` from `report.status.fidelity_tier.clone()`. No check reads it; it is reporting metadata, distinct from the pass/fail obligations.
- `certificate::ActionEntry` gains `pub fidelity_tier: Option<String>` (`#[serde(skip_serializing_if = "Option::is_none")]` — actions with no confirmation tier omit it).
- `certify::action_entry` maps `v.fidelity_tier.clone()` through.
- `certificate.schema.json` `ActionEntry` gains an optional property `fidelity_tier: { enum: [manifold, proxy, proxy_reactive] }` (the closed tier set; `additionalProperties: false` already required declaring it).

Field placement: after `envelope_class` for readability. The canonical content hash is sorted-key, so declaration order does not affect it — but the existing committed cert's `content_hash` changes because the body gains a field; the bless guard regenerates it.

## 2. Certify coverage across the spectrum (committed reference certs)

Generalize the bless-pattern fixture guard (`tests/certificate_fixtures.rs`) from one case to a table:

| skill | embodiment | report | cert | exercises |
|---|---|---|---|---|
| `01-cable-insertion/skill.yaml` | allegro | `driver-report.jsonl` | `certificate.json` | manifold (regenerated with `fidelity_tier`) |
| `01-cable-insertion/skill.yaml` | pneumatic-6f | `driver-report-pneumatic.jsonl` | `certificate-pneumatic.json` | proxy tier + `check_audit_honesty` `expected="proxy"` |
| `03-screw-fasten/skill-flip.yaml` | allegro | `driver-report-flip.jsonl` | `certificate-flip.json` | non-vacuous `sequence_checks` momentary_release |

Each fixture's report + cert live in the same example dir as its skill. The guard, per case: regenerate the report from `ReferenceDriver`; with `RFL_BLESS=1` write report + cert, else assert the committed report byte-equals the reference driver, a fresh `certify::run` reproduces the committed cert's `content_hash`, and the committed cert verifies.

`validate.py`: replace the single-cert validation with a glob over `ROOT/examples/*/certificate*.json`, validating each against `certificate.schema.json` (label `<dir>/<name>`).

## 3. Data flow

`report.status.fidelity_tier` → `ActionVerdict.fidelity_tier` → `ActionEntry.fidelity_tier` → certificate JSON → (sorted-key canonical body) → `content_hash`. Pure addition; the verify path is unchanged (the new field is just more sorted body content).

## 4. Testing

- **certify on allegro** (cable): the `grasp.pinch` action's `fidelity_tier == Some("manifold")`; a `reach.*` action's is `None`; `result == pass`.
- **certify on pneumatic-6f** (cable): the `pinch` action's `fidelity_tier == Some("proxy")`; `result == pass` (honest proxy — `check_audit_honesty` passes on the `expected="proxy"` branch).
- **certify on skill-flip** (screw-fasten / allegro): an action with suffix `flip` is present; `sequence_checks` contains `momentary_release` with `result == pass`, and it is non-vacuous (the flip declares + propagates).
- **generalized fixture guard** covers all three (bless regenerates; default asserts current + verifies each).
- **`rfl verify`** each committed cert → exit 0.
- **`validate.py`** validates all three committed certs.
- Full `cargo test --workspace` + `validate.py` READ in a batch separate from each commit.

## 5. Out of scope (YAGNI)

A top-level certificate "fidelity floor" summary (the spec badge is per-capability; per-action suffices — an adopter can take the floor themselves); certifying the full 3×3×N example matrix (the three cases cover the distinct paths — manifold, proxy, sequence — without exhaustive fixtures); `proxy_reactive` (no example embodiment produces it yet; the enum admits it for when one does).
