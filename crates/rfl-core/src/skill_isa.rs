// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Skill Instruction Set Architecture (Skill ISA).
//!
//! The Skill ISA defines fifty manipulation primitives organized into seven
//! categories, plus a compositional algebra (sequence, parallel, reactive,
//! repeat, branch) that combines primitives into multi-step manipulations.
//!
//! See `spec/01-skill-isa.md` for the formal definition.

/// One of the seven primitive categories defined by the Skill ISA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Category {
    /// Reaching and approach primitives.
    Reach,
    /// Grasping primitives (pinch, power, hook, ...).
    Grasp,
    /// In-hand manipulation primitives.
    InHand,
    /// Transport primitives (carry, hand-off, ...).
    Transport,
    /// Placement primitives.
    Place,
    /// Force-controlled interaction primitives (insertion, screwing, ...).
    Force,
    /// Sensing-only primitives (probe, inspect, ...).
    Sense,
}

/// A canonical Skill ISA primitive identifier.
///
/// Concrete primitives are defined in `spec/01-skill-isa.md` and loaded from
/// the JSON schemas under `schemas/skill-isa.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveId(pub String);

use std::collections::BTreeMap;
use crate::quantity::Quantity;

/// A reference to a let-bound value (`$defs/Ref`): a bare identifier naming a value
/// bound earlier by a let-bind.
pub type Ref = String;
/// A calibrated frame name (`$defs/FrameRef`).
pub type FrameRef = String;

/// A parsed Skill ISA composition file (`schemas/skill-isa.schema.json` top level).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Skill {
    /// Skill name (the composition's identifier).
    pub skill: String,
    /// Human-readable summary.
    #[serde(default)]
    pub description: Option<String>,
    /// Task object references keyed by local name.
    #[serde(default)]
    pub objects: BTreeMap<String, ObjectDecl>,
    /// The skill body (v0 supports the sequence form).
    pub body: Sequence,
}

/// A task object reference (`$defs/ObjectDecl`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ObjectDecl {
    /// The object identity this local name binds to.
    pub r#ref: String,
    /// Optional declared weight (`spec/01` ObjectTarget.estimated_mass: Force), a
    /// known prior the Translation Layer reads for the grasp-force derivations.
    #[serde(default)]
    pub estimated_mass: Option<Quantity>,
}

/// An ordered composition (`$defs/Sequence`). v0 supports the sequence body only.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Sequence {
    /// The ordered statements.
    pub sequence: Vec<Statement>,
}

/// One statement of a sequence: a primitive call or a let-bind. `serde(untagged)`
/// discriminates by shape — a `{let, from}` object is a `LetBind`, any other
/// single-key object is a primitive call.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Statement {
    /// A let-bind statement.
    LetBind(LetBind),
    /// A bare primitive call.
    Primitive(Primitive),
}

/// A let-bind statement (`$defs/LetBind`): bind `let` to the result of `from`
/// (a sense.* call), scoped over the rest of the enclosing sequence.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LetBind {
    /// The bound identifier.
    pub r#let: String,
    /// The expression whose result is bound (a sense.* call).
    pub from: Box<Primitive>,
}

/// A core primitive invocation (`$defs/PrimitiveCall`): a single key naming the
/// primitive, mapping to its parameters. v0 models the seven cable-insertion
/// primitives.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum Primitive {
    /// `sense.locate`.
    #[serde(rename = "sense.locate")]
    SenseLocate(SenseLocate),
    /// `grasp.pinch`.
    #[serde(rename = "grasp.pinch")]
    GraspPinch(GraspPinch),
    /// `transport.move_to_pose`.
    #[serde(rename = "transport.move_to_pose")]
    TransportMoveToPose(TransportMoveToPose),
    /// `reach.align`.
    #[serde(rename = "reach.align")]
    ReachAlign(ReachAlign),
    /// `force.insert_fit`.
    #[serde(rename = "force.insert_fit")]
    ForceInsertFit(ForceInsertFit),
    /// `grasp.release`.
    #[serde(rename = "grasp.release")]
    GraspRelease(GraspRelease),
    /// `reach.retract`.
    #[serde(rename = "reach.retract")]
    ReachRetract(ReachRetract),
    /// `reach.scan`.
    #[serde(rename = "reach.scan")]
    ReachScan(ReachScan),
    /// `reach.hover`.
    #[serde(rename = "reach.hover")]
    ReachHover(ReachHover),
    /// `sense.inspect`.
    #[serde(rename = "sense.inspect")]
    SenseInspect(SenseInspect),
    /// `force.screw`.
    #[serde(rename = "force.screw")]
    ForceScrew(ForceScrew),
    /// `force.unscrew`.
    #[serde(rename = "force.unscrew")]
    ForceUnscrew(ForceUnscrew),
}

/// `sense.locate` modality (`$defs/SenseLocateParams.modality`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    /// Visual sensing.
    Visual,
    /// Tactile sensing.
    Tactile,
    /// Fused multi-modal sensing.
    Fused,
    /// Planner-selected best available.
    Auto,
}

/// `sense.locate` parameters (v0 subset of `$defs/SenseLocateParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseLocate {
    /// The object identity whose pose is estimated.
    pub target_ref: Ref,
    /// Sensing modality (default auto).
    #[serde(default)]
    pub modality: Option<Modality>,
}

/// Reaction to detected slip (`$defs/SlipResponse`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlipResponse {
    /// Abort on slip.
    Abort,
    /// Retighten on slip.
    Retighten,
    /// Hold through slip.
    Hold,
}

/// `tactile_target` argument (`$defs/TactileTargetOrAuto`). v0 needs the `auto`
/// form the reference uses; an inline target or a let-reference round-trips as
/// `Other` (carried, not interpreted, in v0).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum TactileTargetArg {
    /// The literal `auto`.
    Auto(AutoLiteral),
    /// An inline target or a let-reference, carried opaquely.
    Other(serde_yaml::Value),
}

/// The literal string `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum AutoLiteral {
    /// `auto`.
    #[serde(rename = "auto")]
    Auto,
}

impl TactileTargetArg {
    /// Whether this is the `auto` form.
    #[must_use]
    pub fn is_auto(&self) -> bool {
        matches!(self, TactileTargetArg::Auto(_))
    }
}

/// `grasp.pinch` parameters (v0 subset of `$defs/GraspPinchParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPinch {
    /// The object target to pinch (a let-reference in the reference skill).
    pub target: Ref,
    /// Max grip force.
    pub force_budget: Quantity,
    /// Contact-confirmation criterion (default auto).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
    /// Reaction to detected slip.
    #[serde(default)]
    pub slip_response: Option<SlipResponse>,
}

fn tactile_auto() -> TactileTargetArg {
    TactileTargetArg::Auto(AutoLiteral::Auto)
}

/// `transport.move_to_pose` parameters (v0 subset of `$defs/TransportMoveToPoseParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportMoveToPose {
    /// `Pose6D` floored as an inline object (a frame-relative offset) or a ref.
    pub target_pose: serde_yaml::Value,
}

/// `reach.align` axes (`$defs/Axes`): the literal `all` or a set of principal axes.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum Axes {
    /// All three orientation axes.
    All(AllLiteral),
    /// A subset of principal axes.
    Set(Vec<Axis>),
}

/// The literal string `all`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum AllLiteral {
    /// `all`.
    #[serde(rename = "all")]
    All,
}

/// An unsigned principal axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    /// x.
    X,
    /// y.
    Y,
    /// z.
    Z,
}

/// `reach.align` parameters (v0 subset of `$defs/ReachAlignParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachAlign {
    /// Calibrated reference orientation frame.
    pub target_frame: FrameRef,
    /// Controlled-frame axes to bring parallel to `target_frame` (default all).
    #[serde(default = "axes_all")]
    pub axes: Axes,
}

fn axes_all() -> Axes {
    Axes::All(AllLiteral::All)
}

/// `force.insert_fit` compliance (`$defs/Compliance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compliance {
    /// Passive compliance.
    Passive,
    /// Active compliance.
    Active,
    /// Planner-selected.
    Auto,
}

/// `force.insert_fit` parameters (v0 subset of `$defs/ForceInsertFitParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceInsertFit {
    /// The hole / socket / receptacle (a let-reference in the reference skill).
    pub target_fit: Ref,
    /// Max force along the insertion axis.
    pub force_budget: Quantity,
    /// Compliance mode required for the fit.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// `SeatingSpec` stop condition, carried structurally and lowered into
    /// `monitors` by the Translation Layer.
    pub stop_condition: serde_yaml::Value,
}

/// A grasp handle (`$defs/GraspHandle`): the literal `active` or a let-reference.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum GraspHandle {
    /// The current grasp on the controlled frame.
    Active(ActiveLiteral),
    /// A let-reference to a grasp bound earlier.
    Ref(Ref),
}

/// The literal string `active`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum ActiveLiteral {
    /// `active`.
    #[serde(rename = "active")]
    Active,
}

/// `grasp.release` parameters (v0 subset of `$defs/GraspReleaseParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspRelease {
    /// The grasp to release (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// A direction (`$defs/Direction`): a signed named axis (`-tool_axis`), an inline
/// unit vector, or a let-reference. v0 carries the value opaquely.
pub type Direction = serde_yaml::Value;

/// `reach.retract` parameters (v0 subset of `$defs/ReachRetractParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachRetract {
    /// Retreat direction (default the reverse of the tool axis).
    pub direction: Direction,
    /// Retreat travel along `direction`.
    pub distance: Quantity,
}

/// `reach.scan` sweep pattern (`$defs/ReachScanParams.pattern`). v0 lowers raster
/// and waypoints; spiral and arc are accepted but fall back to raster (deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanPattern {
    /// Serpentine surface coverage (default).
    Raster,
    /// Caller-supplied poses verbatim.
    Waypoints,
    /// Archimedean spiral (deferred; falls back to raster).
    Spiral,
    /// Swept arc (deferred; falls back to raster).
    Arc,
}

/// `reach.scan` parameters (v0 subset of `$defs/ReachScanParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachScan {
    /// The region to cover.
    pub region: crate::region::ScanRegion,
    /// Sensor-to-region distance maintained during the sweep.
    pub standoff: Quantity,
    /// Sweep pattern (default raster).
    #[serde(default)]
    pub pattern: Option<ScanPattern>,
    /// Sensor frame whose coverage matters (default the embodiment sensor frame).
    #[serde(default)]
    pub sensor_frame: Option<FrameRef>,
    /// Overlap between passes (v0 requires an explicit ratio; auto is deferred).
    #[serde(default)]
    pub coverage_overlap: Option<f64>,
}

/// `reach.hover` parameters (v0 subset of `$defs/ReachHoverParams`). Sustained
/// station-keeping at a standoff over a bounded interval (`spec/01` § 1.5). v0 models
/// the target frame, the standoff, and the duration (carried, not lowered); the § 1.5
/// tolerances / tracking carry spec defaults and are not emitted.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachHover {
    /// The surface / frame to hover over (v0: a frame name).
    pub target: FrameRef,
    /// Maintained distance from `target` along its outward normal.
    pub standoff: Quantity,
    /// Hold duration (`Duration | until`; carried, symbolic in v0).
    #[serde(default)]
    pub duration: Option<serde_yaml::Value>,
}

/// `sense.inspect` parameters (v0 subset of `$defs/SenseInspectParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseInspect {
    /// What to observe (a let-reference or frame name in the reference skill).
    pub target: Ref,
    /// What to capture (interpretation is out of RFL scope).
    #[serde(default)]
    pub observe: Option<Vec<String>>,
}

/// `force.screw` parameters (v0 subset of `$defs/ForceScrewParams`). `completion`
/// (a `ScrewStop`) and `tool_mediated` / `thread_pitch` are carried structurally;
/// the tool-mediated reaction + rotation↔advance coupling are a later increment (GF4c).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceScrew {
    /// The screw / thread axis (in `frame`).
    pub thread_axis: Direction,
    /// Max torque about `thread_axis`.
    pub torque_budget: Quantity,
    /// `ScrewStop` completion, lowered into `monitors` by the Translation Layer.
    pub completion: serde_yaml::Value,
    /// Rotation→advance coupling pitch (carried; symbolic in v0).
    #[serde(default)]
    pub thread_pitch: Option<Quantity>,
    /// Whether a held tool transmits the torque (carried; reaction is a later increment).
    #[serde(default)]
    pub tool_mediated: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// The grasp on the fastener or the driving tool.
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `force.unscrew` parameters (v0 subset of `$defs/ForceUnscrewParams`). Mirrors
/// `ForceScrew` minus `axial_force_budget`; `completion` is optional (defaults to
/// disengagement, emitted at lowering); `on_disengagement` is the freed-fastener
/// disposition. The authored effort_drop completion variant is schema-blocked.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceUnscrew {
    /// The thread axis (in `frame`).
    pub thread_axis: Direction,
    /// Max loosening torque about `thread_axis`.
    pub torque_budget: Quantity,
    /// `ScrewStop` completion (optional; default disengagement, emitted at lowering).
    #[serde(default)]
    pub completion: Option<serde_yaml::Value>,
    /// Reverse rotation->retreat coupling pitch (carried; symbolic in v0).
    #[serde(default)]
    pub thread_pitch: Option<Quantity>,
    /// Whether a held tool transmits the loosening torque.
    #[serde(default)]
    pub tool_mediated: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// The grasp on the fastener or the driving tool.
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
    /// Freed-fastener disposition `{retain, drop_safe}` (default retain).
    #[serde(default)]
    pub on_disengagement: Option<serde_yaml::Value>,
}

impl Skill {
    /// Parse a Skill ISA composition from YAML text.
    ///
    /// # Errors
    /// Returns `Error::SkillIsa` if the YAML does not match the v0-supported subset
    /// (header + sequence body + the seven primitives + let-bind).
    pub fn parse_yaml(text: &str) -> crate::Result<Self> {
        serde_yaml::from_str(text).map_err(|e| crate::Error::SkillIsa(e.to_string()))
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use std::path::Path;

    fn cable_skill() -> Skill {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/01-cable-insertion/skill.yaml");
        let text = std::fs::read_to_string(p).expect("read skill.yaml");
        Skill::parse_yaml(&text).expect("parse skill.yaml")
    }

    #[test]
    fn parses_header_and_objects() {
        let s = cable_skill();
        assert_eq!(s.skill, "cable-insertion");
        assert!(s.objects.contains_key("connector"));
        assert_eq!(s.objects["receptacle"].r#ref, "receptacle");
    }

    #[test]
    fn connector_declares_estimated_mass() {
        let s = cable_skill();
        assert_eq!(s.objects["connector"].estimated_mass.as_ref().unwrap().0, "1.45 N");
        assert!(s.objects["receptacle"].estimated_mass.is_none());
    }

    #[test]
    fn body_has_eight_statements_in_order() {
        let s = cable_skill();
        let stmts = &s.body.sequence;
        assert_eq!(stmts.len(), 8);
        assert!(matches!(&stmts[0], Statement::LetBind(_))); // let connector_t
        assert!(matches!(&stmts[1], Statement::Primitive(Primitive::GraspPinch(_))));
        assert!(matches!(&stmts[2], Statement::Primitive(Primitive::TransportMoveToPose(_))));
        assert!(matches!(&stmts[3], Statement::LetBind(_))); // let receptacle_t
        assert!(matches!(&stmts[4], Statement::Primitive(Primitive::ReachAlign(_))));
        assert!(matches!(&stmts[5], Statement::Primitive(Primitive::ForceInsertFit(_))));
        assert!(matches!(&stmts[6], Statement::Primitive(Primitive::GraspRelease(_))));
        assert!(matches!(&stmts[7], Statement::Primitive(Primitive::ReachRetract(_))));
    }

    #[test]
    fn pinch_params_parse() {
        let s = cable_skill();
        let Statement::Primitive(Primitive::GraspPinch(p)) = &s.body.sequence[1] else {
            panic!("expected grasp.pinch");
        };
        assert_eq!(p.force_budget.0, "8 N");
        assert!(p.tactile_target.is_auto());
        assert_eq!(p.slip_response, Some(SlipResponse::Retighten));
    }

    #[test]
    fn parses_scan_primitives() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - reach.scan:\n        region: { kind: surface, frame: panel, size_u: 200 mm, size_v: 150 mm }\n        standoff: 100 mm\n        pattern: raster\n        coverage_overlap: 0.2\n    - sense.inspect:\n        target: panel\n        observe: [defect]\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::ReachScan(p)) = &s.body.sequence[0] else { panic!() };
        assert_eq!(p.standoff.0, "100 mm");
        assert!(matches!(p.pattern, Some(ScanPattern::Raster)));
        assert!(matches!(&s.body.sequence[1], Statement::Primitive(Primitive::SenseInspect(_))));
    }

    #[test]
    fn insert_fit_stop_condition_parses() {
        let s = cable_skill();
        let Statement::Primitive(Primitive::ForceInsertFit(p)) = &s.body.sequence[5] else {
            panic!("expected force.insert_fit");
        };
        assert_eq!(p.force_budget.0, "15 N");
        assert_eq!(p.compliance, Some(Compliance::Active));
    }

    #[test]
    fn force_screw_parses() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - force.screw:\n        grasp_handle: active\n        thread_axis: -z\n        torque_budget: 2 N\u{b7}m\n        thread_pitch: 0.8 mm\n        tool_mediated: true\n        compliance: active\n        completion:\n          all_of:\n            - effort_rise: 1.5 N\u{b7}m\n            - reached: { advance: 5 mm }\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::ForceScrew(p)) = &s.body.sequence[0] else {
            panic!("expected force.screw");
        };
        assert_eq!(p.torque_budget.0, "2 N\u{b7}m");
        assert_eq!(p.thread_pitch.as_ref().unwrap().0, "0.8 mm");
        assert!(matches!(p.compliance, Some(Compliance::Active)));
    }
}
