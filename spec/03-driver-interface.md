# Driver Interface — Specification (skeleton)

> **Status**: pre-release skeleton. Full text targeted for v0.1 (2027 Q1).

## Scope

This chapter defines:

1. The ROS 2 message types and action interfaces an embodiment driver must expose
2. The URDF / MJCF extension blocks RFL adds on top of the standard formats
3. The capability manifest schema (what the embodiment can / cannot do)
4. The driver lifecycle (init / capability negotiation / execute / report / shutdown)

## ROS 2 compatibility

The Driver Interface is **ROS 2-compatible** but not ROS 2-mandatory: an embodiment that does not use ROS 2 may implement the protocol over a different transport (gRPC, MQTT, DDS direct) provided the wire format matches the canonical message definitions in `schemas/driver-interface/`.

## URDF / MJCF extensions

RFL adds two extension blocks:

1. `<rfl:capabilities>` inside the URDF / MJCF `<robot>` element — declares which Skill ISA categories the embodiment supports and at what fidelity tier
2. `<rfl:calibration>` — embodiment-specific calibration metadata (e.g., pneumatic pressure-to-force conversion table) consumed by the Translation Layer's retargeting algorithm

Both extensions are **optional**: an embodiment may ship a vanilla URDF / MJCF file and supply the RFL extensions in a sidecar file.

## Embodiment frame model

The Skill ISA expresses every spatial parameter relative to a **frame**, and every primitive defaults its frame arguments to embodiment-declared descriptor fields (`embodiment.default_control_frame`, `embodiment.default_grasp_frame`, …). This section defines what those fields mean, how an embodiment declares them inside `<rfl:capabilities>`, and the conformance obligations they incur. It is the foundation the per-category capability and limit declarations (below) build on: a capability flag or limit is always scoped to one of the frames declared here.

The model is deliberately **non-anthropomorphic** (Principle 1). It names roles — *control*, *grasp*, *sensor*, *support*, *tactile* — never body parts (*wrist*, *hand*, *eye*). A non-anthropomorphic platform (a continuum arm, a mobile base with a single suction cup, a delta robot) declares the same frame structure as a humanoid; only the role assignments differ.

### Frame namespaces

RFL distinguishes two namespaces over the same universe of named, calibration-tracked frames.

| Concept | Definition | Skill ISA reference |
|---|---|---|
| **Frames** (`embodiment.known_frames`) | Every named frame the embodiment exposes: URDF link / MJCF body frames, plus RFL-declared TCP and reference frames. Each is rigidly fixed to a kinematic-tree element or to the world and tracked for calibration. Any `calibration_valid` member may serve as a **reference frame** — a frame a `Pose6D` is expressed *in*. | `reach.to_pose` `frame` parameter |
| **Control frames** (`embodiment.control_frames`) | The subset of frames the embodiment can drive to a commanded Cartesian pose (it owns an IK / control path to them). Every role-specialized frame is a control frame carrying a role tag. | `controlled_frame` parameter of every motion primitive |

A `FrameRef` (the Skill ISA geometric type, `01` § Geometric types) resolves against `known_frames`. Within v0.1 a `FrameRef` addresses a frame on **one** embodiment; cross-embodiment addressing (`transport.handoff`'s `receiver`) is a reserved extension seam — see *Open issues*.

#### Well-known reference frames

Three reference-frame roles are reserved and MUST resolve for any conformant embodiment:

| Reserved name | Meaning |
|---|---|
| `world` | A fixed inertial frame. RFL does **not** assume a "down" in `world`; the gravity vector is declared separately (see *Open issues* — gravity-direction declaration). |
| `base` | The embodiment's root link frame (the URDF / MJCF root). "Base", not "torso": it carries no anthropomorphic meaning. |
| `task` | The active task frame. May be set by the caller; `reach.to_pose`'s `frame` parameter defaults to it. |

The embodiment declares the transform chain relating these so that any declared frame resolves into any other. Resolvability of `world` / `base` / `task` is binding for `retarget` determinism (`02`): a pose whose reference frame does not resolve is rejected, never silently reinterpreted.

### Control-frame declaration

Each control frame is declared with three attributes:

| Attribute | Type | Meaning |
|---|---|---|
| `link` | URDF link / MJCF body name (tf2 `frame_id`) | The kinematic-tree element the control frame is rigidly attached to. Grounds the control frame in the standard formats and in the ROS 2 tf tree. |
| `tool_axis` | `SignedAxis` (`±{x,y,z}` of this frame) | The frame's primary working / approach direction. |
| `roles` | subset of `{grasp, sensor, support, tactile}` | The manipulation roles this control frame can serve. Empty is legal: a frame usable only for free-space `reach` carries no role. |

Membership in `control_frames` *is* the controllability capability; there is no separate `control` role tag. A non-anthropomorphic embodiment may declare several control frames (e.g. one per arm, or per tool turret station); a minimal embodiment declares one.

#### `tool_axis` resolution (`default_tool_axis`)

The Skill ISA writes `embodiment.default_tool_axis` as the default for `approach_axis` / `retract_axis` / `withdraw_axis` and the `force` family's force-application axis. It is **per control frame**, not an embodiment-global constant: `embodiment.default_tool_axis` resolves to the `tool_axis` of the **currently resolved `controlled_frame`**. Overriding `controlled_frame` therefore also changes which `tool_axis` the defaulted axis arguments inherit — the behaviour a caller expects when retargeting a primitive onto a different effector.

### Role frames and defaults

A role-specialized frame is a control frame whose `roles` includes that role. The embodiment names one default per role; primitives fall back to these:

| Descriptor field | Default for | Constraint |
|---|---|---|
| `embodiment.default_control_frame` | `controlled_frame` of the `reach` family | Any member of `control_frames`. |
| `embodiment.default_grasp_frame` | `controlled_frame` of `grasp.*`, `in_hand.*` | A control frame tagged `grasp`. |
| `embodiment.default_sensor_frame` | `sensor_frame` of `reach.scan`, `sense.inspect` | A control frame tagged `sensor`. |
| `embodiment.default_support_frame` | `controlled_frame` of `grasp.platform` | A control frame tagged `support`. |
| `embodiment.default_tactile_frame` | probing frame of `sense.probe` | A control frame tagged `tactile`. |

One frame may carry several roles (a parallel-gripper TCP is commonly `grasp` + `sensor` + `tactile`), so the defaults need not be distinct. Per-role metadata maps are keyed by frame name: `embodiment.workspace(controlled_frame)` (IK reachability, used by every `reach` precondition), `embodiment.grasp_envelope(controlled_frame)` and the per-grasp-mode limits (defined in the capability-manifest section below), and `embodiment.sensors[sensor_frame]` (FOV / `bore_axis` / `max_sweep_rate`, defined in the sensor-descriptor section below).

### URDF / MJCF binding

The frame model is declared inside the `<rfl:capabilities>` block introduced above. The XML is the URDF / MJCF-native surface; it maps one-to-one onto the abstract `embodiment.*` fields and onto the JSON `schemas/embodiment-descriptor.schema.json` whose file format is owned by `02-translation-layer.md`.

```xml
<rfl:capabilities>
  <rfl:reference-frames world="world" base="base_link" task="task"/>
  <rfl:control-frames default="tcp_right">
    <rfl:frame name="tcp_right"  link="right_tool0" tool-axis="+z" roles="grasp sensor tactile"/>
    <rfl:frame name="tcp_left"   link="left_tool0"  tool-axis="+z" roles="grasp"/>
    <rfl:frame name="palm_right" link="right_palm"  tool-axis="+x" roles="support"/>
  </rfl:control-frames>
  <rfl:role-defaults grasp="tcp_right" sensor="tcp_right"
                     support="palm_right" tactile="tcp_right"/>
</rfl:capabilities>
```

- `default` on `<rfl:control-frames>` supplies `embodiment.default_control_frame`.
- `<rfl:role-defaults>` supplies the four role-default fields; each attribute MUST name a frame carrying the matching role.
- Role tags are extensible through `06-extension-registry.md` namespaces (Principle 5). An **unknown** role tag is rejected, not ignored: silently dropping a tag could mask a capability the planner relied on. New roles enter under a namespace prefix (e.g. `rfl-x:vacuum`).

### Conformance obligations

The frame declaration is mechanically checkable (Principle 3); the conformance suite (`05`) verifies:

- **F1 — controllability.** Every declared control frame accepts a `reach.to_pose` to a bench-reachable pose within `embodiment.workspace(frame)`, and external metrology confirms arrival. A frame that cannot be driven may not be declared a control frame.
- **F2 — tool-axis validity.** Each `tool_axis` is one of `±{x,y,z}` of its own frame.
- **F3 — role-default consistency.** Each of the four role defaults names a control frame whose `roles` includes that role.
- **F4 — well-known frame resolution.** `world`, `base`, and `task` resolve and are `calibration_valid`, and the declared transform chain relates every declared frame to each of them.

## Capability manifest

Not every embodiment implements every Skill ISA primitive, and a primitive often needs an orthogonal capacity (a compliance mode, a sensing modality, a safety regime) beyond the bare operation. The **capability manifest** is the embodiment's declaration of what it can do. It is the structure every per-category capability declaration below — grasp modes, in-hand sub-capabilities, transport, place, force, sense, collision, sensor, and safety — instantiates, so that capability checking is uniform across all fifty primitives.

The manifest extends the `<rfl:capabilities>` block alongside the frame model. It declares three kinds of entry — **primitive capabilities**, **auxiliary capabilities**, and **limits** — over a forward-compatible key space.

### Primitive capabilities

A primitive capability is a boolean assertion *"this embodiment implements operation X."* Its key **is the Skill ISA primitive identifier** (`grasp.pinch`, `transport.follow_trajectory`, `force.scrub`, `sense.weigh`), or a category identifier (`transport`) standing for that category's base primitive. Binding the capability key space to the ISA enumeration keeps naming unambiguous and makes declaration directly checkable against the primitive's own conformance test.

- **`reach.*` carries no capability key.** Free-space motion is the mandatory baseline every embodiment implements; the `reach` family is assumed-present and not declared.
- **A category key implies the base primitive.** `transport` asserts `transport.move_to_pose`. A non-base primitive requires *both* its category key and its own key — `transport.follow_trajectory` requires `transport` **and** `transport.follow_trajectory`. (This is what the Skill ISA preconditions phrase as "declares `transport` with `follow_trajectory` support.")
- **Grasp modes are sub-capabilities of `grasp`.** The ten grasp primitives' modes are the canonical keys `grasp.pinch`, `grasp.power`, `grasp.hook`, `grasp.tripod`, `grasp.lateral`, `grasp.platform`, `grasp.pin`, `grasp.envelope`. The Skill ISA's top-level spellings (`pinch_grasp`, `power_grasp`, …) denote these same keys; aligning the per-primitive prose to the dotted form is a pre-freeze formatting pass (as with the grasp-core DRY rewrite), not a semantic change.

### Auxiliary capabilities

An auxiliary capability is a named capacity **orthogonal to any single primitive** — several primitives may require the same one, and one primitive may require several. Unlike primitive capabilities it is not always boolean: it may carry an enumerated value or (for the safety regimes, defined in their own units below) a structured declaration.

| Auxiliary capability | Value domain | Required by (examples) | Defined in |
|---|---|---|---|
| `compliance` | `{passive, active, virtual}` (+ a declared compliant direction where a primitive needs one) | all `force.*`; `force.wipe` / `force.scrub` need normal-direction compliance | force-descriptor unit |
| `tactile_sensing` | boolean | all `grasp.*` (preferred; absence degrades to a force/position proxy, Principle 5) | tactile manifold (`04`) |
| `tool_safety` | structured | `force.cut` and other hazardous-tool primitives | safety-capability unit / `05` |
| `human_collaboration_safety` | structured | `place.hand_to` | safety-capability unit / `05` |

A primitive's precondition names the primitive capability plus any auxiliary capabilities it needs; all are checked together at validation. This table lists the auxiliary capabilities surfaced so far; each is fully specified in the unit noted.

### Limits

A limit is an SI-valued scalar or range in the flat namespace `embodiment.limits.*`, exactly as the Skill ISA references them (`embodiment.limits.v_cartesian_max`, `embodiment.limits.payload_grasp_pinch`). Limits are kept flat — not nested under the capability that uses them — to match those references; the association between a limit and the capability it bounds is documentary, recorded per-capability in the units below.

The binding rule (verifiable): **if a primitive capability is asserted, every `embodiment.limits.*` key its primitives reference MUST be present.** A manifest that claims `grasp.pinch` but omits `payload_grasp_pinch` is malformed.

### Capability checking — the uniform `capability_absent` gate

Capability satisfaction is a **deterministic validation-phase gate**, evaluated before any motion: the planner matches each primitive's required capabilities (primitive + auxiliary) against the manifest, and on any miss returns `capability_absent` — reject, no attempt, no embodiment displacement. Every primitive's `capability_absent` failure mode is this single contract; it is defined once here rather than re-specified per primitive.

### Manifest acquisition and fidelity tier

The manifest is **static**: declared in the URDF / MJCF `<rfl:capabilities>` block (or its sidecar) and acquired once at load-time / capability negotiation. This fixes the static portion of the capability-negotiation-timing question (see *Open issues*); per-action dynamic renegotiation remains open.

Each capability assertion may carry a `tier` attribute reserved for the **fidelity tier** the embodiment claims for that operation. The tier's semantics — how a tier maps to a conformance class and badge — are owned by `05-conformance.md`; this chapter reserves only the syntax.

### URDF / MJCF binding

```xml
<rfl:capabilities>
  <!-- frame model: § Embodiment frame model -->
  <rfl:skills>
    <rfl:skill id="grasp.pinch" tier="full"/>
    <rfl:skill id="grasp.power"/>
    <rfl:skill id="transport"/>                  <!-- category baseline = transport.move_to_pose -->
    <rfl:skill id="transport.follow_trajectory"/>
    <rfl:skill id="force.scrub"/>
  </rfl:skills>
  <rfl:aux>
    <rfl:capability name="compliance" value="active"/>
    <rfl:capability name="tactile_sensing" value="true"/>
  </rfl:aux>
  <rfl:limits>
    <rfl:limit name="payload_grasp_pinch" value="2.0"  unit="N"/>
    <rfl:limit name="v_grasp"             value="0.05" unit="m/s"/>
  </rfl:limits>
</rfl:capabilities>
```

The XML maps onto the abstract fields `embodiment.capabilities` (the asserted primitive-key set), `embodiment.aux` (auxiliary name → value), and `embodiment.limits.*` (flat). Skill IDs and auxiliary names are extensible through `06-extension-registry.md` namespaces (Principle 5); an **unknown skill ID is rejected** (the suite must be able to test every asserted capability), and an unknown auxiliary name must carry a namespace prefix.

### Conformance obligations

- **M1 — capability ↔ implementation.** For every asserted primitive capability, that primitive's C1 nominal conformance test passes. Asserting a capability is a binding promise to implement it.
- **M2 — limit completeness.** Every `embodiment.limits.*` key referenced by an asserted capability's primitives is present and SI-valued.
- **M3 — auxiliary value domain.** Each enumerated auxiliary capability (`compliance`, …) holds a value within its defined domain.
- **M4 — `capability_absent` determinism.** A primitive requiring an undeclared capability is rejected at validation with no attempt (externally measured displacement `< ε`).

### Grasp-mode capabilities and limits

The `grasp` category contributes eight mode capabilities to the manifest. Each is a primitive capability under § Capability manifest; this section enumerates the limits and frame geometry each mode requires and *why* — the limit set follows from the mode's closure type, which is fixed by the grasp state model (`01-skill-isa.md` § Grasp state model and stability metadata, the canonical source). The manifest does not redeclare closure; asserting `grasp.pinch` carries its `force` closure by definition.

| Capability | Closure | Payload limit | Size limit | Force / velocity | Frame geometry | Stability flags |
|---|---|---|---|---|---|---|
| `grasp.pinch` | force | `payload_grasp_pinch` | — | `grip_force_max`, `v_grasp` | `grasp_envelope` | — |
| `grasp.power` | force | `payload_grasp_power` | `enclosure_span` (max characteristic size) | `grip_force_max`, `v_grasp` | `grasp_envelope` | — |
| `grasp.tripod` | force | `payload_grasp_tripod` | `precision_object_size_max` | `grip_force_max`, `v_grasp` | `grasp_envelope` | `rotation_constrained` |
| `grasp.lateral` | force | `payload_grasp_lateral` | `lateral_grasp_max_thickness` | `grip_force_max`, `v_grasp` | `grasp_envelope` | — |
| `grasp.pin` | force | — (the external surface bears the load) | — | `grip_force_max`, `v_grasp` | `grasp_envelope` | `extrinsic`, `surface_bound` |
| `grasp.envelope` | form | `payload_grasp_envelope` | `enclosure_range` (a `[min, max]` range) | `grip_force_max`, `v_grasp` | `grasp_envelope` | `compliant` / `residual_mobility` |
| `grasp.hook` | form | `hook_load_capacity` (directional load rating) | — | `v_grasp` | `grasp_envelope` | — (directional form hold) |
| `grasp.platform` | support | `payload_support` | — | `v_cartesian_max` | `support_polygon` | balance-held |

Reading the table by closure type explains the limit asymmetries:

- **Force-closure modes** oppose the object with controlled grip force, so they share `grip_force_max` and carry a per-mode payload. `grasp.pin` is the exception with **no payload limit**: it is extrinsic force closure — the pinned object's weight is borne by the external `against_surface`, not the effector — and is `surface_bound` (no free transport). A missing pin payload is correct, not an omission.
- **`grasp.hook`** is form closure with a directional hold; it applies no opposing grip force, so `grip_force_max` does not apply. Its load capacity is the directional `hook_load_capacity`.
- **`grasp.platform`** is balance-held support; it forms no closing grip (hence `v_cartesian_max`, the placement-motion bound, not `v_grasp`) and its load limit is `payload_support`.

#### Shared vs. per-mode limits

Two grasp limits are **embodiment-level** — declared once, shared by every force-closure mode the embodiment supports:

- `grip_force_max` — the ceiling on commanded grip force across `grasp.pinch` / `power` / `tripod` / `lateral` / `pin` / `envelope`.
- `v_grasp` — the closing / threading velocity ceiling for every mode that forms contact by closing (all but `grasp.platform`).

Every other grasp limit (`payload_grasp_*`, `payload_support`, `hook_load_capacity`, the size limits) is **per-mode** and required only when its mode is declared. The manifest's M2 (limit completeness) applies at this granularity: declaring `grasp.power` requires `payload_grasp_power`, `enclosure_span`, and the shared `grip_force_max` / `v_grasp`.

#### Frame-keyed grasp geometry

Two grasp inputs are not scalars but geometry keyed by the grasp frame, alongside the per-frame `embodiment.workspace(frame)` of the frame model:

- **`embodiment.grasp_envelope(controlled_frame)`** — the region (object pose / position) within which the named grasp frame can *form* a closure. The grasp preconditions require `target` to lie within it. It is a subset of `workspace(frame)` (IK reachability): reachability is necessary but not sufficient — `grasp_envelope` is the tighter volume in which the effector can actually close on an object. It bounds the **final approach only**; gross transit to bring the target into the envelope is the `reach` family's responsibility.
- **`embodiment.support_polygon(support_frame)`** — the support-contact polygon over which a balance-held object's center of mass must project for `grasp.platform`. With the primitive's `support_normal` parameter, it is the embodiment-side input to the CoM-over-polygon stability decision.

Both are declared per frame and extensible through `06-extension-registry.md`. An embodiment that cannot declare `grasp_envelope` for a grasp frame may not assert a grasp mode on it; one that cannot declare `support_polygon` may not assert `grasp.platform`.

#### Tactile sensing is preferred, never required

`grasp` never hard-requires the `tactile_sensing` auxiliary capability (Principle 5). Every mode lists it as *preferred*: when it is declared, contact confirmation uses the TactileManifold; when it is absent, confirmation degrades to a force / position proxy (the proxy is specified in `04-tactile-manifold.md`). A grasp-mode capability is therefore declarable independently of `tactile_sensing` — a non-tactile embodiment can still assert and pass `grasp.pinch`.

#### Conformance obligations (grasp)

- **G1c — per-mode limit completeness.** For each asserted grasp mode, the payload / size limits in the table above plus the shared `grip_force_max` / `v_grasp` are present and SI-valued (the grasp specialization of M2).
- **G2c — `grasp_envelope` queryable.** For each grasp frame carrying a declared mode, `grasp_envelope(frame)` is declared and a bench object placed within it passes that mode's C1 nominal test.
- **G3c — `support_polygon` present.** When `grasp.platform` is asserted, `support_polygon(support_frame)` is declared and yields a polygon usable for the CoM-over-polygon decision.
- **G4c — tactile independence.** With `tactile_sensing` undeclared, each asserted grasp mode still passes its C1 test via the force / position proxy.

The closure-dependent hold test that verifies these grasps (omnidirectional for `force`, `load_direction`-only for `form`, level-gentle for `support`) is owned by `05-conformance.md`.

### Compliance capability

The `force` category makes contact force the objective, and most of its primitives cannot run on a rigid embodiment: they need the effector to *yield* in a controlled way. **Compliance** is the auxiliary capability (introduced in the auxiliary-capability table above) that declares how an embodiment can do that. A rigid-only embodiment — one that declares no compliance mode — cannot run any `force` primitive that requires compliance.

#### Modes

The compliance capability declares the **set** of modes the embodiment provides, each drawn from:

| Mode | Meaning | Typical realization |
|---|---|---|
| `passive` | Mechanical compliance — the effector yields physically with no control loop. | springs, elastic materials, series-elastic actuators |
| `active` | Active closed-loop force control — force/torque sensing regulates the contact force. | F/T sensor + force-control loop |
| `virtual` (virtual force control, VFC) | Compliance *rendered* by impedance / admittance control over position control — no direct force sensing required. | impedance / admittance control |

#### Declaration vs. request — two levels

Compliance appears at two levels, and they are not the same enumeration:

- **Capability declaration** (manifest auxiliary `compliance`): the *set* of modes the embodiment provides, each ∈ `{passive, active, virtual}`.
- **Primitive request** (the `compliance` parameter on each `force` primitive, typed `{passive, active, auto}`): the mode the primitive asks for. `auto` lets the Translation Layer pick any declared mode; an explicit request must name a declared mode, else `capability_absent`.

The primitive parameter enum omits `virtual`; the capability domain `{passive, active, virtual}` is the authority, and aligning the parameter enum is a pre-freeze formatting pass (as with the grasp-mode key spellings), not a semantic change.

#### Normal-direction compliance and contour following

`force.wipe` and `force.scrub` use hybrid force/position control — force-controlled along the surface normal, position-controlled along the tangent — and require a distinct **normal-direction compliance** sub-capability. It carries one limit:

- **`embodiment.limits.normal_compliance_range`** (`Length`) — the maximum surface-height variation contour following can absorb while holding the normal force. A step or hole beyond this range causes loss of contact (`contact_lost`), which the primitive detects and handles rather than gouging.

An embodiment that does not declare normal-direction compliance may not assert `force.wipe` or `force.scrub`.

#### Capability gate (force specialization of the `capability_absent` rule)

- A `force` primitive requiring compliance is rejected at validation (`capability_absent`, no attempt) on a rigid-only embodiment.
- `force.wipe` / `force.scrub` additionally require normal-direction compliance.
- `force.cut` additionally requires the tool-safety capability (defined in the safety-capability unit); the dependency is noted here only as a seam.

#### Deferred to other chapters

The compliance capability owns only the *declaration*. Three coupled concerns are owned elsewhere and referenced, not redefined here:

- **Hybrid-axis encoding** — which canonical-action axes are force-controlled vs. position-controlled — is owned by `02-translation-layer.md`.
- **The determinism boundary** — `retarget` generates the canonical action deterministically, but compliant search runs against contact dynamics, so the realized trajectory is not byte-for-byte reproducible — is owned by `02-translation-layer.md`.
- **The force-trajectory envelope class** — interval-sampled bounding of the force profile over the whole motion — is owned by `05-conformance.md`.

#### Conformance obligations (compliance)

- **CMP1c — mode demonstrability.** For each declared compliance mode, a `force` primitive requesting that mode passes its C1 test with the force trajectory within budget.
- **CMP2c — normal-compliance range.** When `force.wipe` / `force.scrub` is asserted, `normal_compliance_range` is present; a height step within range is absorbed (contour following succeeds), and a step beyond range produces a clean lift-off (no gouge).
- **CMP3c — rigid-only rejection.** An embodiment declaring no compliance mode rejects every compliance-requiring `force` primitive at validation, with no attempt.
- **CMP4c — undeclared-mode request.** An explicit `compliance` request naming an undeclared mode returns `capability_absent`.

## Collision model

Every motion primitive states its safety in terms of a clearance against a *static collision model* — `min_clearance(…, static_model) ≥ clearance`. This section defines what that model is, the query it must answer, and the obligations it carries. The model is not a Skill ISA capability flag: `reach.align` (a `reach`-family baseline primitive) already needs swept-volume clearance, so the full model is **mandatory** for every conformant embodiment, like the `reach` family itself.

### What RFL defines, and what it does not

RFL does **not** define the environment / world model (the geometry of obstacles in the scene). World models and perception are out of scope (`00-overview.md`). What RFL defines is:

1. A **clearance query** the driver exposes (one of the interfaces under § Scope item 1) — a deterministic service that, given a moving body, a motion, and an exemption set, returns the minimum clearance over that motion against the model.
2. The embodiment's **self-collision geometry** — declared with the standard URDF / MJCF `<collision>` elements. RFL adds no new block for it.

The environment model is supplied to the query as an external input (perception-fed). RFL's contract is the *interface and its guarantees*, not the scene contents — preserving layer discipline (Principle 4).

### The clearance query

```
clearance(
    moving:   MovingSet,           # the body / bodies that move
    motion:   MotionQuery,         # the geometry swept
    exempt:   set<ModelElement>,   # static-model elements excluded (intended proximity)
    augment:  set<ModelElement>,   # extra obstacles added to the static model
) -> Length                        # min clearance over the motion; the caller requires ≥ clearance
```

The query is **deterministic**: identical `(moving, motion, exempt, augment, model)` inputs return an identical clearance value. This is binding because the `no_collision_free_path` decisions it drives are compared against frozen conformance fixtures (`02` retargeting determinism, `05`).

The query evaluates the moving set against the environment model, the elements in `augment`, **and** the embodiment's own self-geometry (the links not in the moving set) — minus the elements in `exempt`.

### Moving set

Which bodies sweep depends on the primitive and the grasp state:

| `MovingSet` | Contents | Primitives |
|---|---|---|
| `effector(frame)` | the control frame's own collision geometry | `reach.*`, `grasp.*` final approach, `grasp.release` withdraw |
| `effector_with_held(frame, grasp)` | effector ∪ the held object's geometry (`GraspState`'s `ObjectTarget.geometry`, placed at the grasp pose) | all `transport.*`, `place` descent |

A held object's swept volume is inflated by the grasp's `StabilityMetadata.residual_mobility` when set: an `envelope_cage` grasp lets the object shift within `cage_clearance`, so the conservative swept volume must cover that freedom. This is the collision model's connection point to the grasp state model.

The just-released object in `grasp.release` is **not** a moving body; it is handled through `augment` (below).

### Motion query

The query answers three motion geometries, **all mandatory**:

| `MotionQuery` | Geometry | Required by |
|---|---|---|
| `at_pose(pose)` | clearance of the moving set at a single configuration | `reach.to_pose` validation |
| `corridor(from, to)` | the straight-line translation corridor, inflated by `clearance` | `reach.approach` / `hover` / `retract` |
| `swept(trajectory \| rotation)` | the full swept volume of an in-place rotation or an arbitrary trajectory | `reach.align`, `reach.scan`, all `transport.*` |

`corridor` is the translation-only special case; it is kept distinct from `swept` to match the primitive prose ("corridor" vs. "swept volume"), but an embodiment must answer all three. There is no "point/corridor-only" conformance tier — swept-volume support is required.

### Exemption and augmentation

Intended proximity to the manipulation target must not register as a collision, so each primitive passes the model elements it is intentionally approaching as `exempt`. The canonical designators:

| `exempt` element | Primitives |
|---|---|
| `target` | `reach.approach` / `hover`, all `grasp.*`, `place` approaches |
| `region` | `reach.scan` |
| `against_surface` (with `target`) | `grasp.pin` |
| `setdown_surface` | `transport.lower` |
| `support_object` | `place.stack` |

`augment` is the dual. When `grasp.release` opens the grasp, the freed object is no longer the manipulation target — it becomes an ordinary obstacle the withdrawing effector must not knock (`min_clearance(controlled_frame, static_model ∪ {released object})`). Over the grasp lifecycle an object transitions from *active target / held* (exempt) to *ordinary obstacle* (augmented, then part of the updated static model). The world-state tracking that records this transition is owned by `01-skill-isa.md`; the collision model only consumes the resulting exempt / augment sets.

### Conformance obligations (collision)

- **X1c — three geometries.** The driver returns the correct minimum clearance for `at_pose`, `corridor`, and `swept` queries (verified against bench-known obstacles).
- **X2c — held-object inclusion.** Under an active grasp, a `swept` query includes the held object's geometry — a trajectory that drives the held object into an obstacle is detected as an intrusion, not passed.
- **X3c — exemption correctness.** Elements in `exempt` are excluded from the clearance result; everything else (including other objects) is not.
- **X4c — augmentation (freed object).** A `grasp.release` withdraw treats the released object as an obstacle and detects a withdraw path that would strike it as `blocked`.
- **X5c — self-collision.** A swept motion that self-collides (e.g. an in-place rotation folding the arm into itself) is detected as `no_collision_free_path`.
- **X6c — determinism.** Identical `(moving, motion, exempt, augment, model)` inputs yield an identical clearance value (fixture reproducibility).

## Open issues

- Capability negotiation timing — **static manifest portion resolved** (§ Capability manifest: declared in `<rfl:capabilities>`, acquired at load-time / negotiation; checked per-primitive at validation). Open: per-action *dynamic* renegotiation (session-start vs. per-action) for embodiments whose capabilities change at runtime.
- Backward compatibility with ROS 2 action server conventions for non-RFL clients
- Real-time guarantee scope (best-effort vs. hard deadline)
