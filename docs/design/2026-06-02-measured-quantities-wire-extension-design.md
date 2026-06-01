# `measured_quantities` — a generic domain-quantity wire extension

Date: 2026-06-02
Status: approved (design) — follows
`2026-06-02-measure-quantity-reconciliation-design.md`

## Problem

The ε-table's category-C domain quantities (`seating_depth`,
`completion_torque`, `turns`, `actuation_force`, `engagement_force`,
`cut_depth`, `oscillation_amplitude`, `contact_force`) are not first-class
driver-report wire fields, so `rfl measure` marks them `null` +
`not_wire_derivable`. They cannot be reliably scraped from existing channels:
`spec/04` defines `actuation_force` / `engagement_force` as a detent
`ForceEvent.magnitude`, but a `ForceEvent`'s magnitude is optional and the
reference driver emits detent events kind-only; `completion_torque` /
`contact_force` would force the measure tool to *guess the semantic instant*
(the torque at completion, not whatever the last sample holds); and
`seating_depth` / `cut_depth` / `turns` / `oscillation_amplitude` have no wire
field at all. All eight share the `abs` (scalar) metric. The driver — which
alone knows each primitive's semantics — is the honest source.

## Decision (approved)

Add an optional, opt-in **`measured_quantities`** map to telemetry, exactly
analogous to the `contact_geometry` extension: a driver reports the domain
scalars it measured, keyed by the committed-table quantity name. `rfl measure`
reads category-C quantities from this channel.

## Wire

`rfl-core::driver::Telemetry` gains:

```rust
/// Primitive-specific domain quantities the driver measured at this sample,
/// keyed by the ε-table quantity name (spec/05 § Class 2-loose). The driver is
/// the authoritative source — it alone knows the semantic instant
/// (completion, seating, the actuation peak). Each value is a scalar Quantity.
#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
pub measured_quantities: BTreeMap<String, Quantity>,
```

`skip_serializing_if` empty keeps every existing telemetry's serialization
byte-identical (no golden churn). `schemas/driver-interface.schema.json`
TelemetryFeedback gains an optional `measured_quantities` object whose values
are quantity strings (the schema's telemetry block is `additionalProperties:
false`, so the field must be declared). `replay.rs`'s `TelemetryIn` mirror gains
`#[serde(default)] measured_quantities: BTreeMap<String, String>`, carried into
`Telemetry` via `.map(Quantity)` like `securing_force`.

## Measure tool

`wire_extractor(quantity) -> Option<fn>` is replaced by
`extract_quantity(quantity: &str, report) -> Option<Repr>`, whose default branch
reads `measured_quantities[quantity]` as a scalar:

```rust
fn extract_quantity(quantity: &str, r: &DriverReport) -> Option<Repr> {
    match quantity {
        "realized_position"    => …final_pose.position,
        "realized_orientation" | "final_orientation" => …final_pose.orientation,
        "realized_wrench"      => …last wrench.force,
        "securing_force"       => …last telemetry securing_force,
        other => last telemetry measured_quantities[other] as Repr::Scalar,
    }
}
```

Consequence: **every committed quantity is now wire-reportable**, so the
`not_wire_derivable` reason is removed. An absent quantity is uniformly
`not_reported` (the channel exists; no run filled it). `aggregate_epsilon` drops
the `wire_derivable` flag; `reason = if samples.is_empty() { Some("not_reported") }
else { None }`.

## What this is not

- It does not make the *reference driver* emit domain quantities (the reference
  driver is deterministic → ε = 0 anyway). Emitting them is a vendor-driver
  concern; the reference driver stays minimal. Tests construct synthetic traces
  carrying `measured_quantities`.
- It does not change the committed `epsilon-tolerances.yaml` (still all `null`).
- It does not grade time-series quantities specially: `oscillation_amplitude`
  is reported by the driver as a single scalar (its measured amplitude), like
  every other category-C quantity — no trajectory analysis in the tool.

## Testing (TDD)

- `rfl-core`: a telemetry with no `measured_quantities` serializes identically
  (golden-safe); one with entries serializes them and round-trips through
  `replay`.
- `measure`: a `force.insert_fit` trace reporting `seating_depth` via
  `measured_quantities` over two perturbed runs → `seating_depth` filled
  (non-null, abs deviation); absent → `not_reported` (never `not_wire_derivable`,
  which no longer exists).
- CLI integration: the committed cable trace (no `measured_quantities`) →
  `seating_depth` `null` + `not_reported`; committed table byte-unchanged.
- Schema: the committed fixtures still validate; a fixture carrying
  `measured_quantities` validates.
