// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! ROS 2 interface mapping for the canonical driver messages (`spec/03` § ROS 2
//! compatibility; design doc `2026-06-01-ros2-transport-binding-design.md`).
//!
//! The spec is **ROS 2-compatible, not ROS 2-mandatory**: the four canonical
//! payloads (`execute` / `telemetry` / `status` / `clearance` query+response)
//! and their field tables are normative, and the ROS 2 binding is one transport
//! over them. This module is the transport-independent, dependency-free core of
//! that binding: it classifies each canonical message to the ROS 2 interface(s)
//! it rides on, per the design doc's mapping table. It needs no `rclrs` and no
//! ROS install, so it is CI-testable against the committed driver-message
//! reference instances.
//!
//! What is **deferred to a real ROS 2 environment** (BLOCKED, not design-open):
//! the `rosidl`-generated `.msg`/`.action`/`.srv` field layouts (a 1:1
//! transcription of the `spec/03` payload tables — which `schemas/driver-interface`
//! already encodes), the `rclrs` node that publishes/subscribes/serves over DDS,
//! and live interop. This module fixes the *routing*; those fix the *transport*.

/// A ROS 2 interface kind a canonical message rides on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ros2InterfaceKind {
    /// An action **goal** (a long-running, cancellable, feedback-bearing command).
    ActionGoal,
    /// An action **feedback** message (high-rate progress on the goal).
    ActionFeedback,
    /// An action **result** (the terminal outcome of the goal).
    ActionResult,
    /// A **topic** publication (a stream not tied to one goal's lifecycle).
    Topic,
    /// A **service** request (a synchronous query).
    ServiceRequest,
    /// A **service** response (the synchronous answer).
    ServiceResponse,
}

/// One ROS 2 surface a canonical message maps to: its interface kind and the
/// `rfl_msgs` interface that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ros2Surface {
    /// The ROS 2 interface kind.
    pub kind: Ros2InterfaceKind,
    /// The `rfl_msgs` interface (e.g. `rfl_msgs/action/Execute`).
    pub interface: &'static str,
}

/// Map a canonical driver message (its top-level `message` discriminator) to the
/// ROS 2 surface(s) it rides on (`spec/03` § ROS 2 compatibility). `telemetry`
/// maps to **two** surfaces (the action feedback and a standalone topic), as the
/// design-doc table specifies; the others map to one. Returns `None` for an
/// unknown message (the fail-closed default — never a silent misroute).
#[must_use]
pub fn ros2_surfaces_for(message: &str) -> Option<Vec<Ros2Surface>> {
    use Ros2InterfaceKind::{
        ActionFeedback, ActionGoal, ActionResult, ServiceRequest, ServiceResponse, Topic,
    };
    let s = |kind, interface| Ros2Surface { kind, interface };
    let surfaces = match message {
        "execute" => vec![s(ActionGoal, "rfl_msgs/action/Execute")],
        "telemetry" => vec![
            s(ActionFeedback, "rfl_msgs/action/Execute"),
            s(Topic, "rfl_msgs/msg/Telemetry"),
        ],
        "status" => vec![s(ActionResult, "rfl_msgs/action/Execute")],
        "clearance_request" => vec![s(ServiceRequest, "rfl_msgs/srv/ClearanceQuery")],
        "clearance_response" => vec![s(ServiceResponse, "rfl_msgs/srv/ClearanceQuery")],
        _ => return None,
    };
    Some(surfaces)
}

/// Classify a whole canonical message value (anything carrying a top-level
/// string `message` field) to its ROS 2 surface(s). Convenience over
/// [`ros2_surfaces_for`] for an already-parsed payload.
#[must_use]
pub fn ros2_surfaces_of(message_value: &serde_json::Value) -> Option<Vec<Ros2Surface>> {
    message_value
        .get("message")
        .and_then(serde_json::Value::as_str)
        .and_then(ros2_surfaces_for)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn load_message(stem: &str) -> serde_json::Value {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/01-cable-insertion/driver-messages")
            .join(format!("{stem}.yaml"));
        serde_yaml::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn every_reference_instance_maps_to_its_designed_surface() {
        use Ros2InterfaceKind::*;
        // The five committed driver-message reference instances classify exactly
        // as the design-doc mapping table prescribes.
        let cases: &[(&str, &[Ros2InterfaceKind])] = &[
            ("execute", &[ActionGoal]),
            ("telemetry", &[ActionFeedback, Topic]),
            ("status", &[ActionResult]),
            ("clearance-request", &[ServiceRequest]),
            ("clearance-response", &[ServiceResponse]),
        ];
        for (stem, expected_kinds) in cases {
            let surfaces = ros2_surfaces_of(&load_message(stem))
                .unwrap_or_else(|| panic!("{stem} did not classify"));
            let kinds: Vec<_> = surfaces.iter().map(|s| s.kind).collect();
            assert_eq!(&kinds, expected_kinds, "{stem} surfaces");
        }
    }

    #[test]
    fn execute_and_status_share_the_one_action_interface() {
        // execute (goal) and status (result) ride the same rfl_msgs/action/Execute
        // — the request/result halves of one action, as the table intends.
        let goal = ros2_surfaces_for("execute").unwrap()[0].interface;
        let result = ros2_surfaces_for("status").unwrap()[0].interface;
        let feedback = ros2_surfaces_for("telemetry").unwrap()[0].interface;
        assert_eq!(goal, "rfl_msgs/action/Execute");
        assert_eq!(result, goal);
        assert_eq!(feedback, goal); // feedback is the same action's feedback channel
    }

    #[test]
    fn unknown_message_is_fail_closed() {
        assert!(ros2_surfaces_for("not_a_message").is_none());
        assert!(ros2_surfaces_of(&serde_json::json!({"no": "message"})).is_none());
    }
}
