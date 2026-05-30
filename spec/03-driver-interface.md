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

## Open issues

- Capability negotiation timing (load-time vs. session-start vs. per-action)
- Backward compatibility with ROS 2 action server conventions for non-RFL clients
- Real-time guarantee scope (best-effort vs. hard deadline)
