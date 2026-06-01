# ROS 2 transport binding — message-mapping design + pure-Rust shim

Status: **INTERFACE-MAPPING CORE IMPLEMENTED 2026-06-02** — `rfl-conformance::ros2`
classifies each canonical message (`execute` / `telemetry` / `status` /
`clearance` request+response) to its ROS 2 surface(s) per the mapping table
below (`ros2_surfaces_for` / `ros2_surfaces_of`), fail-closed on an unknown
message, verified against the five committed driver-message reference instances.
**Still BLOCKED on a ROS 2 environment:** the `rosidl`-generated `.msg`/`.action`/
`.srv` field layouts (redundant with `schemas/driver-interface`, which already
encodes the normative payloads) and the live `rclrs` node / DDS interop. This
core fixes the *routing*; those fix the *transport*.

## Context

The spec is **"ROS 2-compatible, not ROS 2-mandatory"** (`spec/03` § ROS 2
compatibility, line 710): the four canonical payloads — `execute`, `telemetry`,
`status`, and the `clearance` query/response — and their field tables are
normative; the ROS 2 binding is one transport over them. No real ROS 2 messages
or nodes exist yet (the I3 deferred leg). This designs the binding so the
**transport-independent mapping** can be built and tested without a ROS install,
and isolates exactly what needs a live ROS 2 environment.

## Message mapping (canonical payload → ROS 2 interface)

| Canonical payload | ROS 2 interface | Rationale |
|---|---|---|
| `execute` (goal) | an **action** goal (`rfl_msgs/action/Execute`) | a long-running, cancellable, feedback-bearing command |
| `telemetry` (stream) | the action **feedback** + a `rfl_msgs/msg/Telemetry` **topic** | high-rate interval samples on the manifold timebase |
| `status` (terminal) | the action **result** (`Execute.Result`) | three-valued outcome + audit record |
| `clearance` request/response | a **service** (`rfl_msgs/srv/ClearanceQuery`) | a synchronous request/response, exactly the spec's framing |

The `.msg` / `.action` / `.srv` field layouts are a 1:1 transcription of the
`spec/03` payload field tables (the normative source), so the IDL adds no
semantics — it only re-encodes the canonical structures in ROS 2's type system.

## The pure-Rust shim (implementable now, no ROS dependency)

A `bindings/ros2` crate (new, mine — distinct from the other-owned
`bindings/python`) that is **transport-agnostic**:

- defines plain Rust structs mirroring the `rfl_msgs` IDL field layouts;
- provides `to_ros2_execute(canonical_json) -> ExecuteGoal` and
  `from_ros2_telemetry(Telemetry) -> canonical_json` (and the inverse pair),
  reusing `rfl-core`'s canonical types so the mapping cannot drift from the wire;
- is fully unit-testable against the committed `examples/.../driver-messages/*`
  reference instances **without rclrs or a ROS install** — the shim is a pure
  data transform.

This gives a real, CI-testable artifact (the mapping correctness) while needing
zero ROS environment.

## What is BLOCKED on a ROS 2 environment

- The `rclrs` node that actually publishes/subscribes/serves over DDS.
- `.msg`/`.action`/`.srv` IDL generation via `rosidl` (needs a ROS 2 toolchain).
- Interop testing against a real ROS 2 node.

These need a ROS 2 install (Humble/Jazzy) and are **BLOCKED: ROS 2 environment**
(hardware/toolchain dependency), not design-open. The mapping above is the
contract they implement.

## Decision

The message mapping is fixed (this document). The pure-Rust shim is a clean,
ROS-independent, CI-testable increment ready to pick up. The live node + IDL
generation + DDS interop are BLOCKED on a ROS 2 environment and are scoped out
until that environment exists (and a session that owns the ROS bringup, parallel
to the Milchick hardware track).
