# Certifying-a-Driver Adoption Guide Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write `docs/certifying-a-driver.md`, the third-party on-ramp for `rfl certify` / `rfl verify`, with every command and output verified against the committed example fixtures.

**Architecture:** A single standalone markdown doc, six sections, grounded in the committed tooling + fixtures (no code change). The re-verification recipe is the verified jq/Python incantation that reproduces the committed cert's `content_hash`. DRY: point to `driver-interface.schema.json` (normative), don't re-document it.

**Tech Stack:** Markdown; `rfl` CLI; `jq` / `python3` (for the re-verification recipe).

**Design doc:** `docs/design/2026-06-01-certifying-a-driver-guide-design.md` (committed `7acd0db`).

**Discipline:** explicit `git add <paths>` (never `-A`); Conventional Commits ≤72; after each commit `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` = `0 0` before AND ff `git push origin main` after; verify every command in the doc actually runs and matches.

## Captured source material (all real, verified this session)

**Certify summary** (`rfl certify --skill examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --report examples/01-cable-insertion/driver-report.jsonl`):
```
RFL conformance certificate — cable-insertion on wonik-allegro-v4 (spec v0.1-draft)
  [PASS] cable-insertion/wonik-allegro-v4/0001-locate (perception)
  [PASS] cable-insertion/wonik-allegro-v4/0002-pinch (grasp_continuity)
  ... (0003-transport, 0004-locate, 0005-align, 0006-insert_fit, 0007-release, 0008-retract) ...
  [PASS] sequence:momentary_release
RESULT: PASS (class3_driver_protocol covered, env3/class4 excluded) — sha256:4c9ff53d…
```

**Wire-format lines** (raw compact, exactly as a driver emits them, from `driver-report.jsonl`):
```jsonl
{"message":"telemetry","action_id":"cable-insertion/wonik-allegro-v4/0001-locate","t":1.0,"realized_pose":{"position":[0.0,0.0,0.0],"orientation":[0.0,0.0,0.0,1.0]}}
{"message":"status","action_id":"cable-insertion/wonik-allegro-v4/0002-pinch","outcome":"succeeded","verdict":{"value":true,"confidence":1.0,"evidence":["nominal reference-driver execution"]},"fidelity_tier":"manifold","final_pose":{"position":[0.0,0.0,0.0],"orientation":[0.0,0.0,0.0,1.0]}}
```

**Certificate identity + claims** (from `certificate.json`): `skill {id: cable-insertion, sha256: 27e609a0…}`, `embodiment {id: wonik-allegro-v4, sha256: c67f1bd1…}`, `report_sha256: fcf9e82e…`, `result: pass`, `covered: ["class3_driver_protocol"]`, `excluded: ["class4_physical","class2_loose_epsilon","env3_disturbance","fidelity_tier_physical_truth"]`, `content_hash: sha256:4c9ff53d…`.

**A certified action** (manifold): `0002-pinch`, `envelope_class: grasp_continuity`, `fidelity_tier: manifold`, six checks all `pass`, `passed: true`. **Proxy variant** (`certificate-pneumatic.json`, embodiment `generic-pneumatic-6f`): `0002-pinch`, `fidelity_tier: proxy`, `passed: true`.

**Verify outputs:** `VERIFIED: content_hash sha256:4c9ff53d… matches` (exit 0); `TAMPERED: declared sha256:4c9ff53d… != recomputed sha256:cf108ee5…` (exit 1); malformed → `verify: malformed certificate: certificate schema violation: …` (exit 2).

**Verified re-verification recipe** (both reproduce `sha256:4c9ff53d…`):
- jq (newline-safe): `printf '%s' "$(jq -S -c 'del(.content_hash)' certificate.json)" | shasum -a 256`
- Python: `json.dumps(d, sort_keys=True, separators=(",", ":"))` over the body (minus `content_hash`), then sha256.
- The bare `jq -S -c … | shasum` form is WRONG (`efab16…`) — `jq -c` appends a trailing newline `serde_json::to_vec` does not.

----

## Task 1: Write `docs/certifying-a-driver.md`

**Files:**
- Create: `docs/certifying-a-driver.md`

- [ ] **Step 1: Write the doc (six sections, using the captured material)**

Create `docs/certifying-a-driver.md` with:

1. **`# Certifying a driver`** intro + a one-paragraph overview: the certificate is **Class 3 driver-protocol self-certification** (link `[spec/05-conformance.md](../spec/05-conformance.md)` for the normative test classes); the flow is three steps — **capture** a driver report, **`rfl certify`**, **`rfl verify`**.

2. **`## The driver-report wire format`** — a vendor's driver emits one JSON object per line (JSONL), each a `telemetry` or `status` message (discriminated by `message`), correlated by `action_id`, with **exactly one terminal `status` per action**. The normative contract is `[schemas/driver-interface.schema.json](../schemas/driver-interface.schema.json)` — point to it, do not restate it. Embed the two captured raw lines in a ```jsonl block, then a sentence each: telemetry = a sample (pose / wrench / securing_force / events / fidelity_tier as the primitive needs); status = the terminal outcome + `verdict` + (when a confirmation tier applies) `fidelity_tier`.

3. **`## Running rfl certify`** — the command (three inputs), `--out certificate.json`, the captured summary in a ```text block, and the exit codes: **0** every action conforms; **1** a non-conformance (the certificate is still emitted, recording the failure); **2** the run is invalid (unparseable input / broken correlation — no certificate). Note the goal/contract side is derived by the tool via `retarget`, so a vendor cannot forge what they are judged against.

4. **`## Reading the certificate`** — the identity block (`skill`/`embodiment`/`report_sha256` = RFL id + content sha256, never a path), `result`, then **`covered` vs `excluded`** (spell out: it claims Class 3 driver-protocol; it disclaims class4/class2-loose/env3/fidelity-physical-truth). Embed the manifold `0002-pinch` action (annotate: `envelope_class`, the six obligation `checks`, `fidelity_tier`), then the proxy variant line (annotate: `proxy` = the spec/05 badge dimension — how well the capability confirmed, `manifold` > `proxy` > `proxy_reactive`). End with `content_hash`.

5. **`## Verifying a certificate`** — `rfl verify certificate.json` + exit codes (**0** verified / **1** tampered / **2** malformed), with the three captured outputs. Then **`### Re-verifying without the rfl binary`**: state the recipe (drop `content_hash`, sort keys recursively, compact-serialize with no trailing newline, sha256, prefix `sha256:`), and give the verified jq + Python commands. Call out the trailing-newline caveat explicitly.

6. **`## What a certificate does not claim`** — the honest boundary: Class 4 (physical / high-fidelity-sim end-to-end), Class 2-loose ε (data-dependent), ENV3 disturbance-rejection, the *physical truth* of a fidelity-tier claim (only self-report honesty is checked, via `audit_honesty`), and **signer identity** — there is no signature; `content_hash` is tamper-*evidence*, not authentication. Link `[spec/05-conformance.md](../spec/05-conformance.md)` § Three-tier conformance regime for the steward/notified-body tiers.

Use `----` (four hyphens) for any section divider (never `---`). Keep relative links repo-root-relative from `docs/` (i.e. `../spec/...`, `../schemas/...`, `../examples/...`).

- [ ] **Step 2: Verify every command in the doc runs and matches**

Run each, confirm the doc's shown output:

```bash
cd ~/Documents/GitHub/rfl
cargo run -q -p rfl-cli -- certify --skill examples/01-cable-insertion/skill.yaml --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --report examples/01-cable-insertion/driver-report.jsonl ; echo "exit=$?"
cargo run -q -p rfl-cli -- verify examples/01-cable-insertion/certificate.json ; echo "exit=$?"
printf '%s' "$(jq -S -c 'del(.content_hash)' examples/01-cable-insertion/certificate.json)" | shasum -a 256
```
Expected: summary + `RESULT: PASS … sha256:4c9ff53d…`, `exit=0`; `VERIFIED … exit=0`; the shasum prints `4c9ff53d12b4acb7bd9a053200b984bb69be7a3d3d160b0fb6f213f8d6400453` (matches `content_hash` minus the `sha256:` prefix).

- [ ] **Step 3: Verify every referenced path exists**

Run: `cd ~/Documents/GitHub/rfl && for p in spec/05-conformance.md schemas/driver-interface.schema.json examples/01-cable-insertion/driver-report.jsonl examples/01-cable-insertion/certificate.json examples/01-cable-insertion/certificate-pneumatic.json; do test -f "$p" && echo "ok $p" || echo "MISSING $p"; done`
Expected: all `ok` (these are the paths the doc links / quotes, relative to repo root).

- [ ] **Step 4: Commit**

```bash
git add docs/certifying-a-driver.md
git commit -m "docs: add certifying-a-driver adoption guide

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

Then ff-push: `git fetch origin -q && git rev-list --left-right --count origin/main...HEAD` (expect `0 1`), `git push origin main`, re-fetch, expect `0 0`.

----

## Task 2: Discoverability link (conditional)

**Files:**
- Modify: `README.md` (ONLY if clean)

- [ ] **Step 1: Check README cleanliness**

Run: `cd ~/Documents/GitHub/rfl && git fetch origin -q && git status --short README.md`
- If it prints a line for `README.md` (dirty): **SKIP** Task 2 entirely; note in the milestone that the README link is deferred (another session owns its cadence).
- If empty (clean): proceed to Step 2.

- [ ] **Step 2: Add one link near the conformance / status mention**

Find a natural spot (the in-repo specification or status section) and add a single sentence linking the guide, e.g.:

```markdown
To certify your own driver against the Class 3 driver-protocol obligations, see [Certifying a driver](docs/certifying-a-driver.md).
```

Insert it as its own paragraph; do not restructure surrounding prose.

- [ ] **Step 3: Verify the link target + commit**

Run: `cd ~/Documents/GitHub/rfl && test -f docs/certifying-a-driver.md && echo ok && rg -n "certifying-a-driver" README.md`
Expected: `ok` + the new link line.

```bash
git add README.md
git commit -m "docs(readme): link the certifying-a-driver guide

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
Then ff-push (expect `0 1` → push → `0 0`).

----

## Self-Review

**Spec coverage** (design doc → tasks):
- six-section guide → Task 1 Step 1. ✓
- verified re-verification recipe (jq + Python, newline caveat) → Task 1 Step 1 §5 + captured material + Task 1 Step 2 re-confirms. ✓
- every command/output verified → Task 1 Steps 2–3. ✓
- DRY (point to schema, not restate) → Task 1 Step 1 §2. ✓
- discoverability link conditional on clean README → Task 2. ✓
- honest boundary (integrity not signer identity) → Task 1 Step 1 §6. ✓

**Placeholder scan:** the captured material section holds the real outputs; Task 1 §-by-§ specifies exact content. No TBD/TODO. The summary block uses "…" to elide the middle actions — acceptable in a plan (the full 8-line summary is captured above and in the verified command output); the doc itself shows the full summary. ✓

**Consistency:** the `content_hash` `sha256:4c9ff53d…`, the jq/Python recipe, and the exit-code triples (0/1/2) are identical across the design doc, this plan, and the captured material. The doc's relative links resolve from `docs/` (`../spec`, `../schemas`, `../examples`). ✓
