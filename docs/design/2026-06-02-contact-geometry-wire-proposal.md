# PROPOSAL: a contact-geometry wire extension to unblock STB1 / STB2 / supported()

Status: **APPROVED + SHIPPED 2026-06-02** (spec owner: "use defaults"). Landed:
the optional `contact_geometry` telemetry field (`schemas/driver-interface` +
`rfl-core::driver::ContactGeometry` + replay parsing), the `check_contact_geometry`
battery obligation (STB1 tripod non-collinearity + STB2 / `supported()`
CoM-over-polygon, reading `rfl-core::geometry`), and a unit test (spread vs
near-collinear tripod; CoM inside vs outside). Vacuous for any report that omits
`contact_geometry`, so it is opt-in (existing reference certs re-blessed only to
record the new vacuous-pass obligation). **Still deferred (data-dependent, like the
ε-values):** the absolute `min_contact_area` threshold (a fraction of object
cross-section) and the STB2 stability *margin* — v0 enforces non-degeneracy
(non-collinear / inside-polygon). The original proposal follows.

## Why this is a proposal, not an increment

Three deferred conformance checks share one root blocker: the wire reports **no
contact-point geometry**, so a check that needs the contact polygon cannot be
computed. Adding that geometry is a *normative wire-format* change (it extends
`spec/03` telemetry + `spec/04`), so it is the spec owner's decision, not an
autonomous impl. This doc proposes the minimal extension and the checks it
unblocks, so the decision is concrete.

## The three blocked checks and what each needs

- **STB1 — non-degenerate tripod** (`spec/05` § 125, 146): a `grasp.precision_tripod`
  is confirmed only when its three contacts form a **non-collinear triangle above
  a minimum-area threshold** (relative to the object cross-section). Needs the
  **three contact-site positions**.
- **STB2 — support safe-state** (already checked at the *sequence* level) and the
  `grasp.platform` claim: a platform/support grasp balances the object's **CoM
  over the contact polygon** by a margin. The numeric geometric test needs the
  **contact-polygon vertices + the object CoM**.
- **`grasp.release` `supported()`** (`spec/01` line 348): release is gated on the
  object's CoM projecting strictly inside its contact polygon. Same geometry.

All three are the same primitive: **CoM-over-contact-polygon**, with STB1 the
degenerate area-only case.

## Proposed minimal wire extension

Add an optional `contact_geometry` block to the **telemetry** message
(`spec/03` § Canonical driver messages; `schemas/driver-interface.schema.json`),
populated for a grasp/contact that claims a polygon-stability property:

```yaml
contact_geometry:
  sites:                       # the contact-site positions (>= 3 for a polygon)
    - [x, y, z]                # in the telemetry frame, metres
    - [x, y, z]
    - [x, y, z]
  object_com: [x, y, z]        # the supported object's centre of mass (optional;
                               # required for the CoM-over-polygon checks)
```

And one descriptor limit (`spec/03` § Limits;
`schemas/embodiment-descriptor.schema.json`):

```yaml
limits:
  min_contact_area_fraction: 0.15   # STB1 threshold, as a fraction of the
                                    # object cross-section (or an absolute m^2)
```

Both are additive and optional — no existing descriptor, skill, or golden moves
until a primitive opts in. This is the same "structure first, opt-in" shape as
the already-shipped grasp-stability metadata.

## The checks, once the wire exists (ready to implement)

- **STB1**: from `sites` (exactly 3 for a tripod), compute the triangle area
  `A = ½|(p₂−p₁)×(p₃−p₁)|`; confirm `A ≥ min_contact_area_fraction · object_cross_section`
  and non-collinearity (`A > ε`). A near-collinear triple fails.
- **STB2 / platform**: project `object_com` onto the contact-polygon plane;
  confirm it lies strictly inside the polygon (point-in-polygon) by the stability
  margin.
- **`grasp.release` `supported()`**: the same point-in-polygon test, gating the
  release; with `require_stable` set, a release that fails it is rejected.

Each is a pure-geometry function (no new dependency — `nalgebra` is already a
dep), unit-testable with synthetic polygons, then wired to a telemetry-bearing
conformance bench with an adversarial driver (a near-collinear tripod; a
CoM-outside-polygon release).

## Open decision points (for the spec owner)

1. **Frame** of the contact sites — telemetry frame vs the grasp frame. (Proposed:
   telemetry frame, consistent with the realized pose.)
2. **`min_contact_area`** as a fraction of object cross-section vs an absolute
   area. (Proposed: fraction, so it scales with object size.)
3. Whether `object_com` belongs in telemetry (driver-measured) or is carried from
   the skill's `ObjectTarget.center_of_mass` (authored). (Proposed: prefer the
   authored value, fall back to telemetry.)
4. **Site ordering** — are `sites` reported in polygon-boundary order, or as an
   unordered set the check must convex-hull? (Proposed: boundary order, the
   simpler driver contract; the implemented `com_over_polygon` assumes it. An
   unordered contract would add a 2D-hull step.)

On approval, STB1 + the platform CoM check + `grasp.release` `supported()` land as
one geometry increment (the area / point-in-polygon math + the
`contact_geometry` schema fields + an adversarial bench). Without approval they
stay correctly blocked — this doc converts "blocked, no path" into "blocked,
decision pending."
