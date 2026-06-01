# Certifying a driver

This guide shows how to self-certify a driver implementation against the RFL **Class 3
driver-protocol** obligations and how anyone can re-verify the resulting certificate. The normative
definitions live in [spec/05-conformance.md](../spec/05-conformance.md); this is the practical
on-ramp.

The flow is three steps:

1. **Capture** — run your driver and record what it reports as a JSONL stream.
2. **Certify** — `rfl certify` checks that stream against the obligations and emits a certificate.
3. **Verify** — `rfl verify` (or anyone, in any language) re-checks the certificate's integrity.

You do not hand the tool your goals — it derives the canonical actions itself by retargeting the
skill onto the embodiment, so you cannot certify against a forged contract. You only supply what
your driver actually did.

----

## The driver-report wire format

Your driver runs the skill and emits **one JSON object per line** (JSONL). Each line is either a
`telemetry` or a `status` message (discriminated by the `message` field), correlated by
`action_id`. Every action must have **exactly one terminal `status`**; it may have any number of
`telemetry` samples before it.

The normative contract — every field, type, and pattern — is
[schemas/driver-interface.schema.json](../schemas/driver-interface.schema.json). Validate your
output against it; this guide does not restate it.

Two real lines from the worked example
([examples/01-cable-insertion/driver-report.jsonl](../examples/01-cable-insertion/driver-report.jsonl)):

```jsonl
{"message":"telemetry","action_id":"cable-insertion/wonik-allegro-v4/0001-locate","t":1.0,"realized_pose":{"position":[0.0,0.0,0.0],"orientation":[0.0,0.0,0.0,1.0]}}
{"message":"status","action_id":"cable-insertion/wonik-allegro-v4/0002-pinch","outcome":"succeeded","verdict":{"value":true,"confidence":1.0,"evidence":["nominal reference-driver execution"]},"fidelity_tier":"manifold","final_pose":{"position":[0.0,0.0,0.0],"orientation":[0.0,0.0,0.0,1.0]}}
```

- A **`telemetry`** line is a sample on the action's timebase. Only `message`, `action_id`, and `t`
  are mandatory; the rest are present as the primitive needs them (a pose, a `wrench`, a
  `securing_force`, force `events`, a `fidelity_tier`, …).
- A **`status`** line is the action's terminal result: its `outcome`
  (`succeeded` / `failed` / `indeterminate`), the `verdict` with its evidence, and — when the
  action confirmed through a sensing path — the `fidelity_tier` it achieved.

The `action_id` is `{skill}/{embodiment_id}/{NNNN}-{suffix}`. You do not invent it: it is the id of
the canonical action the tool generates, and it is how your report is matched back to the action it
executed.

----

## Running `rfl certify`

```bash
rfl certify \
  --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml \
  --report examples/01-cable-insertion/driver-report.jsonl \
  --out certificate.json
```

`--out` is optional; without it the certificate is computed and summarized but not written. The
summary on the worked example:

```text
RFL conformance certificate — cable-insertion on wonik-allegro-v4 (spec v0.1-draft)
  [PASS] cable-insertion/wonik-allegro-v4/0001-locate (perception)
  [PASS] cable-insertion/wonik-allegro-v4/0002-pinch (grasp_continuity)
  [PASS] cable-insertion/wonik-allegro-v4/0003-transport (grasp_continuity)
  [PASS] cable-insertion/wonik-allegro-v4/0004-locate (perception)
  [PASS] cable-insertion/wonik-allegro-v4/0005-align (terminal_postcondition)
  [PASS] cable-insertion/wonik-allegro-v4/0006-insert_fit (force_trajectory)
  [PASS] cable-insertion/wonik-allegro-v4/0007-release (grasp_continuity)
  [PASS] cable-insertion/wonik-allegro-v4/0008-retract (terminal_postcondition)
  [PASS] sequence:momentary_release
RESULT: PASS (class3_driver_protocol covered, env3/class4 excluded) — sha256:4c9ff53d…
```

The exit code carries the run's meaning:

| Exit | Meaning |
|---|---|
| `0` | Valid run, every action conforms. |
| `1` | Valid run, at least one non-conformance. A certificate is still emitted, recording exactly what failed. |
| `2` | The run is **invalid** — unparseable skill / embodiment, a malformed or schema-invalid report line, or broken correlation (a missing or orphan action). No certificate; an error goes to stderr. |

The distinction between `1` and `2` matters: `1` means "your driver is non-conformant, here is the
proof"; `2` means "your report could not be read at all."

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
driver's **stdin** (one JSON object per line, then EOF), and reads the `telemetry` + `status` lines
your driver writes to its **stdout**. Your driver should read goals from stdin, execute them, emit
its report on stdout, and exit `0`.

`--report` and `--driver` are mutually exclusive; supply exactly one. `--timeout <secs>` (default
`30`) bounds how long the driver may run; a driver that exits non-zero, overruns the timeout, or
emits a malformed report is an invalid run (exit `2`).

----

## Reading the certificate

A certificate identifies its inputs by **RFL id + content sha256** (never a filesystem path), so it
is machine-independent:

```json
{
  "skill": { "id": "cable-insertion", "sha256": "27e609a0…" },
  "embodiment": { "id": "wonik-allegro-v4", "sha256": "c67f1bd1…" },
  "report_sha256": "fcf9e82e…",
  "result": "pass",
  "covered": ["class3_driver_protocol"],
  "excluded": ["class4_physical", "class2_loose_epsilon", "env3_disturbance", "fidelity_tier_physical_truth"],
  "content_hash": "sha256:4c9ff53d…"
}
```

`covered` is what the certificate **claims**; `excluded` is what it explicitly **disclaims** (see
[What a certificate does not claim](#what-a-certificate-does-not-claim)).

Each action carries its envelope class, the obligations that ran, and the fidelity tier it achieved:

```json
{
  "action_id": "cable-insertion/wonik-allegro-v4/0002-pinch",
  "suffix": "pinch",
  "envelope_class": "grasp_continuity",
  "fidelity_tier": "manifold",
  "checks": [
    { "name": "envelope", "result": "pass" },
    { "name": "actuation", "result": "pass" },
    { "name": "engagement", "result": "pass" },
    { "name": "irreversible", "result": "pass" },
    { "name": "freed_part_disposition", "result": "pass" },
    { "name": "audit_honesty", "result": "pass" }
  ],
  "passed": true
}
```

`fidelity_tier` is the [spec/05](../spec/05-conformance.md) **badge** dimension — *how well* the
capability confirmed: `manifold` > `proxy` > `proxy_reactive`. The same skill on an embodiment with
no tactile sensing lowers to `proxy`, and the certificate records it honestly — for example, on
`generic-pneumatic-6f` the same `0002-pinch` action passes with `"fidelity_tier": "proxy"`
([certificate-pneumatic.json](../examples/01-cable-insertion/certificate-pneumatic.json)). A `proxy`
result is still conformant; it just confirms with less fidelity than `manifold`.

A failing obligation appears as `{ "name": "...", "result": "fail", "reason": "..." }` and sets the
action's `passed` to `false` and the certificate's `result` to `fail`.

The `content_hash` is sha256 over the certificate body in a canonical form (next section).

----

## Verifying a certificate

```bash
rfl verify certificate.json
```

```text
VERIFIED: content_hash sha256:4c9ff53d… matches
```

| Exit | Meaning |
|---|---|
| `0` | Schema-valid and the content hash matches — a well-formed, unmodified RFL certificate. |
| `1` | The content hash does not match: the certificate was altered after sealing (`TAMPERED: declared … != recomputed …`). |
| `2` | Malformed — unreadable, unparseable, or schema-invalid; no verdict is possible. |

`rfl verify` proves a certificate's *shape* and *integrity*. It does **not** prove who issued it
(see below).

### Re-verifying without the `rfl` binary

The `content_hash` is sha256 over the certificate body with `content_hash` **and `signature`**
removed, every object's keys sorted recursively, compact-serialized with **no trailing newline**,
prefixed `sha256:`. (An unsigned certificate has no `signature` key, so dropping it is a harmless
no-op; a signed certificate's `content_hash` is over the body excluding both fields.) Any JSON
library reproduces it. The result must equal the certificate's declared `content_hash`.

```bash
# jq — the printf strips the trailing newline jq -c would otherwise add (which would change the hash)
printf '%s' "$(jq -S -c 'del(.content_hash, .signature)' certificate.json)" | shasum -a 256
```

```python
import json, hashlib
d = json.load(open("certificate.json")); d.pop("content_hash", None); d.pop("signature", None)
print("sha256:" + hashlib.sha256(json.dumps(d, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
```

Both print `4c9ff53d12b4acb7bd9a053200b984bb69be7a3d3d160b0fb6f213f8d6400453` for the worked-example
certificate — equal to its `content_hash` minus the `sha256:` prefix. The trailing-newline caveat is
real: the bare `jq -S -c … | shasum` form hashes a different byte string and will not match.

----

## Signing a certificate (optional)

A self-certified certificate carries integrity but no signer. To attach a signer identity, sign it
with an ed25519 key:

```bash
rfl keygen signer.key                 # writes the secret key to signer.key, prints the public key
rfl sign --key signer.key certificate.json > certificate-signed.json
```

`rfl sign` adds a top-level `signature` object — `{ "alg": "ed25519", "public_key": "<hex>",
"sig": "<hex>" }` — over the certificate's `content_hash`. `rfl verify` then reports the signer:

```text
VERIFIED: content_hash sha256:… matches; signed by <public_key> (ed25519)
```

`rfl verify` confirms the signature is **valid** and reports **which key** signed. It does **not**
decide whether that key is **trusted** — mapping a public key to an authorized steward is a
governance question (the Tier-2 / Tier-3 regime in
[spec/05-conformance.md](../spec/05-conformance.md)), out of scope for the tool.

----

## What a certificate does not claim

A Class 3 certificate covers the driver protocol on a nominal run. It explicitly does **not** cover:

- **Class 4** — physical or high-fidelity-simulator end-to-end execution.
- **Class 2-loose ε** — the per-skill realized-execution tolerance (still data-dependent).
- **ENV3 disturbance-rejection** — graceful degradation under injected disturbances needs an active
  bench, which a nominal replay cannot exercise.
- **The physical *truth* of a fidelity-tier claim** — `audit_honesty` checks only that the reported
  tier is not over-claimed relative to the lowering decision, not that the tier is physically
  achieved.
- **Signer *trust*** — `rfl sign` / `rfl verify` (above) attest *which key* signed a certificate,
  but the tool does not establish whether that key is *authorized*. Mapping a public key to a
  trusted steward — the Tier-2 / Tier-3 regime in
  [spec/05-conformance.md](../spec/05-conformance.md) § Three-tier conformance regime — is a
  governance question, out of scope for the tool. An unsigned `rfl certify` certificate is Tier-1
  self-certification.
