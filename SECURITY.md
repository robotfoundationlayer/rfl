# Security Policy

## Supported versions

RFL is **pre-release** (no tagged release yet; see [CHANGELOG.md](CHANGELOG.md)).
Until the v0.1 community-review release, security fixes land on `main`. Once
releases are tagged, this table will record which lines receive security
updates.

| Version | Supported |
|---|---|
| `main` (pre-release) | ✅ |

## Reporting a vulnerability

**Please do not open a public Issue for a security vulnerability** in the
specification, the reference implementation (`crates/`), the conformance suite,
or the schemas.

Two private channels:

- **GitHub** — use *Security → Report a vulnerability* (private advisory) on the
  [repository](https://github.com/robotfoundationlayer/rfl), or
- **Email** — <yo@vox.delivery> with the subject prefix `[security]`.

We will acknowledge within **72 hours** and work with you on a coordinated
disclosure: a fix and an advisory, credited to you unless you prefer otherwise.

## Scope

In scope: anything that lets a conformance result, certificate, or signature be
forged or bypassed (e.g. a certificate that verifies despite tampering, a
retarget that silently violates an envelope contract), and vulnerabilities in
the reference implementation's handling of untrusted input (a driver report, a
skill / descriptor file, a certificate).

Out of scope: the security of a *downstream* robot deployment built on RFL (RFL
is a specification and reference implementation, not a deployed control system),
and issues in unmaintained transitive dependencies for which no fix exists
(tracked in [`deny.toml`](deny.toml)).

## Dependency hygiene

Dependency licenses and security advisories are gated in CI by
[`cargo-deny`](deny.toml), and dependency updates are automated via Dependabot
(`.github/dependabot.yml`).
