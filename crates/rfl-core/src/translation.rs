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
    Axes, Axis, Compliance, DisturbanceArg, ForceCut, ForceInsertFit, ForcePressButton, ForceScrew, ForceSnapEngage, ForceUnscrew, ForceWipe, GraspPinch, GraspRelease, InHandFlip,
    Primitive, ReachAlign, ReachHover, ReachRetract, ReachScan, ScanPattern, SenseInspect, Skill, StabilityMarginArg,
    Statement, TactileTargetArg, TransportCarry, TransportMoveToPose,
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
        Primitive::ReachAlign(_)
        | Primitive::ReachRetract(_)
        | Primitive::ReachScan(_)
        | Primitive::ReachHover(_) => {
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
        // transport.carry is a DISTINCT capability beyond the base transport gate
        // (§ 4.4 precondition: "declares transport with carry support").
        Primitive::TransportCarry(_) => "transport.carry",
        Primitive::ForceInsertFit(_) => "force.insert_fit",
        Primitive::ForceScrew(_) => "force.screw",
        Primitive::ForceUnscrew(_) => "force.unscrew",
        Primitive::ForcePressButton(_) => "force.press_button",
        Primitive::ForceWipe(_) => "force.wipe",
        Primitive::ForceSnapEngage(_) => "force.snap_engage",
        // force.cut is hazardous: it requires BOTH the force.cut skill AND a declared
        // tool_safety capability (spec/01 § 6.7 precondition). A conjunctive gate.
        Primitive::ForceCut(_) => {
            return if !e.has_skill("force.cut") {
                Err(crate::Error::Translation("capability_absent: force.cut".to_string()))
            } else if !e.has_tool_safety() {
                Err(crate::Error::Translation("capability_absent: tool_safety".to_string()))
            } else {
                Ok(())
            };
        }
        Primitive::InHandFlip(_) => "in_hand.flip",
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
        Primitive::TransportCarry(p) => (lower_transport_carry(p, e, ctx), "carry"),
        Primitive::ReachAlign(p) => (lower_reach_align(p, e), "align"),
        Primitive::ForceInsertFit(p) => (lower_force_insert_fit(p, e, ctx), "insert_fit"),
        Primitive::ForceScrew(p) => (lower_force_screw(p, e, ctx), "screw"),
        Primitive::ForceUnscrew(p) => (lower_force_unscrew(p, e, ctx), "unscrew"),
        Primitive::ForcePressButton(p) => (lower_force_press_button(p, e), "press_button"),
        Primitive::ForceWipe(p) => (lower_force_wipe(p, e), "wipe"),
        Primitive::ForceSnapEngage(p) => (lower_force_snap_engage(p, e), "snap_engage"),
        Primitive::ForceCut(p) => (lower_force_cut(p, e), "cut"),
        Primitive::InHandFlip(p) => (lower_in_hand_flip(p, e), "flip"),
        Primitive::GraspRelease(p) => (lower_grasp_release(p, e, ctx), "release"),
        Primitive::ReachRetract(p) => (lower_reach_retract(p, e), "retract"),
        Primitive::ReachScan(p) => (lower_reach_scan(p, e), "scan"),
        Primitive::ReachHover(p) => (lower_reach_hover(p, e), "hover"),
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
            station_keeping: None,
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
        station_keeping: None,
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
        // GC1 static floor: the held carry maintains min_holding_force (the static
        // counterpart of the dynamic a_max clamp below; spec/05 GC1 base continuity for
        // transport.move_to_pose, spec/02 § min_holding_force). Mirrors lower_grasp_pinch.
        let mhf = grasp_force::min_holding_force(held.weight_n, held.mode);
        env.force_profile =
            Some(serde_json::json!({ "min_holding_force": Quantity::from_si(mhf, "N").0 }));
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

/// Lower `transport.carry` (`spec/01` § 4.4): transport a held object while rejecting
/// disturbance. The held-secured floor (GC1) and the carry-clamped a_max are emitted from
/// `ctx.held` (like `lower_transport_move_to_pose`), but the acceleration is clamped *below*
/// the plain dynamic limit via `grasp_force::carry_a_max`, reserving margin for the
/// `disturbance_budget` (`spec/02`:137). `disturbance_budget` + `stability_margin` are emitted
/// for the ENV3 bench (a later increment). v0 lowers the `{to_pose: P}` MoveSpec; `{trajectory}`
/// is deferred. `stability_margin: auto` reads the descriptor default.
fn lower_transport_carry(p: &TransportCarry, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let target_pose = match p.motion.get("to_pose").map(yaml_to_json) {
        Some(serde_json::Value::Object(map)) => {
            let frame = map
                .get("frame")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("task")
                .to_string();
            let offset = map.get("offset").cloned().unwrap_or(serde_json::Value::Null);
            PoseExpr::FrameRelative { frame, offset }
        }
        // {trajectory: ...} or any other MoveSpec form is carried opaquely in v0.
        _ => PoseExpr::FrameRelative { frame: "task".into(), offset: yaml_to_json(&p.motion) },
    };
    let mut env = base_envelope(e);
    if let Some(held) = &ctx.held {
        let disturbance_n = match &p.disturbance_budget {
            Some(DisturbanceArg::Force(q)) => q.parse().map_or(0.0, |(v, _)| v),
            _ => 0.0, // auto deferred -> no extra reserve in v0
        };
        let margin = match &p.stability_margin {
            Some(StabilityMarginArg::Ratio(m)) => *m,
            _ => e.ratio_limit("stability_margin").unwrap_or(0.0), // auto -> descriptor default
        };
        let mhf = grasp_force::min_holding_force(held.weight_n, held.mode);
        env.force_profile = Some(serde_json::json!({
            "min_holding_force": Quantity::from_si(mhf, "N").0,
            "disturbance_budget": Quantity::from_si(disturbance_n, "N").0,
            "stability_margin": margin,
        }));
        let payload = e.scalar_limit(held.mode.payload_key()).and_then(|q| q.parse());
        let ceiling = e.scalar_limit("a_cartesian_max").and_then(|q| q.parse());
        if let (Some((payload_n, _)), Some((ceiling_v, unit))) = (payload, ceiling) {
            let unit = unit.to_string();
            let carry_a = grasp_force::carry_a_max(held.weight_n, payload_n, disturbance_n, margin);
            if carry_a < ceiling_v {
                env.motion_bounds.a_max = Some(Quantity::from_si(carry_a, &unit));
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

/// Lower `reach.hover` (`spec/01` § 1.5): a sustained station-keeping action holding
/// the controlled frame at a standoff setpoint over a bounded interval. v0 emits a
/// symbolic standoff pose (`S = p + standoff·n`); the geometric station invariant is
/// checked structurally (interval-invariant, `05` ENV2). The duration is carried in the
/// skill but not lowered in v0 (the interval check is sample-structural, not timed).
fn lower_reach_hover(p: &ReachHover, e: &Embodiment) -> CanonicalAction {
    let mut env = base_envelope(e);
    // `spec/01` § 1.5 C2: an explicit `settling_time` opts the hover into the ENV3 settling
    // contract; the disturbance-recovery check samples `station_keeping`. A bare hover
    // (`settling_time` absent / `auto`) emits nothing and stays ENV2-only.
    if let Some(settling) = p
        .settling_time
        .as_ref()
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| *s != "auto")
    {
        let tol = p
            .station_tolerance
            .clone()
            .unwrap_or_else(|| crate::quantity::Quantity("2 mm".to_string()));
        env.station_keeping = Some(serde_json::json!({
            "station_tolerance": tol.0,
            "settling_time": settling,
        }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::FrameRelative {
            frame: p.target.clone(),
            offset: serde_json::json!({ "along": "outward_normal", "distance": p.standoff.0.clone() }),
        },
        force_budget: None,
        timing: TimingHints {
            nominal_duration: None,
            timing_mode: TimingMode::Strict,
            stop_at_goal: true,
        },
        tactile_target: None,
        monitors: vec![],
        safety_envelope: env,
    }
}

/// Lower `force.press_button` (`spec/01` § 6.6): an effector press bounded by a force
/// trajectory (press force ≤ `force_budget`) and gated by an actuation event. v0 emits the
/// `force_budget` (the ForceTrajectory leg), the detent actuation marker into `force_profile`
/// (the bench echoes the detent; `check_actuation` verifies it), and the actuation as a
/// `Monitor` stop condition. Detent only; `effort_rise(force_threshold)` is carried but
/// unmarked (deferred). The over-travel / `max_travel` guard needs a displacement signal
/// (deferred).
fn lower_force_press_button(p: &ForcePressButton, e: &Embodiment) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.actuation) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if actuation_is_detent(&p.actuation) {
        env.force_profile = Some(serde_json::json!({ "actuation": "detent" }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target.clone() },
        force_budget: Some(p.force_budget.clone()),
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

/// Lower `force.wipe` (`spec/01` § 6.8): a hybrid force/position interval invariant. v0 emits
/// the two-sided normal-force band into `force_profile` (the contact-maintenance leg the
/// `ForceTrajectory` arm samples) when `normal_force_tolerance` is an explicit Force; an `auto`
/// tolerance emits no band (deferred). `wipe_path` is carried symbolic; the tangential
/// position-tracking + contour-following leg is deferred. `force_budget` is None — the band
/// owns both the upper and lower bounds.
fn lower_force_wipe(p: &ForceWipe, e: &Embodiment) -> CanonicalAction {
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if let Some(tol) = p
        .normal_force_tolerance
        .as_ref()
        .and_then(serde_yaml::Value::as_str)
        .filter(|s| *s != "auto")
    {
        env.force_profile = Some(serde_json::json!({
            "normal_force": p.normal_force.0.clone(),
            "normal_force_tolerance": tol,
        }));
    }
    CanonicalAction {
        target_frame: e.control_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.surface.clone() },
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

/// Lower `force.snap_engage` (`spec/01` § 6.10): a bistable engagement bounded by a force
/// trajectory (engagement force ≤ `force_budget`) and gated by a snap-in event (reusing
/// press_button's detent) + a confirm_held engagement-confirmation. v0 emits force_budget (the
/// ForceTrajectory leg), `force_profile.actuation = "detent"` (when snap_signature is detent /
/// default), and `force_profile.confirm_held = true` (default); the snap_signature lowers into a
/// Monitor. The held part's motion is along `engage_direction` (grasp frame, like force.screw);
/// `mate_feature` is carried symbolic; the snap_disengage reverse path is deferred.
fn lower_force_snap_engage(p: &ForceSnapEngage, e: &Embodiment) -> CanonicalAction {
    let is_detent = p.snap_signature.as_ref().map_or(true, actuation_is_detent);
    let do_confirm = p.confirm_held != Some(false);
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    if is_detent || do_confirm {
        let mut fp = serde_json::json!({});
        if is_detent {
            fp["actuation"] = serde_json::json!("detent");
        }
        if do_confirm {
            fp["confirm_held"] = serde_json::json!(true);
        }
        env.force_profile = Some(fp);
    }
    let sig = p
        .snap_signature
        .clone()
        .unwrap_or_else(|| serde_yaml::Value::String("detent".to_string()));
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&sig) }];
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.engage_direction),
            distance: Quantity("0 mm".to_string()),
        },
        force_budget: Some(p.force_budget.clone()),
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

/// Lower `force.cut` (`spec/01` § 6.7): an irreversible tool-mediated cut bounded by a force
/// trajectory (shear force ≤ `shear_force_budget`). v0 emits the shear budget (the ForceTrajectory
/// leg) + an `irreversible` marker (the partial-state-on-interruption check reads it); the
/// `completion` lowers into a Monitor and `cut_path` is carried opaque (path-bounding deferred).
/// Tool-mediated, so the held cutting tool's grasp frame is the controlled frame (like screw).
/// The tool_safety regime, path-bounding, and on_separation are deferred.
fn lower_force_cut(p: &ForceCut, e: &Embodiment) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.completion) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    env.force_profile = Some(serde_json::json!({ "irreversible": true }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::FrameRelative {
            frame: "task".to_string(),
            offset: yaml_to_json(&p.cut_path),
        },
        force_budget: Some(p.shear_force_budget.clone()),
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

/// Lower `in_hand.flip` (`spec/01` § 3.7): the only continuity-suspending primitive — a large
/// reorientation through a bounded, recoverable unsecured window. v0 lowers a symbolic
/// reorientation about `flip_axis` in the grasp frame and emits **no** force_profile floor (the
/// securing floor is intentionally suspended during the window, so the grasp-continuity check is
/// vacuous for it). `ctx` is unchanged — the flip re-secures, so a prior held object persists for
/// the downstream release. The bounded-window envelope (max_release_time / safe_drop_zone) and
/// the momentary_release audit (AUD2) are handled elsewhere (deferred / conformance).
fn lower_in_hand_flip(p: &InHandFlip, e: &Embodiment) -> CanonicalAction {
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.flip_axis),
            distance: Quantity("0 mm".to_string()),
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

/// True if an `ActuationSpec` is the detent mode (bare `detent` or `{detent: ...}`).
fn actuation_is_detent(v: &serde_yaml::Value) -> bool {
    v.as_str() == Some("detent")
        || v.as_mapping()
            .is_some_and(|m| m.contains_key(serde_yaml::Value::String("detent".to_string())))
}

/// Convert a `serde_yaml::Value` to a `serde_json::Value` deterministically.
fn yaml_to_json(v: &serde_yaml::Value) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// Lower `force.insert_fit`: carry the axial force_budget (clamped by GF3c to the
/// held grasp's reaction capacity — grip_force_max / k_reaction — so the part does
/// not slip in-grasp before seating), lower the SeatingSpec stop_condition into a
/// monitor, set compliance and the axial force_profile.
fn lower_force_insert_fit(p: &ForceInsertFit, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let mut force_budget = p.force_budget.clone();
    if let Some(held) = &ctx.held {
        if let Some((budget_n, unit)) = force_budget.parse() {
            let unit = unit.to_string();
            if let Some((grip_max_n, _)) = e.scalar_limit("grip_force_max").and_then(|q| q.parse()) {
                let limit = grasp_force::reaction_limit(budget_n, grip_max_n, held.mode);
                force_budget = Quantity::from_si(limit, &unit);
            }
        }
    }
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
    env.force_profile = Some(serde_json::json!({ "axial": force_budget.0.clone() }));
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::Ref { r#ref: p.target_fit.clone() },
        force_budget: Some(force_budget),
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

/// Lower `force.screw`: a tool-mediated screw's reaction torque loads the held tool's
/// grasp (GF4c), so the torque budget is clamped to the grasp's rotational holding
/// capacity (`grasp_force::reaction_torque_limit`) when a tool is held; otherwise it
/// passes through. The `ScrewStop` completion lowers into a monitor; `thread_pitch` is
/// expanded into a structured `force_profile.coupling` (the linked DOF). The runtime
/// decoupling-as-failure detection is a driver concern (deferred).
fn lower_force_screw(p: &ForceScrew, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let monitors = vec![Monitor { stop_condition: yaml_to_json(&p.completion) }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    // GF4c reaction-torque clamp: tool-mediated + a tool held -> bound the torque to the
    // tool grasp's rotational capacity (or the driver spins in-grasp).
    let tool_mediated = matches!(&p.tool_mediated, Some(v) if v.as_bool() != Some(false));
    let held = if tool_mediated { ctx.held.as_ref() } else { None };
    let torque = match (
        held,
        p.torque_budget.parse(),
        e.scalar_limit("grip_force_max").and_then(|q| q.parse()),
    ) {
        (Some(h), Some((tb, tu)), Some((gm, _))) => {
            Quantity::from_si(grasp_force::reaction_torque_limit(tb, gm, h.mode), tu)
        }
        _ => p.torque_budget.clone(),
    };
    let mut fp = serde_json::json!({ "torque": torque.0.clone() });
    if let Some(tp) = &p.thread_pitch {
        fp["coupling"] = serde_json::json!({ "advance_per_turn": tp.0.clone() });
    }
    if let Some(tm) = &p.tool_mediated {
        fp["tool_mediated"] = yaml_to_json(tm);
    }
    env.force_profile = Some(fp);
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.thread_axis),
            distance: Quantity("0 mm".to_string()),
        },
        force_budget: None,
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

/// Lower `force.unscrew`: reverse coupled rotation + axial retreat (`spec/01` § 6.5).
/// Mirrors `lower_force_screw` — the loosening torque loads the tool grasp (GF4c
/// reuse), `completion` defaults to a disengagement monitor when omitted, and
/// `rotation_sense: loosen` marks the reverse sense (symbolic, v0).
fn lower_force_unscrew(p: &ForceUnscrew, e: &Embodiment, ctx: &GraspContext) -> CanonicalAction {
    let stop_condition = match &p.completion {
        Some(c) => yaml_to_json(c),
        None => serde_json::json!({ "disengagement": true }),
    };
    let monitors = vec![Monitor { stop_condition }];
    let mut env = base_envelope(e);
    env.compliance = p.compliance.map(|c| {
        match c {
            Compliance::Passive => "passive",
            Compliance::Active => "active",
            Compliance::Auto => "auto",
        }
        .to_string()
    });
    // GF4c reuse: a tool-mediated loosening torque loads the held tool's grasp.
    let tool_mediated = matches!(&p.tool_mediated, Some(v) if v.as_bool() != Some(false));
    let held = if tool_mediated { ctx.held.as_ref() } else { None };
    let torque = match (
        held,
        p.torque_budget.parse(),
        e.scalar_limit("grip_force_max").and_then(|q| q.parse()),
    ) {
        (Some(h), Some((tb, tu)), Some((gm, _))) => {
            Quantity::from_si(grasp_force::reaction_torque_limit(tb, gm, h.mode), tu)
        }
        _ => p.torque_budget.clone(),
    };
    let mut fp = serde_json::json!({ "torque": torque.0.clone(), "rotation_sense": "loosen" });
    fp["on_disengagement"] = match &p.on_disengagement {
        Some(v) => yaml_to_json(v),
        None => serde_json::json!("retain"),
    };
    if let Some(tp) = &p.thread_pitch {
        fp["coupling"] = serde_json::json!({ "advance_per_turn": tp.0.clone() });
    }
    if let Some(tm) = &p.tool_mediated {
        fp["tool_mediated"] = yaml_to_json(tm);
    }
    env.force_profile = Some(fp);
    CanonicalAction {
        target_frame: e.grasp_frame().to_string(),
        target_pose: PoseExpr::AxisRelative {
            direction: yaml_to_json(&p.thread_axis),
            distance: Quantity("0 mm".to_string()),
        },
        force_budget: None,
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
        // Surface region: the pattern selects the generator. arc / waypoints-on-a-
        // surface still fall back to raster in v0 (arc needs a pivot + variable
        // orientation; the waypoints pattern is driven by a Waypoints region).
        crate::region::ScanRegion::Surface { size_u, size_v, .. } => {
            let (h, v) = e
                .sensor_fov(&sensor_frame)
                .map(|f| (angle_rad(&f.h_angle), angle_rad(&f.v_angle)))
                .unwrap_or((0.0, 0.0));
            let (u, vv) = (length_m(size_u), length_m(size_v));
            match pattern {
                ScanPattern::Spiral => crate::sigma::spiral(u, vv, standoff, overlap, h, v),
                _ => crate::sigma::raster(u, vv, standoff, overlap, h, v),
            }
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
    fn transport_emits_min_holding_force_on_held_carry() {
        let (skill, emb) = load("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        // suffixes: locate, pinch, transport(2), locate, align, insert_fit, release, retract
        assert_eq!(out.suffixes[2], "transport");
        let fp = out.actions[2]
            .safety_envelope
            .force_profile
            .as_ref()
            .expect("held transport carries force_profile");
        // connector estimated_mass 1.45 N, pinch -> min_holding_force = 1.45 * 2.0 = 2.9 N
        assert_eq!(fp.get("min_holding_force").and_then(|v| v.as_str()), Some("2.9 N"));
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
        let ctx = super::GraspContext::default();
        let a = super::lower_force_insert_fit(p, &emb, &ctx);
        assert_eq!(a.force_budget.as_ref().unwrap().0, "15 N"); // no held object -> unclamped
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
    fn scan_pattern_spiral_lowers_to_spiral_sweep() {
        let yaml = "skill: surface-scan\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: spiral\n        coverage_overlap: 0.2\n    - sense.inspect: { target: panel, observe: [defect] }\n";
        let skill = Skill::parse_yaml(yaml).unwrap();
        let mut allegro = Embodiment::parse_yaml(&std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/01-cable-insertion/embodiments/allegro.yaml"),
        ).unwrap()).unwrap();
        // The surface-scan skill declares sense.inspect; the 01 allegro does not.
        allegro.capabilities.skills.push("sense.inspect".to_string());
        let out = retarget(&skill, &allegro).expect("retarget");
        let crate::canonical::PoseExpr::SweepPath { poses, pattern } = &out.actions[0].target_pose
        else {
            panic!("expected SweepPath");
        };
        assert_eq!(pattern.as_str(), "spiral");
        assert_eq!(poses.len(), 12); // allegro 60x45 spiral -> 12 stations (raster was 9)
        // The first station is the centroid (U/2, V/2, standoff), round6-emitted.
        assert_eq!(poses[0].position, [0.1, 0.075, 0.1]);
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

    #[test]
    fn insert_fit_reaction_clamps_budget_per_hand() {
        // 15 N budget clamped to grip_force_max / 2: allegro 10, leap 7.5, pneumatic 6.
        for (stem, expected) in [("allegro", "10 N"), ("leap", "7.5 N"), ("pneumatic-6f", "6 N")] {
            let (skill, emb) = load(stem);
            let out = retarget(&skill, &emb).expect("retarget");
            // insert_fit is action index 5.
            assert_eq!(out.actions[5].force_budget.as_ref().unwrap().0, expected, "stem {stem}");
        }
    }

    const SCREW_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.screw:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        completion:\n          all_of:\n            - effort_rise: 1.5 N\u{b7}m\n            - reached: { advance: 5 mm }\n";

    #[test]
    fn screw_lowers_torque_and_completion() {
        let skill = Skill::parse_yaml(SCREW_SKILL).unwrap();
        let mut emb = load("allegro").1; // cable allegro descriptor
        emb.capabilities.skills.push("force.screw".to_string());
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["screw"]);
        let a = &out.actions[0];
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"torque\":\"2 N\u{b7}m\""), "got {fp}"); // no held tool -> no clamp
        assert!(fp.contains("\"advance_per_turn\":\"0.8 mm\""), "got {fp}");
        assert_eq!(a.safety_envelope.compliance.as_deref(), Some("active"));
        let mon = serde_json::to_string(&a.monitors).unwrap();
        assert!(mon.contains("effort_rise"));
        assert!(mon.contains("advance"));
        assert!(a.force_budget.is_none()); // the budget is a torque, in force_profile
    }

    #[test]
    fn screw_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(SCREW_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.screw
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.screw"), "got {err}");
    }

    const PRESS_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.press_button: { target: button, actuation: detent, force_budget: 5 N }\n";

    #[test]
    fn press_button_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(PRESS_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.press_button
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.press_button"), "got {err}");
    }

    const WIPE_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.wipe: { surface: panel, wipe_path: stroke_path, normal_force: 5 N, normal_force_tolerance: 1 N }\n";

    #[test]
    fn wipe_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(WIPE_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.wipe
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.wipe"), "got {err}");
    }

    const SNAP_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.snap_engage: { mate_feature: clip, engage_direction: +z, force_budget: 25 N, confirm_held: true }\n";

    #[test]
    fn snap_engage_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(SNAP_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.snap_engage
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.snap_engage"), "got {err}");
    }

    const CUT_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - force.cut: { cut_path: seam_path, shear_force_budget: 30 N, completion: separation }\n";

    #[test]
    fn cut_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(CUT_SKILL).unwrap();
        let emb = load("allegro").1; // cable allegro lacks force.cut
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.cut"), "got {err}");
    }

    const EMB_CUT_NO_TOOL_SAFETY: &str =
        "embodiment:\n  id: test-hand\n  capabilities:\n    skills: [force.cut]\n";

    #[test]
    fn cut_requires_tool_safety_capability() {
        let skill = Skill::parse_yaml(CUT_SKILL).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(EMB_CUT_NO_TOOL_SAFETY).unwrap();
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: tool_safety"), "got {err}");
    }

    const FLIP_SKILL: &str = "skill: t\nbody:\n  sequence:\n    - in_hand.flip: { flip_axis: +x, angle: 180 deg, max_release_time: 0.3 s, safe_drop_zone: tray }\n";

    #[test]
    fn flip_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(FLIP_SKILL).unwrap();
        let emb = load("allegro").1; // cable-01 allegro lacks in_hand.flip
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: in_hand.flip"), "got {err}");
    }

    #[test]
    fn flip_lowers_no_floor() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-flip.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        // locate(0), pinch(1), flip(2), release(3).
        assert_eq!(out.suffixes[2], "flip");
        // the flip suspends continuity -> no force_profile floor emitted.
        assert!(out.actions[2].safety_envelope.force_profile.is_none());
    }

    #[test]
    fn cut_lowers_irreversible_and_shear_budget() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-cut.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["cut"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("30 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"irreversible\":true"), "got {fp}");
    }

    #[test]
    fn snap_engage_lowers_actuation_confirm_held_and_force_budget() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-snap.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["snap_engage"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("25 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"actuation\":\"detent\""), "got {fp}");
        assert!(fp.contains("\"confirm_held\":true"), "got {fp}");
    }

    #[test]
    fn wipe_lowers_normal_force_band() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-wipe.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["wipe"]);
        let a = &out.actions[0];
        assert!(a.force_budget.is_none()); // the band owns both bounds
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"normal_force\":\"5 N\""), "got {fp}");
        assert!(fp.contains("\"normal_force_tolerance\":\"1 N\""), "got {fp}");
    }

    #[test]
    fn press_button_lowers_force_budget_and_detent_actuation() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill-press.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["press_button"]);
        let a = &out.actions[0];
        assert_eq!(a.force_budget.as_ref().map(|q| q.0.as_str()), Some("5 N"));
        let fp = serde_json::to_string(&a.safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"actuation\":\"detent\""), "got {fp}");
        let mon = serde_json::to_string(&a.monitors).unwrap();
        assert!(mon.contains("detent"), "got {mon}");
    }

    #[test]
    fn screw_torque_clamped_to_tool_grasp_capacity() {
        // examples/03: grasp.pinch holds the driver, so the tool-mediated force.screw
        // clamps the 2 N·m budget to grip_force_max * R_GRIP / k_reaction = 20*0.02/2 = 0.2.
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/03-screw-fasten");
        let skill =
            Skill::parse_yaml(&std::fs::read_to_string(dir.join("skill.yaml")).unwrap()).unwrap();
        let emb = crate::embodiment::Embodiment::parse_yaml(
            &std::fs::read_to_string(dir.join("embodiments/allegro.yaml")).unwrap(),
        )
        .unwrap();
        let out = retarget(&skill, &emb).expect("retarget");
        // force.screw is action index 5 (locate, pinch, transport, locate, align, screw, ...).
        let fp = serde_json::to_string(&out.actions[5].safety_envelope.force_profile).unwrap();
        assert!(fp.contains("\"torque\":\"0.2 N\u{b7}m\""), "got {fp}");
        assert!(fp.contains("\"advance_per_turn\":\"0.8 mm\""), "got {fp}");
    }

    #[test]
    fn unscrew_lowers_to_torque_trajectory_with_disengagement_default() {
        // Standalone (no pinch) -> ctx.held is None -> torque unclamped; completion
        // omitted -> the disengagement default monitor; rotation_sense marks the reverse.
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.unscrew:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        on_disengagement: retain\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let mut emb = load("allegro").1;
        emb.capabilities.skills.push("force.unscrew".to_string());
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["unscrew"]);
        let fp = out.actions[0].safety_envelope.force_profile.as_ref().expect("force_profile");
        assert_eq!(fp.get("torque").and_then(|v| v.as_str()), Some("2 N\u{b7}m")); // unclamped (no tool held)
        assert_eq!(fp.get("rotation_sense").and_then(|v| v.as_str()), Some("loosen"));
        assert_eq!(fp.get("on_disengagement").and_then(|v| v.as_str()), Some("retain"));
        let m = &out.actions[0].monitors[0];
        assert_eq!(m.stop_condition.get("disengagement").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn unscrew_capability_absent_when_gate_key_missing() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.unscrew: { thread_axis: -z, torque_budget: 2 N\u{b7}m }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let emb = load("allegro").1; // cable allegro lacks force.unscrew
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: force.unscrew"), "got {err}");
    }

    const CARRY_SKILL: &str = "skill: cable-carry\nobjects:\n  connector: { ref: connector, estimated_mass: 1.45 N }\nbody:\n  sequence:\n    - let: connector_t\n      from:\n        sense.locate: { target_ref: connector, modality: auto }\n    - grasp.pinch: { target: connector_t, force_budget: 8 N, tactile_target: auto }\n    - transport.carry:\n        motion: { to_pose: { frame: staging, offset: { along: +z, distance: 100 mm } } }\n        disturbance_budget: 0.4 N\n        stability_margin: auto\n";

    fn carry_emb(stem: &str) -> Embodiment {
        let (_, mut emb) = load(stem);
        emb.capabilities.skills.push("transport.carry".to_string());
        emb.limits
            .insert("stability_margin".to_string(), serde_yaml::from_str("0.5").unwrap());
        emb
    }

    #[test]
    fn carry_emits_floor_disturbance_margin_and_clamps_a_max_on_allegro() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        let emb = carry_emb("allegro");
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["locate", "pinch", "carry"]);
        let fp = out.actions[2].safety_envelope.force_profile.as_ref().expect("force_profile");
        assert_eq!(fp.get("min_holding_force").and_then(|v| v.as_str()), Some("2.9 N"));
        assert_eq!(fp.get("disturbance_budget").and_then(|v| v.as_str()), Some("0.4 N"));
        assert_eq!(fp.get("stability_margin").and_then(serde_json::Value::as_f64), Some(0.5));
        // allegro carry a_max clamps from the 1.5 kinematic ceiling to 1.014481.
        assert_eq!(
            out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
            "1.014481 m/s^2"
        );
    }

    #[test]
    fn carry_a_max_clamps_to_zero_on_weaker_hands() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        for stem in ["leap", "pneumatic-6f"] {
            let out = retarget(&skill, &carry_emb(stem)).expect("retarget");
            assert_eq!(
                out.actions[2].safety_envelope.motion_bounds.a_max.as_ref().unwrap().0,
                "0 m/s^2",
                "stem {stem}"
            );
        }
    }

    #[test]
    fn carry_capability_absent_when_not_declared() {
        let skill = Skill::parse_yaml(CARRY_SKILL).expect("parse");
        let (_, mut emb) = load("allegro");
        // The 01 descriptors now declare transport.carry (the skill-carry example); strip it
        // to exercise the gate. transport.carry is a distinct capability from base transport.
        emb.capabilities.skills.retain(|s| s != "transport.carry");
        let err = retarget(&skill, &emb).unwrap_err();
        assert!(err.to_string().contains("capability_absent: transport.carry"), "got {err}");
    }

    #[test]
    fn hover_lowers_to_a_standoff_setpoint() {
        // reach.* is baseline (no capability gate). The hover lowers to a symbolic
        // standoff setpoint in the control frame.
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm, duration: 5 s }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let emb = load("allegro").1;
        let out = retarget(&skill, &emb).expect("retarget");
        assert_eq!(out.suffixes, vec!["hover"]);
        let crate::canonical::PoseExpr::FrameRelative { frame, offset } = &out.actions[0].target_pose
        else {
            panic!("expected FrameRelative");
        };
        assert_eq!(frame, "panel");
        assert_eq!(offset.get("distance").and_then(|v| v.as_str()), Some("50 mm"));
    }

    #[test]
    fn hover_with_settling_time_emits_station_keeping() {
        // spec/01 § 1.5 C2: an explicit settling_time opts the hover into the ENV3 contract.
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm, station_tolerance: 2 mm, settling_time: 1 s }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let emb = load("allegro").1;
        let out = retarget(&skill, &emb).expect("retarget");
        let sk = out.actions[0]
            .safety_envelope
            .station_keeping
            .as_ref()
            .expect("station_keeping emitted");
        assert_eq!(sk.get("station_tolerance").and_then(|v| v.as_str()), Some("2 mm"));
        assert_eq!(sk.get("settling_time").and_then(|v| v.as_str()), Some("1 s"));
    }

    #[test]
    fn bare_hover_emits_no_station_keeping() {
        // No settling_time -> ENV2-only, unchanged (no station_keeping).
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.hover: { target: panel, standoff: 50 mm }\n";
        let skill = Skill::parse_yaml(yaml).expect("parse");
        let out = retarget(&skill, &load("allegro").1).expect("retarget");
        assert!(out.actions[0].safety_envelope.station_keeping.is_none());
    }
}
