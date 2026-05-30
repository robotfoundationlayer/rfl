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

## Open issues

- Capability negotiation timing (load-time vs. session-start vs. per-action)
- Backward compatibility with ROS 2 action server conventions for non-RFL clients
- Real-time guarantee scope (best-effort vs. hard deadline)
