# Packaging and versioning

How the `rfl` crates and binary are versioned and published. This is *prep*
documentation: nothing is published to crates.io yet (the reference
implementation milestone is 2027 Q2–Q3); this records the policy and the
verified-working publish path so the eventual release is mechanical.

## Versioning policy

- **Single workspace version.** All crates share one version via
  `[workspace.package] version` in the root `Cargo.toml`
  (`rfl-core`, `rfl-conformance`, `rfl-cli` move in lockstep). Current: `0.0.1`
  (pre-release).
- **Semantic versioning, gated by the spec.** While the spec is `v0.1-draft`
  (`rfl spec-version`), crate versions stay `0.x` and may break between minors.
  The crate `1.0.0` line is tied to **spec v1.0** (2027 Q2–Q3): from there,
  Principle 5 (forward-compatible) binds — no breaking change in a `1.x` release,
  new capability enters the extension registry (`spec/06`) instead.
- **MSRV.** `rust-version = "1.86"` (edition 2024), declared once in
  `[workspace.package]` and inherited by every crate, and enforced by the `msrv`
  CI job (`cargo check` on the pinned toolchain). The floor is set by transitive
  deps (the `icu_*` crates via `boon` → `url` → `idna` require 1.86), not by the
  RFL code itself.

## Crate graph and publish order

```
rfl-core  ◄── rfl-conformance  ◄── rfl-cli   (binary `rfl`)
```

Internal dependencies are declared in `[workspace.dependencies]` with **both** a
`path` (for local builds) and a `version` (required by crates.io):

```toml
rfl-core = { path = "crates/rfl-core", version = "0.0.1" }
rfl-conformance = { path = "crates/rfl-conformance", version = "0.0.1" }
```

crates.io rejects a publish whose dependency "does not specify a version
requirement", so the `version` is mandatory; the `path` is stripped on publish
and the published crate resolves the dependency from crates.io. Publishing is
therefore **bottom-up**, each crate live before the next depends on it:

```bash
cargo publish -p rfl-core
cargo publish -p rfl-conformance   # after rfl-core is live on crates.io
cargo publish -p rfl-cli           # after both are live
```

## Verifying before a release

```bash
cargo publish -p rfl-core --dry-run        # Packaged 16 files, ~315 KiB (verified clean)
```

A dependent crate's `--dry-run` only fully succeeds once its dependencies are
**actually on crates.io** (the dry-run's verify step resolves them from the
index), so a pre-publish dry-run of `rfl-conformance` / `rfl-cli` will report the
unpublished `rfl-core` until the bottom-up publish reaches them. That is expected,
not a packaging defect.

The `bindings/python` crate is **excluded** from the workspace (built only via
maturin) and is not part of the cargo publish flow.

## Binary releases

The `rfl` binary (the `rfl-cli` crate) is the user-facing artifact. The release
profile (`[profile.release]` in the root manifest) is `lto = "thin"`,
`codegen-units = 1`, `strip = true` for a small, fast binary. A tagged GitHub
release attaching prebuilt `rfl` binaries per platform is the distribution
channel for non-cargo users; cargo users `cargo install rfl-cli` once published.

## docs.rs

Each crate carries `[package.metadata.docs.rs] all-features = true`; `cargo doc`
is warning-clean under `RUSTDOCFLAGS=-D warnings` (verified), so docs.rs renders
the API reference automatically on publish.
