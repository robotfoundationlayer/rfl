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
    AlignSpec, CanonicalAction, Envelope, Monitor, MotionBounds, PoseExpr, ProxySpec, SweepPose,
    TactileTargetOut, TimingHints, TimingMode,
};
use crate::embodiment::Embodiment;
use crate::quantity::Quantity;
use crate::skill_isa::{
    Axes, Axis, Compliance, ForceInsertFit, GraspPinch, GraspRelease, Primitive, ReachAlign,
    ReachRetract, ReachScan, ScanPattern, SenseInspect, Skill, Statement, TactileTargetArg,
    TransportMoveToPose,
};
use crate::grasp_force::{self, GraspMode};
use std::collections::BTreeMap;

/// The retargeting result: the canonical action stream plus the per-action
/// primitive suffix used to build deterministic action ids.
#[derive(Debug, Clone)]
pub struct RetargetOutput {
    /// The emitted canonical actions, in skill order.
    pub actions: Vec<CanonicalAction>,
    /// The primitive suffix for each action (for the action id).
    pub suffixes: Vec<&'static str>,
}

/// The object currently held by the active grasp, carried across the sequence walk
/// so a later `transport` / `force` primitive can derive mass-dependent bounds
/// (`spec/02` GF2c / GF3c). Set when a grasp closes, cleared when it releases.
#[derive(Debug, Clone)]
struct HeldObject {
    /// Held-object weight in newtons (`target.estimated_mass`).
    weight_n: f64,
    /// The closure mode of the active grasp.
    mode: GraspMode,
}

/// Mutable grasp state threaded through the retarget sequence walk.
#[derive(Debug, Clone, Default)]
struct GraspContext {
    /// The object currently held, if any.
    held: Option<HeldObject>,
}

/// Resolve each let-variable bound from a `sense.locate` to the declared
/// `estimated_mass` of its target object, giving `let-var -> weight`. A declared
/// prior, never runtime-measured (RD1c), so a grasp targeting such a variable can
/// look up the held-object weight at retarget time.
fn build_weights(skill: &Skill) -> BTreeMap<String, Quantity> {
    let mut weights = BTreeMap::new();
    for stmt in &skill.body.sequence {
        if let Statement::LetBind(b) = stmt {
            if let Primitive::SenseLocate(sl) = b.from.as_ref() {
                if let Some(decl) = skill.objects.get(&sl.target_ref) {
                    if let Some(mass) = &decl.estimated_mass {
                        weights.insert(b.r#let.clone(), mass.clone());
                    }
                }
            }
        }
    }
    weights
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
    let weights = build_weights(skill);
    let mut ctx = GraspContext::default();
    for stmt in &skill.body.sequence {
        let prim = match stmt {
            Statement::Primitive(p) => p,
            Statement::LetBind(b) => &b.from,
        };
        check_capability(prim, embodiment)?;
        let (action, suffix) = lower(prim, embodiment, &mut ctx, &weights);
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
        Primitive::ReachAlign(_) | Primitive::ReachRetract(_) | Primitive::ReachScan(_) => {
            return Ok(())
        }
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
        Primitive::SenseInspect(_) => "sense.inspect",
    };
    if e.has_skill(key) {
        Ok(())
    } else {
        Err(crate::Error::Translation(format!("capability_absent: {key}")))
    }
}

/// Lower one primitive to a canonical action and its action-id suffix. Every
/// cable-insertion primitive is handled, so lowering is infallible (capability
/// errors are raised earlier by `check_capability`).
fn lower(
    prim: &Primitive,
    e: &Embodiment,
    ctx: &mut GraspContext,
    weights: &BTreeMap<String, Quantity>,
) -> (CanonicalAction, &'static str) {
    match prim {
        Primitive::SenseLocate(p) => (lower_sense_locate(p, e), "locate"),
        Primitive::GraspPinch(p) => (lower_grasp_pinch(p, e, ctx, weights), "pinch"),
        Primitive::TransportMoveToPose(p) => (lower_transport_move_to_pose(p, e, ctx), "transport"),
        Primitive::ReachAlign(p) => (lower_reach_align(p, e), "align"),
        Primitive::ForceInsertFit(p) => (lower_force_insert_fit(p, e), "insert_fit"),
        Primitive::GraspRelease(p) => (lower_grasp_release(p, e, ctx), "release"),
        Primitive::ReachRetract(p) => (lower_reach_retract(p, e), "retract"),
        Primitive::ReachScan(p) => (lower_reach_scan(p, e), "scan"),
        Primitive::SenseInspect(p) => (lower_sense_inspect(p, e), "inspect"),
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

/// Lower `grasp.pinch`. force_budget is clamped to grip_force_max (CA4c) and floored
/// at the GF1c static minimum derived from the declared held weight; the floor is also
/// emitted in force_profile (the grasp-continuity invariant `05` GC1 samples it). The
/// tactile_target `auto` is kept (manifold tier) on a tactile embodiment and degraded
/// to the force/position proxy when tactile_sensing is undeclared (`spec/04`
/// § Graceful degradation). The held object is recorded for the downstream transport /
/// force derivations (GF2c / GF3c).
fn lower_grasp_pinch(
    p: &GraspPinch,
    e: &Embodiment,
    ctx: &mut GraspContext,
    weights: &BTreeMap<String, Quantity>,
) -> CanonicalAction {
    let mut force_budget = clamp_force(&p.force_budget, "grip_force_max", e);
    let tactile_target = Some(match (&p.tactile_target, e.tactile_sensing()) {
        (TactileTargetArg::Auto(_), true) => TactileTargetOut::Auto,
        (TactileTargetArg::Auto(_), false) => TactileTargetOut::Proxy {
            proxy: ProxySpec { tier: "proxy", criterion: "position_convergence_and_force_hold" },
        },
        (TactileTargetArg::Other(v), _) => {
            TactileTargetOut::Explicit(serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
        }
    });
    let mut env = base_envelope(e);
    if let Some((weight_n, _)) = weights.get(&p.target).and_then(|q| q.parse()) {
        let mhf = grasp_force::min_holding_force(weight_n, GraspMode::Pinch);
        env.force_profile =
            Some(serde_json::json!({ "min_holding_force": Quantity::from_si(mhf, "N").0 }));
        if let Some((fb, unit)) = force_budget.parse() {
            if mhf > fb {
                let unit = unit.to_string();
                force_budget = Quantity::from_si(mhf, &unit);
            }
        }
        ctx.held = Some(HeldObject { weight_n, mode: GraspMode::Pinch });
    }
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: Some(force_budget),
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target,
        monitors: vec![],
        safety_envelope: env,
    }
}

/// Lower `transport.move_to_pose`. The frame-relative target is carried through;
/// a_max is clamped to the GF2c dynamic-stability limit when a held object makes it
/// tighter than the kinematic ceiling `base_envelope` set (otherwise the ceiling's
/// authored string is kept verbatim — no reformat).
fn lower_transport_move_to_pose(
    p: &TransportMoveToPose,
    e: &Embodiment,
    ctx: &GraspContext,
) -> CanonicalAction {
    let target_pose = match yaml_to_json(&p.target_pose) {
        serde_json::Value::Object(map) => {
            let frame = map
                .get("frame")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("task")
                .to_string();
            let offset = map.get("offset").cloned().unwrap_or(serde_json::Value::Null);
            PoseExpr::FrameRelative { frame, offset }
        }
        other => PoseExpr::FrameRelative { frame: "task".into(), offset: other },
    };
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
        let ceiling = e.scalar_limit("a_cartesian_max").and_then(|q| q.parse());
        if let (Some((payload_n, _)), Some((ceiling_v, unit))) = (payload, ceiling) {
            let unit = unit.to_string();
            let dyn_a = grasp_force::dynamic_a_max(held.weight_n, payload_n);
            if dyn_a < ceiling_v {
                env.motion_bounds.a_max = Some(Quantity::from_si(dyn_a, &unit));
            }
        }
    }
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose,
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: env,
    }
}

/// Lower `reach.align`: orientation-only; the residual is the minimum geodesic
/// rotation from the current orientation (`spec/02` CA2c), emitted as a directive.
fn lower_reach_align(p: &ReachAlign, e: &Embodiment) -> CanonicalAction {
    let axes = match &p.axes {
        Axes::All(_) => vec!["x".into(), "y".into(), "z".into()],
        Axes::Set(v) => v
            .iter()
            .map(|a| {
                match a {
                    Axis::X => "x",
                    Axis::Y => "y",
                    Axis::Z => "z",
                }
                .to_string()
            })
            .collect(),
    };
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::OrientationAlign {
            align: AlignSpec {
                target_frame: p.target_frame.clone(),
                axes,
                residual: "min_geodesic_rotation",
            },
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Convert a `serde_yaml::Value` to a `serde_json::Value` deterministically.
fn yaml_to_json(v: &serde_yaml::Value) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// Lower `force.insert_fit`: carry the axial force_budget, lower the SeatingSpec
/// stop_condition into a monitor, set compliance and the axial force_profile. The
/// reaction-load bound is symbolic in v0 (design § 3).
fn lower_force_insert_fit(p: &ForceInsertFit, e: &Embodiment) -> CanonicalAction {
    let force_budget = Some(p.force_budget.clone()); // axial fit force; no grip-mode limit applies
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.stop_condition) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    env.force_profile = Some(serde_json::json!({ "axial": p.force_budget.0.clone() }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_fit.clone() },
        force_budget,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::TimeScalable,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors,
        safety_envelope: env,
    }
}

/// Lower `grasp.release`: clears the active grasp from the context and emits a
/// zero-distance withdraw along the default retract direction. The break-contact
/// postcondition (`spec/01`) is symbolic in v0.
fn lower_grasp_release(_p: &GraspRelease, e: &Embodiment, ctx: &mut GraspContext) -> CanonicalAction {
    ctx.held = None;
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: serde_json::Value::String("-tool_axis".into()),
            distance: Quantity("0 mm".into()),
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Lower `reach.retract`: an axis-relative withdraw (direction + distance). The
/// force-monotonicity abort is symbolic in v0.
fn lower_reach_retract(p: &ReachRetract, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.direction),
            distance: p.distance.clone(),
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Parse a length quantity to metres (m / mm / cm). Returns 0.0 on a malformed value
/// (v0 scan regions are author-declared, not referenced).
fn length_m(q: &Quantity) -> f64 {
    match q.parse() {
        Some((v, "m")) => v,
        Some((v, "mm")) => v / 1000.0,
        Some((v, "cm")) => v / 100.0,
        _ => 0.0,
    }
}

/// Parse an angle quantity to radians (deg / rad).
fn angle_rad(q: &Quantity) -> f64 {
    match q.parse() {
        Some((v, "deg")) => v.to_radians(),
        Some((v, "rad")) => v,
        _ => 0.0,
    }
}

/// Lower `reach.scan`: compile the region into the sweep set Σ and emit one action
/// carrying it as a `SweepPath`. reach.* is the baseline (no capability gate); the
/// FOV comes from the embodiment sensor descriptor.
fn lower_reach_scan(p: &ReachScan, e: &Embodiment) -> CanonicalAction {
    let sensor_frame = p
        .sensor_frame
        .clone()
        .unwrap_or_else(|| e.sensor_frame().to_string());
    let pattern = p.pattern.unwrap_or(ScanPattern::Raster);
    let standoff = length_m(&p.standoff);
    let overlap = p.coverage_overlap.unwrap_or(0.0);

    let poses = match &p.region {
        crate::region::ScanRegion::Waypoints { poses, .. } => {
            let wps: Vec<([f64; 3], [f64; 4])> =
                poses.iter().map(|w| (w.position, w.orientation)).collect();
            crate::sigma::waypoints(&wps)
        }
        // Surface region: raster (spiral/arc fall back to raster in v0).
        crate::region::ScanRegion::Surface { size_u, size_v, .. } => {
            let (h, v) = e
                .sensor_fov(&sensor_frame)
                .map(|f| (angle_rad(&f.h_angle), angle_rad(&f.v_angle)))
                .unwrap_or((0.0, 0.0));
            crate::sigma::raster(length_m(size_u), length_m(size_v), standoff, overlap, h, v)
        }
    };

    let pattern_name = match pattern {
        ScanPattern::Raster => "raster",
        ScanPattern::Waypoints => "waypoints",
        ScanPattern::Spiral => "spiral",
        ScanPattern::Arc => "arc",
    };
    let sweep_poses: Vec<SweepPose> = poses.iter().map(SweepPose::from_pose).collect();
    CanonicalAction {
        target_frame: sensor_frame,
        target_pose: PoseExpr::SweepPath { pattern: pattern_name.to_string(), poses: sweep_poses },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: base_envelope(e),
    }
}

/// Lower `sense.inspect`: a perception action observing the target (like sense.locate).
fn lower_sense_inspect(p: &SenseInspect, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.sensor_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
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
    fn full_skill_retargets_to_eight_actions() {
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.actions.len(), 8);
        assert_eq!(
            out.suffixes,
            vec![
                "locate", "pinch", "transport", "locate", "align", "insert_fit", "release",
                "retract"
            ]
        );
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
        let weights = super::build_weights(skill);
        let mut ctx = super::GraspContext::default();
        super::lower_grasp_pinch(p, emb, &mut ctx, &weights)
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

    #[test]
    fn transport_carries_frame_relative_pose() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::TransportMoveToPose(p)) = &skill.body.sequence[2] else {
            panic!("expected transport.move_to_pose at index 2");
        };
        let ctx = super::GraspContext::default();
        let a = super::lower_transport_move_to_pose(p, &emb, &ctx);
        let json = serde_json::to_string(&a.target_pose).unwrap();
        assert!(json.contains("\"frame\":\"receptacle\""));
        // no held object in this isolated call -> kinematic ceiling kept.
        assert_eq!(a.safety_envelope.motion_bounds.a_max.as_ref().unwrap().0, "1.5 m/s^2");
    }

    #[test]
    fn align_emits_residual_directive() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::ReachAlign(p)) = &skill.body.sequence[4] else {
            panic!("expected reach.align at index 4");
        };
        let a = super::lower_reach_align(p, &emb);
        let json = serde_json::to_string(&a.target_pose).unwrap();
        assert!(json.contains("min_geodesic_rotation"));
        assert!(json.contains("\"target_frame\":\"receptacle\""));
        assert!(json.contains("\"z\""));
    }

    #[test]
    fn insert_fit_lowers_stop_condition_into_monitors() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::ForceInsertFit(p)) = &skill.body.sequence[5] else {
            panic!("expected force.insert_fit at index 5");
        };
        let a = super::lower_force_insert_fit(p, &emb);
        assert_eq!(a.force_budget.as_ref().unwrap().0, "15 N");
        assert_eq!(a.safety_envelope.compliance.as_deref(), Some("active"));
        let monitors = serde_json::to_string(&a.monitors).unwrap();
        assert!(monitors.contains("all_of"));
        assert!(monitors.contains("effort_rise"));
        assert!(monitors.contains("depth"));
    }

    #[test]
    fn retract_is_axis_relative() {
        let (skill, emb) = load("allegro");
        let Statement::Primitive(Primitive::ReachRetract(p)) = &skill.body.sequence[7] else {
            panic!("expected reach.retract at index 7");
        };
        let a = super::lower_reach_retract(p, &emb);
        let json = serde_json::to_string(&a.target_pose).unwrap();
        assert!(json.contains("-tool_axis"));
        assert!(json.contains("50 mm"));
    }

    #[test]
    fn scan_lowers_to_sweep_path_per_fov() {
        let yaml = "skill: surface-scan\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: raster\n        coverage_overlap: 0.2\n    - sense.inspect: { target: panel, observe: [defect] }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let mut allegro = Embodiment::parse_yaml(&std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/01-cable-insertion/embodiments/allegro.yaml"),
        ).unwrap()).unwrap();
        // The 02 surface-scan descriptors declare sense.inspect; the 01 allegro does not.
        allegro.capabilities.skills.push("sense.inspect".to_string());
        let out = retarget(&skill, &allegro).expect("retarget");
        assert_eq!(out.actions.len(), 2);
        let crate::canonical::PoseExpr::SweepPath { poses, pattern } = &out.actions[0].target_pose
        else {
            panic!("expected SweepPath");
        };
        assert_eq!(pattern.as_str(), "raster");
        assert_eq!(poses.len(), 9); // allegro palm_cam 60x45 -> 9 sweep poses
        assert_eq!(out.suffixes, vec!["scan", "inspect"]);
    }

    #[test]
    fn pinch_emits_min_holding_force_floor() {
        let (skill, emb) = load("allegro");
        let out = retarget_pinch_only(&skill, &emb);
        let fp = serde_json::to_string(&out.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"min_holding_force\":\"2.9 N\""), "got {fp}");
        // 8 N task budget exceeds the 2.9 N floor and is under the 20 N ceiling -> unchanged.
        assert_eq!(out.force_budget.as_ref().unwrap().0, "8 N");
    }

    #[test]
    fn release_clears_held_context() {
        let (skill, emb) = load("allegro");
        let weights = super::build_weights(&skill);
        let mut ctx = super::GraspContext::default();
        let Statement::Primitive(Primitive::GraspPinch(p)) = &skill.body.sequence[1] else {
            panic!("expected grasp.pinch at index 1");
        };
        super::lower_grasp_pinch(p, &emb, &mut ctx, &weights);
        assert!(ctx.held.is_some());
        let Statement::Primitive(Primitive::GraspRelease(r)) = &skill.body.sequence[6] else {
            panic!("expected grasp.release at index 6");
        };
        super::lower_grasp_release(r, &emb, &mut ctx);
        assert!(ctx.held.is_none());
    }

    #[test]
    fn transport_a_max_clamps_dynamically_on_weakest_hand() {
        // pneumatic: payload 1.5 N, held 1.45 N -> 9.80665/29 ≈ 0.33816 < 0.8 ceiling.
        let (skill, emb) = load("pneumatic-6f");
        let out = retarget(&skill, &emb).expect("retarget");
        // transport is action index 2 (locate, pinch, transport).
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "0.33816 m/s^2"
        );
    }

    #[test]
    fn transport_a_max_keeps_kinematic_ceiling_on_strong_hand() {
        // allegro: payload 3 N, held 1.45 N -> dynamic ≈ 10.5 > 1.5 ceiling (kept verbatim).
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "1.5 m/s^2"
        );
    }
}
