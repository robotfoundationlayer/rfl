// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Translation Layer.
//!
//! Resolves high-level Skill ISA compositions into per-embodiment canonical
//! actions via a deterministic retargeting algorithm. Holds the canonical
//! embodiment-agnostic action representation that all driver implementations
//! consume.
//!
//! See `spec/02-translation-layer.md` for the formal definition.

use crate::canonical::{
    CanonicalAction, Envelope, MotionBounds, PoseExpr, ProxySpec, TactileTargetOut, TimingHints,
    TimingMode,
};
use crate::embodiment::Embodiment;
use crate::quantity::Quantity;
use crate::skill_isa::{GraspPinch, Primitive, Skill, Statement, TactileTargetArg};

/// The retargeting result: the canonical action stream plus the per-action
/// primitive suffix used to build deterministic action ids.
#[derive(Debug, Clone)]
pub struct RetargetOutput {
    /// The emitted canonical actions, in skill order.
    pub actions: Vec<CanonicalAction>,
    /// The primitive suffix for each action (for the action id).
    pub suffixes: Vec<&'static str>,
}

/// Retarget a skill onto an embodiment (`spec/02`). Deterministic for identical
/// inputs (RD1c): embodiment-dependent values are resolved now; runtime-measured
/// poses stay symbolic.
///
/// # Errors
/// Returns `Error::Translation("capability_absent: <key>")` when a primitive's gate
/// key is absent from the descriptor's capability manifest.
pub fn retarget(skill: &Skill, embodiment: &Embodiment) -> crate::Result<RetargetOutput> {
    let mut actions = Vec::new();
    let mut suffixes = Vec::new();
    for stmt in &skill.body.sequence {
        let prim = match stmt {
            Statement::Primitive(p) => p,
            Statement::LetBind(b) => &b.from,
        };
        check_capability(prim, embodiment)?;
        let (action, suffix) = lower(prim, embodiment)?;
        actions.push(action);
        suffixes.push(suffix);
    }
    Ok(RetargetOutput { actions, suffixes })
}

/// The uniform `capability_absent` gate (`spec/03` § Capability checking). The key
/// each primitive maps to follows § Primitive capabilities: `reach.*` is the
/// unkeyed baseline; a category key (`transport`) implies its base primitive; a
/// grasp mode is its own `grasp.<mode>` key. `tactile_sensing` is preferred, never
/// required (G4c), so it is not gated here.
fn check_capability(prim: &Primitive, e: &Embodiment) -> crate::Result<()> {
    let key: &str = match prim {
        // reach.* is the unkeyed mandatory baseline: no gate.
        Primitive::ReachAlign(_) | Primitive::ReachRetract(_) => return Ok(()),
        // grasp.release is presupposed by any declared grasp capability (spec/03
        // § Grasp-mode capabilities lists only the eight modes; the descriptors do
        // not declare grasp.release). Require at least one grasp.* mode.
        Primitive::GraspRelease(_) => {
            return if e.capabilities.skills.iter().any(|s| s.starts_with("grasp.")) {
                Ok(())
            } else {
                Err(crate::Error::Translation("capability_absent: grasp".into()))
            };
        }
        Primitive::SenseLocate(_) => "sense.locate",
        Primitive::GraspPinch(_) => "grasp.pinch",
        // A category key implies the base primitive: `transport` = transport.move_to_pose.
        Primitive::TransportMoveToPose(_) => "transport",
        Primitive::ForceInsertFit(_) => "force.insert_fit",
    };
    if e.has_skill(key) {
        Ok(())
    } else {
        Err(crate::Error::Translation(format!("capability_absent: {key}")))
    }
}

/// Lower one primitive to a canonical action. Each primitive arm is implemented in
/// its own task; this skeleton implements sense.locate and routes the rest to a
/// not-yet-implemented error so the engine compiles incrementally.
fn lower(prim: &Primitive, e: &Embodiment) -> crate::Result<(CanonicalAction, &'static str)> {
    match prim {
        Primitive::SenseLocate(p) => Ok((lower_sense_locate(p, e), "locate")),
        Primitive::GraspPinch(p) => Ok((lower_grasp_pinch(p, e), "pinch")),
        other => Err(crate::Error::Translation(format!(
            "lowering not yet implemented for {}",
            primitive_name(other)
        ))),
    }
}

fn primitive_name(p: &Primitive) -> &'static str {
    match p {
        Primitive::SenseLocate(_) => "sense.locate",
        Primitive::GraspPinch(_) => "grasp.pinch",
        Primitive::TransportMoveToPose(_) => "transport.move_to_pose",
        Primitive::ReachAlign(_) => "reach.align",
        Primitive::ForceInsertFit(_) => "force.insert_fit",
        Primitive::GraspRelease(_) => "grasp.release",
        Primitive::ReachRetract(_) => "reach.retract",
    }
}

/// `sense.locate` lowers to a perception action: the sensor acquires the named
/// object's pose; the result binds the enclosing let-variable at runtime. The
/// emitted target_pose references the object identity (symbolic, runtime-resolved).
fn lower_sense_locate(p: &crate::skill_isa::SenseLocate, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.sensor_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_ref.clone() },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: Envelope {
            motion_bounds: MotionBounds::default(),
            force_profile: None,
            clearance: None,
            compliance: None,
            stop_time: None,
        },
    }
}

/// Clamp a force quantity to a descriptor scalar limit by magnitude, emitting the
/// smaller as a string (CA4c). Both are assumed to share a unit (N for grip force);
/// if the limit is absent or non-scalar the value passes through.
fn clamp_force(value: &Quantity, limit_key: &str, e: &Embodiment) -> Quantity {
    let Some(limit) = e.scalar_limit(limit_key) else {
        return value.clone();
    };
    match (value.parse(), limit.parse()) {
        (Some((v, vu)), Some((l, lu))) if vu == lu && l < v => limit.clone(),
        _ => value.clone(),
    }
}

/// A baseline envelope with motion bounds clamped to the embodiment's reach limits
/// and stop_time taken from the descriptor (CA4c). Force-specific fields are added
/// per-primitive.
fn base_envelope(e: &Embodiment) -> Envelope {
    Envelope {
        motion_bounds: MotionBounds {
            v_max: e.scalar_limit("v_cartesian_max").cloned(),
            a_max: e.scalar_limit("a_cartesian_max").cloned(),
            w_max: e.scalar_limit("w_cartesian_max").cloned(),
        },
        force_profile: None,
        clearance: None,
        compliance: None,
        stop_time: e.scalar_limit("stop_time").cloned(),
    }
}

/// Lower `grasp.pinch`. force_budget is clamped to grip_force_max (CA4c). The
/// tactile_target `auto` is kept (manifold tier) on a tactile embodiment and
/// degraded to the force/position proxy when tactile_sensing is undeclared
/// (`spec/04` § Graceful degradation: position-convergence ∧ force-rise-and-hold,
/// disclosed at the proxy fidelity tier).
fn lower_grasp_pinch(p: &GraspPinch, e: &Embodiment) -> CanonicalAction {
    let force_budget = Some(clamp_force(&p.force_budget, "grip_force_max", e));
    let tactile_target = Some(match (&p.tactile_target, e.tactile_sensing()) {
        (TactileTargetArg::Auto(_), true) => TactileTargetOut::Auto,
        (TactileTargetArg::Auto(_), false) => TactileTargetOut::Proxy {
            proxy: ProxySpec { tier: "proxy", criterion: "position_convergence_and_force_hold" },
        },
        (TactileTargetArg::Other(v), _) => {
            TactileTargetOut::Explicit(serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        }
    });
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embodiment::Embodiment;
    use crate::skill_isa::Skill;
    use std::path::Path;

    fn load(stem: &str) -> (Skill, Embodiment) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/01-cable-insertion");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(root.join("skill.yaml")).unwrap()).unwrap();
        let emb = Embodiment::parse_yaml(
            &std::fs::read_to_string(root.join(format!("embodiments/{stem}.yaml"))).unwrap(),
        )
        .unwrap();
        (skill, emb)
    }

    #[test]
    #[ignore = "un-ignored in Task 10 once all primitives lower"]
    fn retarget_emits_one_action_per_motion_statement() {
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.actions.len(), 8);
    }

    #[test]
    fn capability_absent_when_gate_key_missing() {
        let (skill, mut emb) = load("allegro");
        emb.capabilities.skills.retain(|s| s != "grasp.pinch");
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(matches!(err, crate::Error::Translation(_)));
        assert!(err.to_string().contains("capability_absent"));
        assert!(err.to_string().contains("grasp.pinch"));
    }

    fn retarget_pinch_only(skill: &Skill, emb: &Embodiment) -> CanonicalAction {
        let Statement::Primitive(Primitive::GraspPinch(p)) = &skill.body.sequence[1] else {
            panic!("expected grasp.pinch at index 1");
        };
        super::lower_grasp_pinch(p, emb)
    }

    #[test]
    fn pinch_clamps_force_and_keeps_manifold_tier_on_allegro() {
        let (skill, emb) = load("allegro");
        let out = retarget_pinch_only(&skill, &emb);
        assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N"); // 8 <= grip_force_max 20
        assert!(matches!(out.tactile_target, Some(TactileTargetOut::Auto)));
    }

    #[test]
    fn pinch_degrades_to_proxy_on_pneumatic() {
        let (skill, emb) = load("pneumatic-6f");
        let out = retarget_pinch_only(&skill, &emb);
        assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N"); // 8 <= grip_force_max 12
        assert!(matches!(out.tactile_target, Some(TactileTargetOut::Proxy { .. })));
    }
}
