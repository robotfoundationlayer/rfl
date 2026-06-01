# rfl_msgs — ROS 2 interface definitions

The ROS 2 interface package for the RFL Driver Interface (`spec/03` § Canonical
driver messages). It realizes the canonical-message → ROS 2 mapping that
[`rfl-conformance::ros2`](../../../crates/rfl-conformance/src/ros2.rs) classifies
and the [design doc](../../../docs/design/2026-06-01-ros2-transport-binding-design.md)
specifies:

| Canonical message | ROS 2 interface |
|---|---|
| `execute` (goal) | `rfl_msgs/action/Execute` goal |
| `telemetry` (stream) | `rfl_msgs/action/Execute` feedback **and** the `rfl_msgs/msg/Telemetry` topic |
| `status` (terminal) | `rfl_msgs/action/Execute` result |
| `clearance` request/response | `rfl_msgs/srv/ClearanceQuery` |

## Design: typed protocol fields, JSON-floored payloads

The `spec/03`-owned **protocol-level** fields (`action_id`, `t`, `outcome`,
`failure_class`, `clearance`, `result`, `fidelity_tier`, …) are typed ROS 2
fields. The **representation-owned** sub-payloads — the `CanonicalAction`
(`spec/02`), `Pose6D` / `Wrench` (`spec/02` / `spec/04`), tactile readings,
`ForceEvent`s, `safety_flags`, `contact_geometry` — are deliberately *floored* by
`schemas/driver-interface.schema.json` (the spec does not re-encode the
representations other chapters own). So they ride here as **canonical JSON
strings** (`*_json` fields) rather than being re-encoded into ROS IDL, keeping
the ROS binding faithful to the normative payload tables instead of duplicating
— and possibly drifting from — `02`/`04`'s representations.

## Building (requires a ROS 2 environment)

```bash
# In a ROS 2 (Humble / Jazzy) workspace:
colcon build --packages-select rfl_msgs
```

This generates the language bindings (`rfl_msgs/action/Execute`, etc.) for any
ROS 2 client. **The live node** that publishes/subscribes/serves these over DDS
(an `rclrs` driver-side server + planner-side client) needs a ROS 2 toolchain
and is **not** built here — that is the remaining ROS-environment-gated leg
(`docs/design/2026-06-01-ros2-transport-binding-design.md` § BLOCKED). The
interface definitions in this package are the contract that node implements.

## Verifying without ROS 2

The interface files are plain text; `rfl-conformance::ros2` unit-tests the
canonical-message → interface classification against the committed
driver-message reference instances, with no ROS install. A driver author can
serialize an RFL telemetry/status payload to the `*_json` fields directly from
`schemas/driver-interface.schema.json`.
