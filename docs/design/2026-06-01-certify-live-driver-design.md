# Design — `certify --driver`: live stdio driver mode

**Date:** 2026-06-01
**Track:** conformance-certify — increment 5 (live driver mode)
**Status:** approved
**Scope:** `rfl-conformance` (`certify` — extract `certify_core`, add `drive_subprocess` + `run_live`), `rfl-cli` (`certify --driver`/`--timeout`, remove the `conformance` stub), `docs/certifying-a-driver.md` (a Live-mode subsection). **No change to `replay` / `battery` / `certificate`**, **no `rfl-core` change**, **no schema change**.

## 0. Why this increment

v0 certify takes a *captured* driver report (`--report file.jsonl`); the vendor must orchestrate the capture out-of-band. The `conformance --driver` subcommand has been a bailing stub since the start, documented as the future live path. This increment delivers it: `rfl certify --driver ./my_driver` spawns the driver, exchanges over stdio, and certifies in one command. It is the last headline feature of the input story (replay → live).

The certify logic is already transport-agnostic — the driver's stdout *is* the replay input, captured live — so the new work is almost entirely careful subprocess handling.

## 1. Protocol (batch over stdio)

- The tool writes all canonical **execute** goals — byte-for-byte the `rfl retarget` output (`canonical::to_jsonl`) — to the driver's **stdin**, one JSON object per line, then closes stdin (EOF).
- The driver executes each action and writes **`telemetry`** + **`status`** lines to its **stdout**, then exits 0.
- The tool reads stdout to EOF; that stream is the driver report, fed into the existing `replay → correlate → battery → seal` pipeline.

Batch (send-all / read-all), not a request-response loop: it matches the replay model exactly (the only difference from `--report` is *where the report bytes come from*), and needs no per-action framing or synchronization. The symmetry is the contract: **stdin is `rfl retarget` output; stdout is a driver report.**

## 2. `certify_core` (behavior-preserving refactor)

Extract the report-to-outcome core from `run`:

```rust
fn certify_core(skill_bytes: &[u8], emb_bytes: &[u8], report_text: &str) -> Result<CertifyOutcome>
```

It parses the skill + embodiment from the bytes, retargets, replays `report_text`, correlates the expected goal set, runs the battery, and seals — with `FileRef.sha256` from `skill_bytes` / `emb_bytes` and `report_sha256` from `report_text`'s bytes. `run` becomes: read the three files → `certify_core`. The seven existing certify tests must stay green (pure refactor).

## 3. `drive_subprocess` (the new, careful piece)

```rust
fn drive_subprocess(driver: &Path, goals: &str, timeout: Duration) -> Result<String>
```

Spawn `driver` with piped stdin/stdout/stderr, then:

- **stdin-writer thread:** writes `goals` to the child's stdin and drops it (EOF). It **ignores a broken-pipe / write error** — a driver that emits a full report without consuming all its input is valid, not a failure.
- **stdout-reader thread** (and a stderr-reader thread): read to EOF concurrently with the writer. Concurrency is mandatory: writing all of stdin while the driver fills its stdout pipe, with nobody draining stdout, deadlocks both sides.
- **main thread:** polls `child.try_wait()` in a `~10 ms` loop against `Instant::now()` elapsed vs `timeout`; on timeout, `child.kill()` + `wait()` and return `Err`. On exit, join the threads, and if the status is non-zero return `Err` (including the captured stderr, trimmed).

Dependency-free (`std::process`, `std::thread`, `std::time`). Returns the captured stdout on success.

## 4. `run_live`

```rust
pub fn run_live(skill_path: &Path, embodiment_path: &Path, driver_path: &Path, timeout: Duration) -> Result<CertifyOutcome>
```

Read skill + embodiment bytes; parse them; `retarget`; `canonical::to_jsonl(&skill.skill, &emb.id, &out.actions, &out.suffixes)` → the goals JSONL; `drive_subprocess(driver_path, &goals, timeout)` → the report text; `certify_core(&skill_bytes, &emb_bytes, &report_text)`. (The double-parse of skill/emb — once here for retarget, once inside `certify_core` — is cheap and keeps `certify_core` self-contained.)

## 5. CLI

`certify` gains two args and the input becomes a choice:

- `--report <file>` (replay) and `--driver <bin>` (live) are **mutually exclusive**; **exactly one is required**. Neither / both → an error to stderr, exit **2**.
- `--timeout <secs>` (default `30`) bounds live mode.

The exit-code taxonomy is unchanged and extends to live for free: spawn-failure / non-zero driver exit / timeout / malformed-or-uncorrelated stdout all surface as `Err` from `run_live` → **exit 2** (couldn't run); a well-formed but non-conformant report → **exit 1** (certificate emitted); all-conform → **exit 0**.

**Remove the `conformance` subcommand stub** (variant + handler arm + doc line): its purpose — run the conformance suite against a driver — is now exactly `certify --driver`. Per the no-cruft rule, delete it rather than leave a bailing alias.

## 6. Testing

`#[cfg(unix)]` (CI matrix is `ubuntu-latest` + `macos-latest` — both unix). A helper writes an executable shell-script mock driver (`#!/bin/sh`, drains stdin with `cat >/dev/null`, then the body):

- **Happy path:** body `cat '<committed driver-report.jsonl>'` → `run_live` on cable-insertion/allegro → `result == Pass`, and the cert's `content_hash` equals the committed allegro `certificate.json`'s (live ≡ replay for the same report).
- **Non-zero exit:** body `exit 3` → `run_live` → `Err` (invalid run).
- **Malformed stdout:** body `echo "not a json line"` → `run_live` → `Err` (replay rejects it).
- **Timeout:** body `sleep 30`, `timeout = 1 s` → `run_live` → `Err`, and the call returns promptly (the child is killed).

Plus the existing seven certify tests stay green (the `certify_core` refactor is behavior-preserving), and a `rfl-cli` smoke test could exercise `certify --driver <mock>` end-to-end — but the library-level `run_live` tests already cover the plumbing; the CLI arg-routing (XOR `--report`/`--driver`) is covered by a small unit/integration assertion. Full `cargo test --workspace` + `validate.py` read separately.

## 7. Out of scope (YAGNI)

Request-response framing (batch suffices for a finite action sequence); ROS 2 transport (a separate, deferred track — this is a convenience stdio path, not the production transport); streaming / interleaved multi-action execution; configurable stdin encodings; a Windows mock (CI is unix; if Windows CI is added later, swap the shell mock for a small Rust `[[bin]]`).
