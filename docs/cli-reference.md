# `rfl` CLI reference

The `rfl` command-line interface drives the reference implementation: it
validates and retargets Skill ISA compositions, and certifies / verifies /
signs driver conformance certificates. Build it from the workspace root:

```bash
cargo build -p rfl-cli      # binary at target/debug/rfl
```

Every subcommand exits non-zero on failure, with a distinct code per failure
class (tabulated below) so the tool composes in scripts and CI.

## Synopsis

```
rfl validate <skill.yaml>
rfl retarget <skill.yaml> --embodiment <descriptor.yaml>
rfl certify  --skill <skill.yaml> --embodiment <descriptor.yaml>
             (--report <report.jsonl> | --driver <bin> [--timeout <secs>])
             [--out <certificate.json>]
rfl verify   <certificate.json>
rfl keygen   <secret-key-out>
rfl sign     --key <secret-key> <certificate.json>
rfl badge    <certificate.json>
rfl sim      --skill <skill.yaml> --embodiment <descriptor.yaml>
rfl spec-version
```

----

## `rfl validate`

Parse a Skill ISA composition and run the Class-1 composition-validity checks
(DOF-admissibility, the `no_active_grasp` lifecycle guard, the STB3
stability-class successor algebra, and GraspRef-supersession).

```bash
rfl validate examples/01-cable-insertion/skill.yaml
# VALID: skill 'cable-insertion' — 8 statement(s) (6 primitive call(s), 2 let-bind(s))
```

| Exit | Meaning |
|---|---|
| `0` | Valid composition. |
| `2` | Unreadable file, YAML/ISA parse error, or composition-validity violation. |

## `rfl retarget`

Lower a skill onto a specific embodiment and print the resolved canonical
actions as JSONL (one `execute` goal per line) on stdout. This is the exact
stream a driver consumes (and the same bytes `rfl certify --driver` writes to a
live driver's stdin).

```bash
rfl retarget examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml
# {"message":"execute","action_id":"cable-insertion/wonik-allegro-v4/0001-locate", ...}
# {"message":"execute","action_id":"cable-insertion/wonik-allegro-v4/0002-pinch", ...}
# ...
```

| Exit | Meaning |
|---|---|
| `0` | Retarget succeeded; JSONL written to stdout. |
| `1` | Any error: unreadable file, parse error, a `capability_absent` rejection, or a retarget failure (printed as `Error: …` on stderr). |

Retargeting is byte-deterministic: the same skill + descriptor always produce
the same JSONL (invariant I1).

## `rfl certify`

Certify a vendor driver against the Class-3 driver-protocol obligations and emit
a deterministic, content-hashed certificate. The driver report can be **replayed**
from a captured JSONL stream (`--report`) or produced **live** by spawning the
driver over stdio (`--driver`); the two paths run the identical pipeline and
produce byte-identical certificates for identical telemetry.

```bash
rfl certify \
  --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml \
  --report examples/01-cable-insertion/driver-report.jsonl \
  --out certificate.json
# RFL conformance certificate — cable-insertion on wonik-allegro-v4 (spec v0.1-draft)
#   [PASS] cable-insertion/wonik-allegro-v4/0001-locate (perception)
#   ...
#   [PASS] sequence:momentary_release
# RESULT: PASS (class3_driver_protocol covered, env3/class4 excluded) — sha256:4767…4531
# certificate -> certificate.json
```

Live mode is a drop-in substitution of the report source:

```bash
rfl certify --skill <s> --embodiment <e> --driver ./my-driver --timeout 30 --out certificate.json
```

| Flag | Default | Meaning |
|---|---|---|
| `--skill` | (required) | Skill ISA YAML. |
| `--embodiment` | (required) | Embodiment descriptor YAML. |
| `--report` | — | Captured driver-report JSONL (replay mode). Mutually exclusive with `--driver`. |
| `--driver` | — | Driver binary to spawn (live mode). Mutually exclusive with `--report`. |
| `--timeout` | `30` | Per-run timeout in seconds (live mode). |
| `--out` | — | Write the canonical certificate JSON to this path. |

| Exit | Meaning |
|---|---|
| `0` | Conformance **pass**; certificate emitted. |
| `1` | Conformance **fail**; the certificate is still emitted (a failing certificate is a valid artifact). |
| `2` | Invalid run: neither or both of `--report`/`--driver`, a spawn / timeout / malformed-telemetry error, or a `--out` write failure. |

## `rfl verify`

Schema-validate a certificate and re-verify its content hash (tamper
detection); when the certificate carries an ed25519 signature, also check the
signature and report the signer.

```bash
rfl verify certificate.json
# VERIFIED: content_hash sha256:4767…4531 matches (unsigned)
```

| Exit | Meaning |
|---|---|
| `0` | `VERIFIED` — content hash matches (and, if signed, the signature is valid). |
| `1` | `TAMPERED` (declared hash ≠ recomputed) or `SIGNATURE INVALID`. |
| `2` | Unreadable or malformed certificate. |

`verify` proves a certificate's **shape and integrity**, and *which* key signed
it — not whether that key is *trusted*. Trust (key governance) is out of scope.
The hash is reproducible in any language; see
[Certifying a driver](certifying-a-driver.md) for the canonicalization recipe.

## `rfl keygen`

Generate an ed25519 keypair: write the secret key (hex) to `<out>` (mode `0600`
on Unix) and print the public key to stdout.

```bash
rfl keygen signer.key
# public_key 3b1e…              (stdout)
# secret key written to signer.key   (stderr)
```

| Exit | Meaning |
|---|---|
| `0` | Keypair generated. |
| `2` | Failed to write the secret-key file. |

## `rfl sign`

Attach an ed25519 signature (over the certificate's `content_hash`) and print
the signed certificate JSON to stdout. The typed certificate is unchanged; the
signature is a `Value`-level block, so a signed certificate still verifies its
content hash.

```bash
rfl sign --key signer.key certificate.json > certificate.signed.json
rfl verify certificate.signed.json
# VERIFIED: content_hash sha256:… matches; signed by 3b1e… (ed25519)
```

| Exit | Meaning |
|---|---|
| `0` | Signed certificate written to stdout. |
| `2` | Unreadable certificate / key, or a signing error. |

## `rfl badge`

Derive a conformance **badge** from a certificate (`spec/05` § Fidelity tier and
the badge): the regime tier (who verified), the achieved fidelity tier (the
weakest confirmed action's tier, so the badge never over-claims), and the `RFL™`
trademark gate. A read-only derivation — it changes nothing the `content_hash`
covers.

```bash
rfl badge certificate.json
# RFL conformance badge — cable-insertion on wonik-allegro-v4
#   result: PASS
#   regime tier: Tier 1 (self-certification)
#   achieved fidelity: manifold
#   RFL(TM) trademark: not permitted (Tier 1 self-certification)
#   [PASS] …/0002-pinch (grasp_continuity) — fidelity manifold
#   …
```

A certificate from `rfl certify` is **Tier 1** (self-certification), so its badge
never permits the trademark (Tier 2 / Tier 3 are steward / independent
verification). The achieved fidelity rolls up to the weakest tier: a no-tactile
hand that degrades to the force/position proxy badges `proxy`, never `manifold`.

| Exit | Meaning |
|---|---|
| `0` | Badge derived and printed. |
| `2` | Unreadable or malformed certificate. |

## `rfl sim`

The **reference simulator driver** (the supply-side reference): retarget a skill
onto an embodiment, execute it with the nominal in-process reference driver, and
emit a conformant driver-report JSONL (telemetry + status) on stdout. It
*generates* the report — so it works for **any** skill, not only ones with a
committed recording.

```bash
rfl sim --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml > report.jsonl
rfl certify --skill examples/01-cable-insertion/skill.yaml \
  --embodiment examples/01-cable-insertion/embodiments/allegro.yaml --report report.jsonl
# RESULT: PASS …
```

| Exit | Meaning |
|---|---|
| `0` | Report emitted to stdout. |
| `1` | Unreadable file, parse error, or a `capability_absent` retarget rejection. |

The emitted report is what a conformant driver would return, so piping it into
`rfl certify --report` passes. (For *live* third-party certification, point
`rfl certify --driver` at a real driver binary instead.)

## `rfl spec-version`

Print the specification version this build implements.

```bash
rfl spec-version
# v0.1-draft
```

Always exits `0`.

----

## End-to-end

A complete hardware-free loop (retarget → replay-certify → verify) is in
[getting-started.md](getting-started.md); the third-party driver on-ramp
(capture → certify → read → verify → boundary) is in
[certifying-a-driver.md](certifying-a-driver.md).
