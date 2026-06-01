# Translation Layer — Specification

> **Status**: design-complete (2026-05-31) — the canonical action representation + `Envelope`, the quaternion pose representation with single-scalar geodesic orientation error, the under-constrained-orientation residual rule, rest-at-goal terminal semantics, the determinism boundary (Class 2-strict generation / Class 2-loose contact-dynamics execution), multi-embodiment coordination, capability negotiation, the grasp-force / stability derivations, and trajectory generation + time-scaling are specified. The `02` group of § Open issues is closed (12/12) — and with it the entire cross-chapter Open-issues TODO (`01` / `03` / `04` / `05` / `02` all resolved). The normative `Σ`-generator construction is fixed in § Appendix A. Remaining before freeze: the JSON Schema.

## Scope

This chapter defines:

1. The canonical embodiment-agnostic action representation
2. The embodiment descriptor schema (`schemas/embodiment-descriptor.schema.json`)
3. The deterministic retargeting algorithm `retarget(skill, embodiment) -> canonical_actions`
4. The safety envelope semantics and how envelope constraints flow from Skill ISA into per-embodiment action parameters

## Relation to the whitepaper's I1–I5 contract

The whitepaper states the retargeting contract as five binding invariants (I1–I5). This chapter is their machine-checkable form: it mechanizes the same contract as the finer obligations `CA1c–CA4c` (canonical-action) and `RD1c–RD4c` (retarget-determinism), with two invariants whose home is an adjacent chapter. The correspondence (derived; I2–I4 each span more than one obligation, so treat this table as the traceability map, not a one-to-one rename):

| Whitepaper invariant | Mechanized by |
|---|---|
| **I1 — Determinism** | `RD1c` (generation byte-determinism, unconditional) + `RD2c` (the Class 2-loose realized-execution boundary for contact dynamics) |
| **I2 — Embodiment-respect** | `CA1c` (pose reachability decidable) + `CA4c` (envelope clamped to the embodiment's declared limits) + the `03` `capability_absent` gate |
| **I3 — Composability (rest-stable)** | the `01` compositional algebra's rest-stable composition validity — not a `02` obligation; recorded here so the invariant is traceable |
| **I4 — Failure-mode preservation** | `CA4c` (an action exceeding a declared limit is malformed, never silently relaxed) + the binary `capability_absent` gate (an explicit refusal rather than a degraded sequence). `RD4c` uncertainty-robust routing is the embodiment-respect complement (I2), not a failure-mode obligation |
| **I5 — Extension-namespace isolation** | the `06` extension registry pass-through + unknown-tag rejection (`03`) |

I1–I5 is the authoritative public statement of the contract; `CA*c` / `RD*c` are the same contract at the granularity a conformance test reads. The two label systems are not competing claims.

## Canonical action representation

A canonical action is the embodiment-agnostic instruction `retarget` emits — the one representation a conformant driver (`03`) consumes. It is a tuple:

```
CanonicalAction := (
    target_frame:    FrameRef,                  # the controlled frame this action moves (01 frame model)
    target_pose:     Pose6D,                    # goal pose of target_frame (§ Pose representation)
    force_budget:    Option<Force>,             # driven-force ceiling (force category); None for pure motion
    timing:          TimingHints,               # nominal duration + timing mode (§ Terminal semantics; Unit 4 time-scaling)
    tactile_target:  Option<TactileTarget>,     # contact-confirmation criterion (04); None when no contact intended
    monitors:        set<Monitor>,              # StopConditions + feature events the action watches (01 / 04)
    safety_envelope: Envelope,                  # the safety constraints in force throughout (below)
)
```

The field *types* are owned elsewhere — `Pose6D` / `TactileTarget` and the `StopCondition` (`01`) / `ForceEvent` (`04`) a `Monitor` wraps — so this chapter fixes how a skill's parameters are *compiled into* the tuple and how the tuple is made deterministic, not the types themselves. `monitors` is the canonical-action home of the feature-monitoring seam every other chapter left to `02`: a `StopCondition` or a `ForceEvent` the action watches is emitted here as a `Monitor`, with its threshold / signature evaluated per the `01` / `04` definitions.

### The `Envelope`

The `safety_envelope` carries the safety constraints in force for the action — the per-embodiment instantiation of the primitive's Skill ISA safety envelope (`01`), with every bound clamped to the embodiment's declared limits (`03`):

```
Envelope := {
  motion_bounds:  { v_max, a_max, w_max, … },   # kinematic caps (reach / transport), clamped to embodiment.limits.*
  force_profile:  Option<ForceProfile>,          # the force / torque trajectory bound (force category); interval-checked by 05 ENV4
  clearance:      Length,                          # min clearance to the collision model, with the exempt / augment sets (03)
  compliance:     Option<Compliance>,              # the requested compliance mode + direction (03)
  stop_time:      Duration,                        # max time to reach a safe state on a breach (embodiment.limits.stop_time)
}
```

The envelope is the data the `05` envelope classes (`05` § The envelope-class taxonomy) sample against: `motion_bounds` for the interval-invariant class, `force_profile` for the force / torque-trajectory class. How each bound is *derived* — `min_holding_force`, the dynamic-stability `a_max` clamp, the reaction-load limit — is § Grasp-force and stability derivations (later unit).

## Pose representation and orientation error

`Pose6D` is a rigid-body pose: a **position** in R³ (metres) plus an **orientation** as a **unit quaternion**. This is the ROS 2-aligned representation (`geometry_msgs/Pose` parity, consistent with the ROS 2-compatible driver protocol of `03`); SE(3) is the group beneath it (used for composition and frame chaining), and the unit quaternion is the canonical serialization. The choice is provider-neutral (Principle 4): no embodiment's orientation convention is privileged.

**The single-scalar geodesic orientation error.** The representation is chosen so that orientation error reduces to one scalar — the requirement that makes `pose_not_reached` decidable. For target orientation `q_t` and current `q_c` (unit quaternions):

```
θ_orient = 2 · arccos( |⟨q_t, q_c⟩| )      # geodesic angle on SO(3), in [0, π]
```

is the geodesic rotation between them; the `|·|` resolves the quaternion double-cover so `q` and `−q` denote one orientation. A pose is *reached* iff position error ≤ the primitive's `position_tolerance` **and** `θ_orient` ≤ its `orientation_tolerance`; `pose_not_reached` is the negation. The single scalar `θ_orient` is what every primitive's terminal postcondition and `orientation_tolerance` (`01`) compare against.

**Under-constrained-orientation residual rule** (decided). When a primitive constrains fewer than three orientation axes — `reach.align` with a subset of `axes`, `place.orient` with one `OrientationSpec` constraint — the remaining orientation freedom is resolved by the **minimum geodesic rotation from the current orientation**: among all orientations satisfying the declared constraints, `retarget` selects the one nearest (smallest `θ_orient`) to the frame's current orientation. The rule applies **uniformly** wherever orientation is under-constrained, which is what makes the resolution deterministic (binding for `retarget` determinism, `05` Class 2) rather than an arbitrary pick from the satisfying set.

## Terminal semantics — rest-at-goal

v0.1 guarantees **rest-at-goal**: every motion primitive terminates at rest (zero commanded velocity) at its goal pose. The terminal-postcondition envelope class (`05`) and every `reach` primitive's "terminating at rest" postcondition (`01`) depend on it. To leave room for non-stop **trajectory blending** in a future version without breaking the guarantee (Principle 5), the canonical action's `timing` reserves a `stop_at_goal: bool` (default `true`): v0.1 fixes it `true`; a later version may set it `false` to blend one primitive's goal into the next's start without resting, and the change is additive — a v1.x driver implementing only `stop_at_goal = true` stays conformant.

### Conformance obligations (canonical action)

- **CA1c — pose reachability decidable.** `pose_not_reached` is decided from the position error and the single-scalar geodesic orientation error `θ_orient = 2·arccos|⟨q_t, q_c⟩|` against the primitive's `position_tolerance` / `orientation_tolerance`.
- **CA2c — under-constrained residual determinism.** Where orientation is under-constrained, `retarget` resolves the residual as the minimum geodesic rotation from the current orientation, uniformly and deterministically.
- **CA3c — rest-at-goal.** Every motion primitive terminates at rest at its goal in v0.1 (`stop_at_goal = true`); the reserved `stop_at_goal` field admits future non-stop blending additively without breaking the guarantee.
- **CA4c — envelope clamping.** Every `Envelope` bound is the primitive's Skill ISA safety-envelope constraint clamped to the embodiment's declared `embodiment.limits.*`; an action whose bound exceeds a declared limit is malformed.

### Deferred and referenced

- **`Pose6D`, `TactileTarget`, `StopCondition`, the per-primitive tolerances and safety envelopes** the tuple and `Envelope` carry — `01-skill-isa.md` / `04-tactile-manifold.md`.
- **The embodiment limits** every `Envelope` bound clamps to — `03-driver-interface.md` § Capability manifest.
- **The derivations** of the force / acceleration bounds (`min_holding_force`, dynamic-stability `a_max`, reaction-load) — § Grasp-force and stability derivations (later unit).
- **`TimingHints` and time-scaling** — § Trajectory generation and timing (later unit).

## Retargeting algorithm — determinism requirement

`retarget(skill, embodiment)` must be **deterministic**: identical inputs produce identical canonical-action sequences, byte-for-byte. The determinism requirement is binding because conformance testing (`05-conformance.md`) compares observed retargeting output against frozen fixtures. Determinism applies to the **generation** of the canonical action; whether the *realized execution* of that action is reproducible is a separate question, bounded next.

## The determinism boundary

`retarget` generation is byte-deterministic unconditionally. The **realized execution** of an action is not always reproducible, and the boundary is where conformance splits (`05` § The determinism floor):

- **Kinematic primitives** — `reach`, and any action executed by position / velocity control against free space — have a reproducible realized execution: Class 2-strict (bit-identical). `reach.scan`'s sweep set `Σ` is the archetype — a purely-kinematic function of its inputs (§ Trajectory generation and timing).
- **Contact-dynamics primitives** — every `force` primitive (compliant search executes against contact dynamics) and the **passive-drive** modes `in_hand.pivot(drive = gravity | external)` (the outcome depends on embodiment dynamics, not just the canonical action) — have a realized execution that is *not* byte-reproducible. Their canonical-action **generation** is still Class 2-strict; only the **realized-execution** check is Class 2-loose (semantic equivalence within a per-skill ε), with the tolerance loosened accordingly — parallel to `reach.scan`'s purely-kinematic contract as the strict archetype.

Making the boundary explicit is what keeps a passive or compliant primitive from being failed for not reproducing a contact-dynamics trajectory it was never deterministic in.

## Multi-embodiment coordination

Single-embodiment `retarget` is byte-deterministic (above). `transport.handoff` between **two** parties exceeds that boundary:

- **Bimanual handoff** (within one embodiment) closes inside one `retarget` run — both effectors are planned together, so it is fully deterministic and fully specified in-spec (the two-party continuity GC6 in `05`, the `EffectorRef` in `03`).
- **Inter-robot handoff** coordinates **two** `retarget` runs, one per embodiment, exceeding single-embodiment byte determinism. v0.1 specifies the handoff **semantics** (the GC6 two-party continuity invariant and the `EffectorRef` addressing) and the **two-party determinism boundary** (each run is deterministic, but the cross-run interleaving is not held to single-embodiment byte reproducibility); it **defers** the inter-robot coordination *protocol* — the wire mechanism two embodiments use to synchronize the dual-grasp window — as a named v0.1 deferral. Bimanual is the fully-specified case; inter-robot is semantics-now, mechanism-later.

## Capability negotiation and routing

`retarget` selects a capable embodiment for a task. Beyond the binary presence / absence gate (`03`'s `capability_absent`), negotiation matches a task's **geometry uncertainty** against an embodiment's **declared robustness**: an uncertain-geometry task is routed to an **envelope-capable** embodiment, using the robustness `grasp.envelope` declares (a compliant / caging grasp tolerates pose uncertainty a precision pinch cannot). Uncertainty-robust routing is the retarget-side complement to `03`'s static gate — the gate decides *can this embodiment do it at all*, negotiation decides *which capable embodiment is robust enough for this task's uncertainty*.

### Conformance obligations (retargeting determinism)

- **RD1c — generation determinism.** `retarget` generates the canonical action byte-for-byte deterministically for identical inputs, unconditionally (the strict floor on generation).
- **RD2c — realized-execution boundary.** A primitive whose realized execution runs against contact dynamics (`force` compliant search; `in_hand.pivot` passive drive) is held to Class 2-loose on realized execution (semantic equivalence within a per-skill ε), never byte-reproducibility; its canonical-action generation stays Class 2-strict.
- **RD3c — bimanual vs. inter-robot.** Bimanual handoff closes within one embodiment's `retarget` (fully deterministic, in-spec); inter-robot handoff's two-party determinism semantics are specified and its coordination protocol is a named v0.1 deferral.
- **RD4c — uncertainty-robust routing.** Capability negotiation routes an uncertain-geometry task to an envelope-capable embodiment by matching the task's geometry uncertainty against the embodiment's declared `grasp.envelope` robustness, beyond the binary `capability_absent` gate.

### Deferred and referenced

- **The Class 2-strict / Class 2-loose conformance split** and the per-skill ε-table — `05-conformance.md` § The determinism floor.
- **The two-party continuity invariant (GC6)** and the **`EffectorRef` addressing** the handoff semantics build on — `05-conformance.md` / `03-driver-interface.md`.
- **`grasp.envelope`'s declared robustness** the routing reads — `01-skill-isa.md` / `03-driver-interface.md`.

## Grasp-force and stability derivations

The `Envelope` (§ Canonical action representation) carries force and acceleration bounds; this section is where they are **derived**. Four quantities, all computed deterministically from the grasp's `StabilityMetadata` (`01` § Grasp state model), the object mass, the grasp geometry, and a declared friction coefficient — never runtime-measured, so `retarget` generation stays byte-deterministic (RD1c). They share one underlying quantity: the grasp's **holding capacity**.

### Holding capacity — the common quantity

A grasp's **holding capacity** is the maximum external load it can resist before the object moves in-grasp, and it is **directional** — capacity differs by axis and by closure type (`01` `StabilityMetadata.closure` / `secured_dof`):

- a **force-closure** grasp resists load by friction: capacity along an axis ≈ `μ · Σ(normal forces) · geometry factor`, bounded by the available grip force;
- a **form-closure** grasp resists load by shape along its `stable_directions` (high capacity there, ~none in the free directions);
- a **support** grasp bears load through the support polygon (balance), with capacity set by the CoM margin, not grip.

The four derivations below all compare a load against this capacity along the relevant axis. The capacity model is schematic and provider-neutral (Principle 4): RFL fixes the *dependency* — capacity is a function of closure, secured DOF, grip / normal force, geometry, and friction — not a single vendor's friction law.

### `min_holding_force` — the static minimum

`min_holding_force` is the grip force below which the object falls under its own weight — the static floor. It is derived from the object **weight** (`target.estimated_mass`), the **grasp mode** (which fixes how capacity scales with grip — force vs. form vs. support), the **friction** coefficient, and the **load direction** (gravity relative to the grasp's secured directions): the grip must produce holding capacity ≥ the gravity load along the unsecured axes. For a force-closure pinch it is roughly `m·g / (μ · geometry)`; for a form or support grasp the shape or balance bears the weight and the grip minimum is lower. It is the floor the grasp-continuity invariant checks (`05` GC1) and that `grasp.adjust` maintains.

### Dynamic stability — the `max_acceleration` clamp

Under acceleration the object's inertial load (`m·a`) adds to gravity (`m·g`); the grasp holds iff the combined load stays within holding capacity along the load direction. The **dynamic-stability limit** is the largest acceleration `a_max` at which `inertial + gravity load ≤ holding capacity` — solved from `StabilityMetadata` (`secured_dof` / `closure` / `flags`) + object mass + grasp geometry. It **clamps `max_acceleration`** in the `Envelope.motion_bounds` of every `transport` primitive (the dynamic counterpart of `min_holding_force`, and the core of transport safety). A `transport.carry`'s acceleration is clamped *below* this to reserve margin for its `disturbance_budget` (`05` ENV3).

### Reaction-load limit

A `force` primitive applies force to a target, and the **reaction** loads the grasp: an insertion's axial reaction, a press's normal reaction. The reaction must not exceed holding capacity along the reaction axis, or the held part slips before the task completes. The **reaction-load limit** is derived from `StabilityMetadata` + grasp geometry along the reaction axis, and bounds the force budget so the task completes without in-grasp slip. Two generalizations:

- **Rotational capacity** (`force.screw` / `force.unscrew`): the reaction is a **torque** about the tool axis; the grasp must resist it with rotational holding capacity (the form / friction resistance to twist), or the tool spins in-grasp.
- **Periodic-reversal load** (`force.scrub`): the tangential reaction **reverses** each stroke; the tool must be retained at *every* reversal, where the load direction flips — capacity must hold in both tangential directions, not just one.

### Tool-mediated force and coupled motion

`force.screw` (and later `force.cut`) transmit force / torque to the target through a **held tool** — a driver, a cutter. Two consequences for `retarget`:

- **The tool is the force-transmission path, and its grasp bears the reaction.** The reaction-load limit applies to the **tool's grasp** (tool-grasp-under-reaction-load), not the target's — the grip on the driver must resist the driving torque, or the driver slips in-hand. The canonical action represents the held tool as the transmission path so the reaction loads the right grasp.
- **Coupled DOF.** `force.screw` couples rotation to axial advance at `thread_pitch` (one turn ↔ `thread_pitch` of advance); `retarget` expands this coupling into the canonical action as a linked DOF, and a **decoupling** — advance without turn, or turn without advance — is a failure signal (a cross-thread or a stripped fastener), not a free parameter.

### Conformance obligations (grasp-force derivations)

- **GF1c — `min_holding_force` derivation.** `min_holding_force` is derived deterministically from object weight, grasp mode, friction, and load direction; it is the floor the grasp-continuity invariant (`05` GC1) checks and that `grasp.adjust` maintains.
- **GF2c — dynamic-stability clamp.** `max_acceleration` is clamped to the largest acceleration at which inertial + gravity load stays within holding capacity along the load direction, derived from `StabilityMetadata` + mass + geometry; applied to every `transport` primitive's `motion_bounds`.
- **GF3c — reaction-load limit.** A `force` primitive's reaction may not exceed holding capacity along the reaction axis (linear), the rotational capacity about the tool axis (torque reaction), or be lost at a periodic reversal (`scrub`); the limit is derived and the primitive aborts before in-grasp slip.
- **GF4c — tool-mediated transmission and coupling.** Force through a held tool loads the tool's grasp (tool-grasp-under-reaction-load); `force.screw`'s rotation↔advance coupling at `thread_pitch` is expanded into the canonical action, and a decoupling (advance without turn, or vice versa) is a failure signal.

### Deferred and referenced

- **`StabilityMetadata`** (`closure` / `secured_dof` / `flags`), the grasp modes, and the `force`-category force budgets these derivations bound — `01-skill-isa.md`.
- **The friction coefficient and object mass** the derivations consume (declared contact / target properties; the perception that estimates them is out of scope) — `01-skill-isa.md` § type system.
- **The interval force / torque-trajectory verification** that checks the realized profile against the derived bound — `05-conformance.md` § ENV4.

## Trajectory generation and timing

Two parts of `retarget` *generate* a path or a timing rather than compile a single pose — the scan sweep set `Σ` and the time-scaling of a trajectory. Both are **deterministic generators**: pure functions of their declared inputs, so generation stays Class 2-strict (RD1c) even though one of them (time-scaling) feeds a contact-bearing transport whose *realized* execution may be Class 2-loose.

### Normative sweep-pattern generators

`reach.scan` covers a region by a generated **sweep set `Σ`** — the ordered sensor poses whose union observes the region. `Σ` is a deterministic function of `(region, pattern, standoff, coverage_overlap, fov)`:

- the **observation footprint** at the surface is `2 · standoff · tan(half_fov)` (the `fov` from `03`'s sensor descriptor, the `standoff` from the primitive);
- the **pass spacing** is the footprint reduced by `coverage_overlap`;
- the **pattern** lays the passes out: `raster` = parallel passes at the spacing; `spiral` = an in / out spiral for a centered region; `arc` = a swept arc about a pivot; `waypoints` = the caller's ordered poses (the degenerate generator, `Σ` = the waypoints).

The four generators are **normative** — a normative appendix fixes the exact construction — so `Σ` is byte-reproducible across implementations (`reach.scan` C2, `05` Class 2). RFL fixes the geometric coverage construction; the perception that interprets what the sweep observes is out of scope (the `reach.scan` / `sense.inspect` capturability boundary, `04`).

### Time-scaling

`transport.follow_trajectory(timing_mode = time_scalable)` takes a caller-supplied path (a `Trajectory`, `01`) and **re-times** it — slowing the timing while **preserving the path shape** — until its curvature-and-speed profile fits within two bounds:

- the **dynamic grasp-stability limit** (the `a_max` of § Grasp-force and stability derivations) — the held object must not slip under the trajectory's accelerations;
- the **embodiment's kinematic limits** (`v` / `a` / jerk, `03`).

A trajectory that is safe slowly but would exceed dynamic stability at full speed is **slowed, not rejected** — the geometric route is unchanged, only the velocity profile along it is scaled down. The algorithm is deterministic: identical `(path, dynamic-stability limit, kinematic limits)` yield identical timing, preserving `retarget` determinism (RD1c). Preserving path *shape* is the contract — time-scaling never alters the geometric path, only its parameterization in time.

### Conformance obligations (trajectory generation)

- **TG1c — normative sweep set.** The sweep set `Σ` for `pattern ∈ {raster, spiral, arc, waypoints}` is a deterministic function of `(region, pattern, standoff, coverage_overlap, fov)` per § Appendix A — Normative sweep-pattern generators; identical inputs yield a byte-identical `Σ` (`reach.scan` C2).
- **TG2c — time-scaling determinism and shape preservation.** `time_scalable` re-times a trajectory — preserving the geometric path shape — until its curvature-and-speed profile fits within the dynamic-stability limit and the embodiment's kinematic limits; the re-timing is a deterministic function of `(path, limits)` and never alters the path.

### Deferred and referenced

- **The `Trajectory` and `ScanRegion` types**, and `reach.scan` / `transport.follow_trajectory` — `01-skill-isa.md`.
- **The `fov` and kinematic limits** the generators consume — `03-driver-interface.md` § Sensor descriptor / § Capability manifest.
- **The dynamic grasp-stability `a_max`** time-scaling fits within — § Grasp-force and stability derivations.
- **The exact per-pattern `Σ` construction** — § Appendix A — Normative sweep-pattern generators (this chapter).

## Resolved in the 2026-05-30 design pass

These issues surfaced during `reach` / `grasp` primitive design and are now addressed in the Skill ISA type system and the grasp state model (`01-skill-isa.md`). Detail lives in the spec body; retained here as a design-history trail.

- **`SurfaceTarget` / `ObjectTarget` / `FeatureRef` / `ScanRegion` types** → `01` § Skill ISA type system. Perception-derived target types are defined (fields + `UncertaintyBound`); the perception that produces them stays out of scope.
- **`ObjectTarget` center-of-mass** → `01` type system (`center_of_mass` field).
- **Grasp stability metadata in the result schema** (`closure`, `stable_directions`) → `01` § Grasp state model. Applied retroactively to `grasp.pinch` / `grasp.power`.
- **Stability-metadata generalization** (form-held vs friction-held vs balance-held DOF) → `01` `StabilityMetadata.secured_dof`.
- **Third closure type `support`** → `01` `StabilityMetadata.closure ∈ {force, form, support}`.
- **Extrinsic / surface-bound flags**, **`compliant`**, **`rotation_constrained`**, **`residual_mobility`** → `01` `StabilityMetadata.flags` / fields.
- **Active-grasp state model and `GraspRef`** → `01` § Grasp state model (`GraspState`, lifecycle FSM, transition table).
- **"Grasp core" abstraction structuring** → `01` Category 2 intro (core + contact-pattern delta documented; the full DRY rewrite of each primitive remains a pre-freeze formatting pass, not a semantic gap).

## Open issues

> **All groups resolved (2026-05-31).** Every item below is marked `[resolved → … § …]`: the `03`, `01`, `04`, and `05` chapter groups and this chapter's own `02` group are all closed. The section is retained as the cross-chapter design-history trail. The normative `Σ`-generator construction is now fixed in § Appendix A. Remaining pre-freeze work is implementation artifacts — the JSON Schema and the per-skill ε-tolerance table — not open design questions.

### Owned by `03-driver-interface.md` (embodiment descriptor + collision model)

> **Frame-declaration layer resolved (2026-05-30).** The embodiment frame model — `known_frames` / `control_frames`, the well-known `world` / `base` / `task` reference frames, per-control-frame `tool_axis`, the role tags `{grasp, sensor, support, tactile}`, the five `default_*_frame` fields, the `<rfl:capabilities>` binding, and conformance obligations F1–F4 — is now specified in `03` § Embodiment frame model. The remaining items below are the per-category capability flags, limits, collision-model, and sensor/tool/safety descriptors that build on it.

> **Capability-manifest skeleton resolved (2026-05-30).** The generic manifest structure the per-category declarations below instantiate is now specified in `03` § Capability manifest: primitive capabilities (keyed by ISA primitive ID; `reach.*` is the unkeyed baseline; grasp modes are `grasp.<mode>` sub-capabilities), auxiliary capabilities (`compliance` / `tactile_sensing` / `tool_safety` / `human_collaboration_safety`, with value domains), the flat `embodiment.limits.*` namespace with the capability→limit completeness rule, the uniform `capability_absent` validation gate, static manifest acquisition, the reserved `tier` syntax, and conformance obligations M1–M4. The per-category items below now reduce to *enumerating which flags / limits each category contributes* to this skeleton.

- ~~**Control-frame declaration**~~ **[resolved → `03` § Embodiment frame model]**: `control_frames` set and `default_control_frame` field; retargeting maps `controlled_frame → embodiment native frame`. Non-anthropomorphic embodiments may expose several control frames.
- ~~**`default_tool_axis`** (per control frame)~~ **[resolved → `03`]**: used by `reach.approach` / `reach.align` and the `force` family. Resolves to the active `controlled_frame`'s `tool_axis`.
- ~~**`default_sensor_frame` + per-sensor `fov` / `bore_axis` / `max_sweep_rate`**~~ **[resolved → `03` § Sensor descriptor]**: the frame-keyed `embodiment.sensors[sensor_frame]` map (`bore_axis`, the angular `fov` as `{h_angle, v_angle}` about the bore axis, `working_range`, `max_sweep_rate`, `modalities`), the non-contact-vs-contact scope split (contact-sensor fields stay in `04`), and the determinism of `Σ` from declared `fov`. `default_sensor_frame` itself was resolved in § Embodiment frame model.
- ~~**`tracking_bandwidth`** in `embodiment.limits`~~ **[resolved → `03` § Reach baseline limits]**: declared among the reach-baseline motion limits; gates `reach.hover(track_target)` and its `track_lost` mode.
- ~~**Collision-model target exclusion**~~ **[resolved → `03` § Collision model]**: the clearance query takes an `exempt` set (per-primitive: `target` / `region` / `against_surface` / `setdown_surface` / `support_object`) and a dual `augment` set (the freed object in `grasp.release`).
- ~~**Swept-volume clearance query**~~ **[resolved → `03` § Collision model]**: the query answers `at_pose` / `corridor` / `swept` geometries, all mandatory; swept-volume covers in-place rotation and trajectories, with self-collision checked.
- ~~**Per-grasp-mode descriptor fields**~~ **[resolved → `03` § Grasp-mode capabilities and limits]**: the eight mode capabilities (`grasp.pinch` / `power` / `hook` / `tripod` / `lateral` / `platform` / `pin` / `envelope`, canonical dotted keys for the Skill ISA's `*_grasp` spellings), the shared `grip_force_max` / `v_grasp` vs. per-mode payload / size limits (`payload_grasp_*`, `payload_support`, `hook_load_capacity`, `enclosure_span`, `precision_object_size_max`, `lateral_grasp_max_thickness`, `enclosure_range`), the frame-keyed `grasp_envelope(frame)` and `support_polygon(support_frame)` geometry, and the tactile-preferred-not-required policy. `default_grasp_frame` / `default_support_frame` were resolved earlier in § Embodiment frame model.
- ~~**In-hand descriptor fields**~~ **[resolved → `03` § In-hand capabilities and limits]**: category `in_hand` (no base primitive) + the seven sub-capabilities, the shared `w_inhand_max` / `v_inhand_max`, and the grasp-mode-keyed range maps `inhand_rotation_range[grasp_mode]` / `inhand_translation_range[grasp_mode]`. Slide's closed-loop slip sensing stays with `04`; DOF-admissibility with `01`.
- ~~**Transport descriptor fields**~~ **[resolved → `03` § Transport capabilities and limits]**: category `transport` (= base `move_to_pose`) + the five sub-capabilities, the general `v_cartesian_max` / `a_cartesian_max` (reach-introduced), and the transport-specific `cograsp_force_budget` (handoff) / `stability_margin` (carry). Held-object inclusion was resolved in § Collision model (`effector_with_held`); the dynamic-stability `max_acceleration` clamp and time-scaling stay with `02`; inter-robot handoff (`EffectorRef` + coordination channel) remains open as the multi-embodiment item below.
- ~~**Gravity-direction declaration**~~ **[resolved → `03` § Gravity declaration]**: `embodiment.world.gravity` is an optional world-frame acceleration vector (direction + magnitude); the `±gravity` defaults resolve from it deterministically; a gravity-free environment (absent / zero) makes the gravity-defaulted directions required and disables `drive = gravity` primitives. The dynamic-stability gravity term and mass↔weight reconciliation stay with `02` / `01` respectively.
- ~~**Place descriptor field**~~ **[resolved → `03` § Place capabilities]**: category `place` + the six sub-capabilities as a composite category — each presupposes the composed `transport.lower` / `grasp.release` (and `in_hand` for `orient`), adds no new force-dynamics axis, and introduces no new limit. The supported-state / recursive-stack / containment predicates stay with `01`.
- ~~**Compliance capability + force descriptor fields**~~ **[resolved → `03` § Compliance capability + § Force capabilities and limits]**: compliance (the `{passive, active, virtual}` domain vs. `{passive, active, auto}` request, normal-direction-compliance with `normal_compliance_range`, the rigid-only gate) and the ten `force` sub-capabilities with `oscillation_max` as the sole new limit (force budgets are target-clamped parameters). Tool-mediated force / thread coupling, hybrid-axis encoding, and the force-trajectory envelope stay with `02` / `05`.
- ~~**Sense descriptor fields**~~ **[resolved → `03` § Sense capabilities]**: category `sense` (no base primitive) + the five sub-capabilities, each with a sensing requirement referenced to `04` / the sensor descriptor; `max_probe_force` is target-derived (not an embodiment limit) and `sense` adds no new limit. `default_tactile_frame` was resolved in § Embodiment frame model; the disturb threshold is a perception-derived target property; the measurement-non-disturbance invariant stays with `05` / `04`.
- **Hybrid force/position control + contour following**: `force.wipe` / `force.scrub` control force along the surface normal and position along the tangent simultaneously (the classic hybrid controller). `retarget` must encode which axes are force-controlled vs position-controlled in the canonical action. Contour following needs declared **normal-direction compliance** whose range bounds the surface height variation the primitive can accommodate. **[Normal-direction-compliance capability + `normal_compliance_range` resolved → `03` § Compliance capability; the canonical-action force-vs-position axis encoding remains owned by `02`.]**
- ~~**Oscillation descriptor + wear/heat limit**~~ **[resolved → `03` § Force capabilities and limits]**: `embodiment.limits.oscillation_max` (`Frequency`, Hz) is `force.scrub`'s new limit; the optional wear/heat budget that bounds sustained abrasion is deployment-declared.
- ~~**`EffectorRef` and multi-embodiment addressing**~~ **[resolved → `03` § Multi-embodiment addressing]**: `EffectorRef` as local `frame_name` | remote `embodiment_id:frame_name` (the cross-embodiment generalization of `FrameRef`), the stable `embodiment.id`, the `coordination_channel` capability for inter-robot handoff, and the bimanual-vs-inter-robot split. The coordination *protocol* and two-party determinism stay with `02` (multi-embodiment-coordination item below); discovery / registry is a deployment concern.

### Owned by `05-conformance.md` (test classes)

- ~~**Interval-invariant test class**~~ **[resolved → `05` § The envelope-class taxonomy]**: one of the four envelope classes — sustained primitives (`reach.hover`; `transport.carry`) sample the maintained invariant at every step over the interval, distinct from the terminal-postcondition endpoint check; a `disturbance-rejecting` interval test injects calibrated disturbances up to `disturbance_budget` (ENV2 / ENV3). Interval sampling runs on the `04` manifold timebase for fixture determinism.
- ~~**Grasp-continuity envelope class**~~ **[resolved → `05` § Grasp-continuity modes, Base continuity]**: the held → held invariant "a securing contact set holds the object at ≥ `min_holding_force` at every sampled instant", verified from the force trace (GC1); the base mode of the most elaborate envelope class.
- ~~**Hold-test canonicalization + closure branching**~~ **[resolved → `05` § Grasp-continuity modes, The hold test and closure branching]**: the hold test (calibrated sub-budget perturbation → retention) is the operational definition of closure, with the perturbation profile branched on `closure ∈ {force: omnidirectional, form: load-direction, support: level-gentle}` (GC2) — one canonical procedure across all ten grasp modes.
- ~~**Non-degenerate-triangle confirmation**~~ **[resolved → `05` § Closure, stability, and composition verification, Non-degenerate tripod]**: a `grasp.precision_tripod` is confirmed only when its three contacts form a non-collinear triangle above a minimum-area threshold (else the claimed `rotation_constrained` is absent) — STB1.
- ~~**Support-grasp safe state**~~ **[resolved → `05` § Closure, stability, and composition verification, Support-grasp safe state]**: a `support` / `balance_held` grasp's safe response is a controlled lowering to the nearest surface minimizing fall height, never an open-release (STB2).
- ~~**Reversibility spectrum + persistent state change**~~ **[resolved → `05` § Reversibility and irreversible-operation safety, The reversibility spectrum]**: primitives are positioned reversible / semi-reversible-persistent / irreversible, safety weight rising with irreversibility (REV1); `force.snap_engage` is semi-reversible — engagement-confirmation (`confirm_held`) + a documented `snap_disengage` reverse path (REV2); the spectrum is open at both ends for `06` extensions.
- ~~**Irreversible-operation safety class**~~ **[resolved → `05` § Reversibility and irreversible-operation safety, The irreversible-operation safety class]**: an irreversible operation (`force.cut`) runs only under a strictly bounded action path, pre-execution confirmation of the path and the material beyond it, and precise partial / irreversible-state reporting on failure (REV3) — the precedent for future irreversible extensions (weld, adhesive).
- ~~**Tool-safety capability**~~ **[declaration → `03` § Safety capabilities; conformance bench resolved → `05` § Hazardous-operation conformance benches]**: hazardous tools (cutters in `force.cut`) require the declared `tool_safety` capability (`03`: `hazard_class` + `standard`, the hazardous-tool gate) and the instrumented bench (`05`: shear measurement, separation detection, hard-inclusion injection → safe arrest) — HAZ1 / HAZ2. A primitive wielding a hazardous tool must not run without the capability.
- ~~**Human-collaboration safety + handover conformance**~~ **[declaration → `03` § Safety capabilities; dummy-hand bench resolved → `05` § Hazardous-operation conformance benches]**: `place.hand_to` requires the declared `human_collaboration_safety` capability (`03`: `standard` + `max_interaction_force` / `weight_transfer_threshold`) and the instrumented dummy-hand bench (`05`: release only after weight transfer, exchanged force ≤ `max_interaction_force`, compliant yielding) — HAZ3 (ISO 10218 / 13482, L4).
- ~~**Force-trajectory envelope class**~~ **[resolved → `05` § The envelope-class taxonomy, The force/torque-trajectory class]**: the fourth envelope class — the `force` category's profile bounded interval-sampled against per-axis budgets (a mid-motion spike is a violation), generalized to a torque trajectory about an axis for `force.screw` / `force.unscrew` (ENV4). The terminating breakaway / detent events are evaluated on the same sampled trajectory (`04` § Force events).
- ~~**Composition validity by grasp stability class**~~ **[resolved → `05` § Closure, stability, and composition verification, Composition validity by stability class]**: the stability-class → permitted-successor table — `surface_bound` forbids free transport, `form_held` / `rotation_constrained` DOF forbid the corresponding `in_hand` op, `support` forbids free transport + open-release — a static validation atop the `01` lifecycle table, the conformance complement of `01`'s DOF-admissibility rule (STB3).
- ~~**Gaiting + make-before-break continuity verification**~~ **[resolved → `05` § Grasp-continuity modes, Make-before-break and gaiting]**: continuity holds iff at every instant the union of engaged contacts (old, new, or both) satisfies `min_holding_force`, and a regrasp confirms the new grasp *before* releasing the old (GC3); the most general form of the class, of which base continuity is the degenerate case.
- ~~**Controlled-under-actuation continuity mode**~~ **[resolved → `05` § Grasp-continuity modes, Controlled under-actuation]**: a pivot releases exactly the named DOF (under-constrained) while all others secure at ≥ `min_holding_force`, and re-secures it at completion (GC4) — continuity preserved through a relaxed, not swapped, constraint.
- ~~**Bounded continuity-exception subclass**~~ **[resolved → `05` § Grasp-continuity modes, Bounded continuity-exception]**: `in_hand.flip`'s continuity suspension is a bounded subclass — unsecured window ≤ `max_release_time`, re-catch within `catch_envelope`, safe-landing fallback in `safe_drop_zone` (GC5) — distinct from the continuity-preserving modes.
- ~~**Two-party / co-grasp continuity**~~ **[resolved → `05` § Grasp-continuity modes, Two-party co-grasp]**: at every instant at least one party secures the object at ≥ `min_holding_force`, and the combined force stays ≤ `cograsp_force_budget` in the dual-grasp window (GC6); bimanual is fully verifiable, inter-robot coordination stays a `02` open issue.
- ~~**`momentary_release` audit propagation**~~ **[resolved → `05` § Audit and transparency]**: `in_hand.flip` sets `momentary_release = true`, persisted and propagated downstream so the L4 / L8 loops can trace every continuity break (AUD2). Generalized: the audit record carries each result's evidence-bearing `Verdict`, its `04` fidelity tier on a degraded confirmation, and a freeing operation's freed-part disposition (AUD1 / AUD3) — RFL produces the evidence (`01` / `04`), `05` persists and propagates it; the L4 / L8 loops themselves stay in the strategy / governance docs.

### Owned by `04-tactile-manifold.md` (field set)

- ~~**Tactile-absent confirmation proxy**~~ **[resolved → `04` § Graceful degradation and the force/position proxy]**: the proxy is the conjunction of position-convergence (`grasp_width ≈ target cross-section`, not past it — the empty-close guard) and force-rise-and-hold; per-grasp-mode proxy expansions are tabulated. Force-closure confirmation is proxy-degradable; slip / deformation are proxy-irreducible and degrade to reactive-only at a lower fidelity tier — never a tactile hard-reject (the orthogonal `tool_safety` / `human_collaboration_safety` capabilities own the hard gates; risk-acceptance of a tier is the L4 decision).
- ~~**Bend / crease indicator**~~ **[resolved → `04` § Deformation, Crush vs. bend discrimination]**: `bending` (`BendState`: `flexing` / `crease_axis` / `curvature`) is a curvature-about-a-crease-line signal, discriminated from `deformation_rate` (symmetric bulk compression / crush) by the contact distribution shape — a pressure gradient indicates curling, uniform compression indicates crush. Backs `grasp.lateral`'s `bend_abort`; the shape indicator is proxy-irreducible (the declared `max_contact_force` / bend limit stays proxy-enforceable, the shape early-warning degrades to reactive-only).
- ~~**Intended-rolling vs gross-slip discrimination**~~ **[resolved → `04` § Slip, Rolling discriminator]**: the general `intended-slip model` classifies an observed `SlipState` (`stage`/`direction`/`rate`) against the command's expected slip; `roll`'s model is the rolling kinematics, so slip that skids beyond the migration rate, runs off the roll tangent, or persists at ω = 0 is `gross_slip`, while migration consistent with the commanded ω is controlled.
- ~~**Intended-slip vs drop-slip discrimination + closed-loop slip sensing**~~ **[resolved → `04` § Slip, Sliding discriminator + Closed-loop slip sensing]**: `slide`'s model is directional — the `slide_direction` slip component is intended feed, an orthogonal-DOF component beyond tolerance is `drop_slip` (discrimination by direction, not rate). Closed-loop slip sensing is the `slip` feature at loop-rate temporal resolution; absent, displacement tracking still runs (position proxy) but drop-slip detection degrades to reactive-only gross-escape at `proxy_reactive` tier (the irreducible-guard rule).
- ~~**Breakaway detection**~~ **[resolved → `04` § Force events, Breakaway]**: a `breakaway` is a `force_derivative` fall past a declared negative-rate threshold plus a settled lower effort level; it fires on the leading edge so the effector arrests without follow-through lurch, and recurs as `force.unscrew`'s `torque_drop`. Unified with detent under the `ForceEvent` type (the manifold resolution of the `StopCondition` `effort_drop` variant).
- ~~**Detent / snap detection**~~ **[resolved → `04` § Force events, Detent vs. bottoming-out]**: a `detent` is a rise-then-drop effort signature while motion continues (the click / snap); a monotone rise to a plateau is bottoming-out, never reported as actuation success. Resolves the `StopCondition` `detent` variant; the multi-rate temporal-alignment timebase (common monotonic timebase + slowest-contributing-feature grid) that makes `ForceEvent` detection deterministic is defined in the same section, resolving the chapter's second skeleton open issue.
- ~~**Freed-part handling**~~ **[resolved → `04` § Freed-part handling at constraint release]**: the freeing moment is a `ForceEvent` (breakaway for `force.unscrew`, cut-completion for `force.cut`) read as a constrained → free transition; on it a disposition contract binds — `retained` or `safe_zone_release(zone)`, never an uncontrolled drop — the same guarantee as `in_hand.flip`'s `safe_drop_zone` and `place.discard`. The freed part hands off to the collision augment set (`03`) and world-state (`01`); the contract binds even under degraded reactive-only detection.
- ~~Tactile-target representation under TactileManifold (general).~~ **[resolved → `04` § The `TactileTarget` type]**: a `TactileTarget` is a conjunction of `TactileClause`s (`feature`, comparator, `threshold`, site-quantifier) evaluated over the abstract site model (`04` § Site model) against the closed-core feature taxonomy; the grasp primitives' `auto` spellings are its canonical expansions, and an unsatisfiable target degrades to the force/position proxy rather than failing. The feature-field model, the closed-core + registry field-set decision, the contact-sensor descriptor `embodiment.tactile[tactile_frame]`, and per-feature uncertainty are specified in the same chapter.
- ~~**Measurement non-disturbance invariant**~~ **[resolved → `04` § Sensing-scope contracts, Measurement non-disturbance]** (verification owned by `05`): the contact-sensing scope — contact force suppressed below the target's disturb threshold (a target property), the dual of `force`'s budget (drive-up-to vs. suppress-below); `presence = false` is a valid measurement, never a fabricated reading. The suppression constraint is proxy-degradable to a force cap; the target-unchanged-across-measurement bench stays with `05`.
- ~~**Observation-capturability contract**~~ **[resolved → `04` § Sensing-scope contracts, Observation-capturability]**: the non-contact-sensing scope — `capturable = in_fov ∧ unoccluded ∧ in_range`, decided from `03`'s `fov` / `working_range`; occlusion / out-of-range is reported unobservable, never a confident reading from a bad view. Unified with non-disturbance under one RFL / perception boundary: RFL guarantees a measurement was physically valid to take, never its interpretation (perception / VLA).

### Owned by `01-skill-isa.md` (algebra + world state)

- ~~**`reactive`-body envelope precedence**~~ **[resolved → `01` § Predicates, verdicts, and three-valued control flow]**: a concurrent envelope violation takes precedence over a clean `until` completion, deterministically.
- ~~**`StatePredicate` type + `sense.verify` as the algebra `Predicate`**~~ **[resolved → `01` § Predicates, verdicts, and three-valued control flow]**: `Predicate` ≡ `StatePredicate` (one type) evaluating to a `Verdict`; two paths — internal-state predicates evaluate directly, perception-dependent ones via `sense.verify` (the general evaluator). The verdict drives `reactive` / `branch` / `force.scrub` `state_change`.
- ~~**Three-valued verdict in control flow**~~ **[resolved → `01` § Predicates, verdicts, and three-valued control flow]**: `branch` gains an optional `unknown` arm (absent → escalate, never coerce); `reactive`'s `until` terminates only on a confident `true`, so `indeterminate` never silently completes.
- ~~**Verdict honesty + evidence audit**~~ **[resolved → `01` § Predicates, verdicts, and three-valued control flow]**: `Verdict` asserts `true` / `false` only above `confidence_threshold`, else `indeterminate`, and always carries `evidence`. RFL owns producing the evidence-bearing verdict; the L4 / L8 audit loops that consume the persisted evidence stay with `05`.
- ~~**Start-in-contact + break-contact envelope clause**~~ **[resolved → `01` § World-state model, Shared break-contact clause]**: the `break_contact` postcondition + directional force-monotonicity, factored out as one clause shared by `reach.retract` / `grasp.release` / `place.put_down`.
- ~~**Supported-state predicate**~~ **[resolved → `01` § World-state model]**: `supported(object)` = CoM inside its contact polygon (the `grasp.platform` CoM-over-polygon test on a resting object); gates `grasp.release` / `place.*`.
- ~~**Recursive (stack) stability predicate + world-state support tracking**~~ **[resolved → `01` § World-state model]**: the world-state model tracks support relations ("A rests on B"); `stack_stable` applies the supported-state predicate recursively over the tracked chain (each level's CoM in the level-below polygon, combined CoM in the base polygon).
- ~~**Containment predicate + container geometry**~~ **[resolved → `01` § World-state model]**: `GeometryRef` gains a container aspect (opening + interior); `contained(object, container)` gates `place.insert_loose`; the `insert_loose` vs `insert_fit` selection criterion is "opening clearance > object cross-section".
- ~~**Last-resort `flip` admissibility**~~ **[resolved → `01` § Composition validity, Last-resort admissibility]**: the "does a continuity-preserving alternative exist?" check is part of composition validity; `flip` is admissible only when `rotate` / `roll` / `regrasp` cannot achieve the reorientation.
- ~~**Place ↔ in_hand reverse-dependency composition**~~ **[resolved → `01` § Composition validity, Reverse-dependency composition]**: the "compose a prerequisite primitive first" pattern — the planner inserts an `in_hand` reorientation before `place.orient` so its precondition holds, each primitive single-responsibility.
- ~~**Stability-metadata-driven admissibility**~~ **[resolved → `01` § Composition validity, DOF-admissibility]**: one rule — an `in_hand` operation moves a DOF only if it is `friction_held`, never `form_held` / `rotation_constrained` — with the per-primitive `*_inadmissible` modes as its instances.
- ~~**`ContactConfig` type**~~ **[resolved → `01` § Spatial, motion, and structural types]**: effector-region → object-surface mapping; `auto` or explicit for `in_hand.regrasp` / `grasp.*`.
- ~~**Rollable-geometry in `GeometryRef`**~~ **[resolved → `01` § Object reference, geometry, and measurement]**: `GeometryRef` exposes a rolling-surface aspect (cylinder / sphere / cone section + rolling axis) for `in_hand.roll`; container geometry stays with the containment (world-state) item.
- ~~**`SlideStop` type**~~ **[resolved → `01` § Stop / completion conditions]**: a restriction of the unified `StopCondition` family (`reached(distance) | landmark(tactile) | landmark(external)`).
- ~~**`Region` type**~~ **[resolved → `01` § Spatial, motion, and structural types]**: base spatial-extent type (geometry + frame); `ScanRegion` is now a `Region` plus a sweep `kind`.
- ~~**`Trajectory` type**~~ **[resolved → `01` § Spatial, motion, and structural types]**: ordered waypoints / spline + timing; shares the pose-sequence backbone with `reach.scan`'s `Σ` but stays caller-specified (Σ is `02`-generated).
- ~~**`MoveSpec` type**~~ **[resolved → `01` § Spatial, motion, and structural types]**: `to_pose(Pose6D) | trajectory(Trajectory)` for `transport.carry`.
- ~~**`OrientationSpec` type**~~ **[resolved → `01` § Spatial, motion, and structural types]**: a set of `(object_axis → Direction)` constraints for `place.orient`; functional-label-to-geometry conversion is the caller's.
- ~~**`Measurement` type**~~ **[resolved → `01` § Object reference, geometry, and measurement]**: the `sense` output type (optional presence / location / normal / stiffness / mass / center_of_mass / pose / predicate_result + uncertainty); populates `ObjectTarget` fields via `LetBind`. `predicate_result` references the three-valued `Verdict` defined with the algebra.
- ~~**`ObjectRef` type + resolution flow**~~ **[resolved → `01` § Object reference, geometry, and measurement]**: the unresolved object identity; `ObjectRef → sense.locate → Measurement(pose) → ObjectTarget`.
- ~~**Precision honesty + uncertainty matching**~~ **[resolved → `01` § Composition validity, `LetBind` flow checks]**: at a `LetBind` carrying a `sense.locate` pose, the algebra checks `measured uncertainty ≤ the primitive's target-resolution bound`; a pose is never stamped with unachieved precision.
- ~~**Sense → ObjectTarget measured-value supply (observe-act closed loop)**~~ **[resolved → `01` § Composition validity, `LetBind` flow checks]**: a `Measurement`'s `mass` / `center_of_mass` / `pose` populate the matching `ObjectTarget` fields via the binding, after which the downstream precondition (`estimated_mass ≤ payload`, …) is checkable — the primitive-level L1 Data loop.
- ~~**`SeatingSpec` type + seating/jam discrimination**~~ **[resolved → `01` § Stop / completion conditions]**: a restriction of the `StopCondition` family (`effort_rise(force) | reached(depth) | all_of{effort_rise, reached(depth)}`); the force-at-state seating/jam rule is the `all_of` conjunction (effort rise *at* depth = seated; without depth = jam). The detection informs `04` / `05`.
- ~~**`StopCondition` type family**~~ **[resolved → `01` § Stop / completion conditions]**: the seven named stop types (`SlideStop` / `SeatingSpec` / `PullStop` / `ScrewStop` / `ActuationSpec` / `CutStop` / `ScrubStop`) are unified into one `StopCondition` family (`reached` / `effort_rise` / `effort_drop` / `detent` / `count` / `elapsed` / `landmark` / `predicate` / `all_of` / `any_of`), each a restriction of it; the `all_of` conjunction carries the force-at-state seating/jam rule; `predicate(StatePredicate)` is the force↔sense coupling point. The `effort_drop` / `detent` detection stays with `04`; `StatePredicate` itself with the algebra item; the canonical-action encoding with `02`.
- ~~**New-`GraspRef` supersession**~~ **[resolved → `01` § Composition validity, `GraspRef` supersession + § Grasp state model]**: `in_hand.regrasp` / `transport.handoff` supersede the originating `GraspRef`; the stale handle is invalidated and a later use is a composition error caught at validation (dangling-reference prevention).

### Owned by `02-translation-layer.md` (this chapter)

- ~~**Pose representation choice** (SE(3) / quat+t / axis-angle)~~ **[resolved → § Pose representation and orientation error]**: `Pose6D` is position (R³) + unit quaternion (ROS 2 `geometry_msgs/Pose` parity; SE(3) the group beneath). The single-scalar geodesic orientation error `θ_orient = 2·arccos|⟨q_t, q_c⟩|` makes `pose_not_reached` decidable (CA1c).
- ~~**Under-constrained-orientation residual rule** (*decided*)~~ **[resolved → § Pose representation and orientation error]**: among orientations satisfying the declared constraints, `retarget` picks the minimum-geodesic-rotation one from the current orientation, uniformly — deterministic, binding for `retarget` determinism (CA2c).
- ~~**Rest-at-goal vs. trajectory blending**~~ **[resolved → § Terminal semantics — rest-at-goal]**: v0.1 guarantees rest-at-goal; `timing` reserves `stop_at_goal: bool` (default `true`) so future non-stop blending is additive without breaking the guarantee (CA3c, Principle 5).
- ~~**Normative sweep-pattern generators**~~ **[resolved → § Trajectory generation and timing, Normative sweep-pattern generators]**: `Σ` for `pattern ∈ {raster, spiral, arc, waypoints}` is a deterministic, byte-reproducible function of `(region, pattern, standoff, coverage_overlap, fov)` — footprint `2·standoff·tan(half_fov)`, spacing reduced by overlap (TG1c); the exact per-pattern construction is fixed in § Appendix A — Normative sweep-pattern generators.
- ~~**`min_holding_force` derivation**~~ **[resolved → § Grasp-force and stability derivations, `min_holding_force`]**: derived deterministically from object weight, grasp mode, friction, and load direction against the grasp's directional holding capacity; the static floor the grasp-continuity invariant (`05` GC1) checks and `grasp.adjust` maintains (GF1c).
- ~~**Dynamic grasp-stability limit derivation**~~ **[resolved → § Grasp-force and stability derivations, Dynamic stability]**: the largest acceleration at which inertial + gravity load stays within holding capacity, from `StabilityMetadata` + mass + geometry; clamps `max_acceleration` in every `transport` primitive's `Envelope.motion_bounds` (GF2c) — the dynamic counterpart of `min_holding_force`.
- ~~**Grasp-under-reaction-load**~~ **[resolved → § Grasp-force and stability derivations, Reaction-load limit]**: a `force` primitive's reaction may not exceed holding capacity along the reaction axis, the rotational capacity about the tool axis (torque reaction, screw/unscrew), or be lost at a periodic reversal (scrub); derived and aborted-before-slip (GF3c).
- ~~**Tool-mediated force + coupled-motion constraint**~~ **[resolved → § Grasp-force and stability derivations, Tool-mediated force and coupled motion]**: a force transmitted through a held tool loads the *tool's* grasp (tool-grasp-under-reaction-load); `force.screw`'s rotation↔advance coupling at `thread_pitch` is expanded into the canonical action, and a decoupling (advance without turn, or vice versa) is a failure signal (GF4c).
- ~~**Uncertainty-robustness in capability negotiation**~~ **[resolved → § Capability negotiation and routing]**: negotiation matches a task's geometry uncertainty against an embodiment's declared `grasp.envelope` robustness, routing uncertain-geometry tasks to envelope-capable embodiments — the retarget-side complement to `03`'s binary `capability_absent` gate (RD4c).
- ~~**Passive-drive determinism boundary**~~ **[resolved → § The determinism boundary]**: `retarget` generation is byte-deterministic unconditionally, but the *realized execution* of a contact-dynamics primitive (`in_hand.pivot` passive drive; all `force` compliant search) is not — it is held to Class 2-loose (semantic equivalence within a per-skill ε) on realized execution, Class 2-strict on generation (RD2c). Parallel to `reach.scan`'s purely-kinematic strict archetype.
- ~~**Multi-embodiment coordination + determinism**~~ **[resolved → § Multi-embodiment coordination]**: bimanual handoff closes within one `retarget` (fully deterministic, in-spec); inter-robot handoff's two-party determinism semantics (GC6 + `EffectorRef`) are specified and its coordination *protocol* (the dual-grasp-window wire mechanism) is a named v0.1 deferral (RD3c). Semantics-now, mechanism-later.
- ~~**Time-scaling semantics**~~ **[resolved → § Trajectory generation and timing, Time-scaling]**: `transport.follow_trajectory(timing_mode = time_scalable)` re-times a trajectory (preserving path shape, slowing not rejecting) until its curvature-and-speed profile fits within the dynamic-stability limit and the embodiment's kinematic limits; a deterministic function of `(path, limits)` that preserves `retarget` determinism (TG2c).

## Appendix A — Normative sweep-pattern generators

This appendix fixes the exact construction of the scan sweep set `Σ` for each `pattern`, making `Σ` a byte-reproducible function of `(region, pattern, standoff, coverage_overlap, fov)` (TG1c; `reach.scan` C2). The construction is **normative**: a conformant `retarget` produces the identical `Σ` for identical inputs. RFL fixes the geometric coverage; the perception that interprets the captured observations is out of scope (`04`).

### Common construction

All patterns share a setup, computed in the region's reference frame (`Region` = geometry + `frame`, `01`):

- **Footprint.** The `fov` (`03` § Sensor descriptor — an angular field `{h_angle, v_angle}` about the bore axis) and the `standoff` give a rectangular observation footprint on the surface: `f_u = 2 · standoff · tan(h_angle / 2)`, `f_v = 2 · standoff · tan(v_angle / 2)`.
- **Pass spacing.** Reduced by overlap: `s_u = f_u · (1 − coverage_overlap)`, `s_v = f_v · (1 − coverage_overlap)`, with `coverage_overlap ∈ [0, 1)`.
- **Surface parameterization.** A surface `ScanRegion` is parameterized over its axis-aligned bounding rectangle `[0, U] × [0, V]` in the region frame (the bounds computed deterministically from the region geometry). A station at parameter `(u, v)` maps to a sensor pose: position `= surface_point(u, v) + standoff · n̂(u, v)` (outward normal); orientation `=` the sensor `bore_axis` anti-parallel to `n̂`, with the bore frame's secondary axis aligned to the `+u` direction (a fixed roll convention, so the pose is fully determined).

`Σ` is the ordered list of these poses; the order is the scan path (a continuous sweep). Counts use `ceil` and centered placement so coverage is symmetric and complete.

### `raster`

Parallel passes across the surface, serpentine-ordered:

```
n_v = ceil(V / s_v)                          # number of passes
v_k = (k + 0.5) · V / n_v,   k = 0 … n_v−1
n_u = ceil(U / s_u)                          # stations per pass
u_j = (j + 0.5) · U / n_u,   j = 0 … n_u−1
Σ   = [ pose(u_j, v_k) ]   with j ascending on even k, descending on odd k   # boustrophedon
```

The serpentine (boustrophedon) order makes `Σ` a continuous path — each pass starts where the previous ended — minimizing transit and fixing the order deterministically.

### `spiral`

An Archimedean spiral outward from the surface centroid, for a centered region:

```
s     = min(s_u, s_v)                         # isotropic pass spacing
r(θ)  = (s / 2π) · θ                          # radial pitch per turn = s
stations at constant arc-length step Δℓ = s, from θ = 0 outward
       until r(θ) exceeds the region's bounding radius
Σ     = [ pose at polar (r(θ), θ) ]           # CCW from the centroid, by convention
```

The fixed start (centroid), fixed radial pitch (`s`), fixed arc-length step, and fixed direction (CCW) make the spiral byte-reproducible.

### `arc`

A swept arc at fixed radius about a declared pivot (e.g. circumferential inspection of a cylindrical surface):

```
radius = standoff                             # sensor sweeps at standoff from the pivot
Δθ     = s_u / standoff                        # angular step so the footprint advances by s_u along the arc
n      = ceil(arc_extent / Δθ)
θ_i    = arc_start + (i + 0.5) · arc_extent / n,   i = 0 … n−1
Σ      = [ pose on the arc at θ_i, bore pointing at the pivot ]
```

### `waypoints`

The degenerate generator: the caller supplies the ordered poses and `Σ` is exactly them, verbatim. `standoff` / `coverage_overlap` / `fov` are ignored — the caller owns coverage. This is the escape hatch for coverage geometries the three parametric patterns do not capture, kept deterministic because the poses are given, not generated.

### Region kinds beyond surface

A `surface` `ScanRegion` uses the generators directly. A `path` region sweeps stations spaced `s_u` along the given path (a one-dimensional `raster`). A `volume` region is covered as a deterministic stack of surface layers at depth intervals `s_v` along the volume's principal axis, each layer a surface sweep — so the volume reduces to repeated surface coverage. The per-kind reduction keeps one parametric construction rather than three.
