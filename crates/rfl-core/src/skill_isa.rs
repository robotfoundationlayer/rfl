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

use crate::grasp_force::GraspMode;
use crate::quantity::Quantity;
use crate::stability::{Closure, DofSecuring, StabilityMetadata};
use std::collections::BTreeMap;

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
// A parse-time AST node (lives in a `Vec<Statement>`, deserialized once); boxing the larger
// variant would obscure the serde(untagged) shape for a negligible perf gain.
#[allow(clippy::large_enum_variant)]
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
    /// `grasp.power`.
    #[serde(rename = "grasp.power")]
    GraspPower(GraspPower),
    /// `grasp.lateral`.
    #[serde(rename = "grasp.lateral")]
    GraspLateral(GraspLateral),
    /// `grasp.precision_tripod`.
    #[serde(rename = "grasp.precision_tripod")]
    GraspPrecisionTripod(GraspPrecisionTripod),
    /// `grasp.hook`.
    #[serde(rename = "grasp.hook")]
    GraspHook(GraspHook),
    /// `grasp.envelope`.
    #[serde(rename = "grasp.envelope")]
    GraspEnvelope(GraspEnvelope),
    /// `transport.move_to_pose`.
    #[serde(rename = "transport.move_to_pose")]
    TransportMoveToPose(TransportMoveToPose),
    /// `transport.lift`.
    #[serde(rename = "transport.lift")]
    TransportLift(TransportLift),
    /// `transport.lower`.
    #[serde(rename = "transport.lower")]
    TransportLower(TransportLower),
    /// `transport.follow_trajectory`.
    #[serde(rename = "transport.follow_trajectory")]
    TransportFollowTrajectory(TransportFollowTrajectory),
    /// `place.put_down`.
    #[serde(rename = "place.put_down")]
    PlacePutDown(PlacePutDown),
    /// `place.stack`.
    #[serde(rename = "place.stack")]
    PlaceStack(PlaceStack),
    /// `place.insert_loose`.
    #[serde(rename = "place.insert_loose")]
    PlaceInsertLoose(PlaceInsertLoose),
    /// `place.orient`.
    #[serde(rename = "place.orient")]
    PlaceOrient(PlaceOrient),
    /// `place.hand_to`.
    #[serde(rename = "place.hand_to")]
    PlaceHandTo(PlaceHandTo),
    /// `place.discard`.
    #[serde(rename = "place.discard")]
    PlaceDiscard(PlaceDiscard),
    /// `reach.align`.
    #[serde(rename = "reach.align")]
    ReachAlign(ReachAlign),
    /// `reach.to_pose`.
    #[serde(rename = "reach.to_pose")]
    ReachToPose(ReachToPose),
    /// `reach.approach`.
    #[serde(rename = "reach.approach")]
    ReachApproach(ReachApproach),
    /// `force.insert_fit`.
    #[serde(rename = "force.insert_fit")]
    ForceInsertFit(ForceInsertFit),
    /// `force.push`.
    #[serde(rename = "force.push")]
    ForcePush(ForcePush),
    /// `force.pull`.
    #[serde(rename = "force.pull")]
    ForcePull(ForcePull),
    /// `force.scrub`.
    #[serde(rename = "force.scrub")]
    ForceScrub(ForceScrub),
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
    /// `sense.probe`.
    #[serde(rename = "sense.probe")]
    SenseProbe(SenseProbe),
    /// `sense.verify`.
    #[serde(rename = "sense.verify")]
    SenseVerify(SenseVerify),
    /// `sense.weigh`.
    #[serde(rename = "sense.weigh")]
    SenseWeigh(SenseWeigh),
    /// `force.screw`.
    #[serde(rename = "force.screw")]
    ForceScrew(ForceScrew),
    /// `force.unscrew`.
    #[serde(rename = "force.unscrew")]
    ForceUnscrew(ForceUnscrew),
    /// `transport.carry`.
    #[serde(rename = "transport.carry")]
    TransportCarry(TransportCarry),
    /// `force.press_button`.
    #[serde(rename = "force.press_button")]
    ForcePressButton(ForcePressButton),
    /// `force.wipe`.
    #[serde(rename = "force.wipe")]
    ForceWipe(ForceWipe),
    /// `force.snap_engage`.
    #[serde(rename = "force.snap_engage")]
    ForceSnapEngage(ForceSnapEngage),
    /// `force.cut`.
    #[serde(rename = "force.cut")]
    ForceCut(ForceCut),
    /// `in_hand.flip`.
    #[serde(rename = "in_hand.flip")]
    InHandFlip(InHandFlip),
    /// `grasp.pin`.
    #[serde(rename = "grasp.pin")]
    GraspPin(GraspPin),
    /// `grasp.platform`.
    #[serde(rename = "grasp.platform")]
    GraspPlatform(GraspPlatform),
    /// `in_hand.regrasp`.
    #[serde(rename = "in_hand.regrasp")]
    InHandRegrasp(InHandRegrasp),
    /// `in_hand.pivot`.
    #[serde(rename = "in_hand.pivot")]
    InHandPivot(InHandPivot),
    /// `in_hand.rotate`.
    #[serde(rename = "in_hand.rotate")]
    InHandRotate(InHandRotate),
    /// `in_hand.translate`.
    #[serde(rename = "in_hand.translate")]
    InHandTranslate(InHandTranslate),
    /// `in_hand.roll`.
    #[serde(rename = "in_hand.roll")]
    InHandRoll(InHandRoll),
    /// `in_hand.slide`.
    #[serde(rename = "in_hand.slide")]
    InHandSlide(InHandSlide),
    /// `transport.handoff`.
    #[serde(rename = "transport.handoff")]
    TransportHandoff(TransportHandoff),
}

impl Primitive {
    /// The grasp mode this primitive establishes, if it forms a grasp. Read by the
    /// STB3 composition check to track the active grasp's stability class across a
    /// sequence. `None` for non-grasp primitives.
    #[must_use]
    pub fn establishes_grasp(&self) -> Option<GraspMode> {
        match self {
            Primitive::GraspPinch(_) => Some(GraspMode::Pinch),
            Primitive::GraspPower(_) => Some(GraspMode::Power),
            Primitive::GraspLateral(_) => Some(GraspMode::Lateral),
            Primitive::GraspPrecisionTripod(_) => Some(GraspMode::PrecisionTripod),
            Primitive::GraspHook(_) => Some(GraspMode::Hook),
            Primitive::GraspEnvelope(p) => Some(p.grasp_mode()),
            Primitive::GraspPin(_) => Some(GraspMode::Pin),
            Primitive::GraspPlatform(_) => Some(GraspMode::Platform),
            // a regrasp supersedes the active grasp with its target mode (spec/01 § 3.3).
            Primitive::InHandRegrasp(r) => Some(r.target_grasp_mode()),
            // a handoff supersedes the giver's grasp with the receiver's new grasp (§ 4.3).
            Primitive::TransportHandoff(h) => Some(h.receiver_grasp_mode()),
            _ => None,
        }
    }

    /// True if this primitive supersedes the active grasp — produces a new GraspState that
    /// invalidates the originating `GraspRef` (`spec/01` § 387): `in_hand.regrasp` and
    /// `transport.handoff`.
    #[must_use]
    pub fn supersedes_grasp(&self) -> bool {
        matches!(
            self,
            Primitive::InHandRegrasp(_) | Primitive::TransportHandoff(_)
        )
    }

    /// True if this primitive releases the active grasp (clears the active-grasp state). The
    /// `place.*` family all end with a controlled release, so they clear the held state too.
    #[must_use]
    pub fn releases_grasp(&self) -> bool {
        matches!(
            self,
            Primitive::GraspRelease(_)
                | Primitive::PlacePutDown(_)
                | Primitive::PlaceStack(_)
                | Primitive::PlaceInsertLoose(_)
                | Primitive::PlaceOrient(_)
                | Primitive::PlaceHandTo(_)
                | Primitive::PlaceDiscard(_)
        )
    }

    /// True if this primitive freely transports a held object through space — the
    /// successor a `surface_bound` grasp forbids (`spec/05` STB3).
    #[must_use]
    pub fn is_free_transport(&self) -> bool {
        matches!(
            self,
            Primitive::TransportMoveToPose(_) | Primitive::TransportCarry(_)
        )
    }

    /// True if this primitive unambiguously requires a held object (a `held` input frame) — using
    /// it with no active grasp is `no_active_grasp` (`spec/01` § Lifecycle transitions). Covers the
    /// operations that act ON a held object: `in_hand.*`, `place.*`, `transport.handoff`,
    /// `grasp.release`, `force.pull`, and `sense.weigh`. The base/vertical transports
    /// (`move_to_pose` / `lift` / `lower` / `carry` / `follow_trajectory`) are deliberately excluded
    /// — a free move of the empty effector is a degenerate-but-legal use.
    #[must_use]
    pub fn requires_held_grasp(&self) -> bool {
        matches!(
            self,
            Primitive::InHandFlip(_)
                | Primitive::InHandRegrasp(_)
                | Primitive::InHandPivot(_)
                | Primitive::InHandRotate(_)
                | Primitive::InHandTranslate(_)
                | Primitive::InHandRoll(_)
                | Primitive::InHandSlide(_)
                | Primitive::TransportHandoff(_)
                | Primitive::PlacePutDown(_)
                | Primitive::PlaceStack(_)
                | Primitive::PlaceInsertLoose(_)
                | Primitive::PlaceOrient(_)
                | Primitive::PlaceHandTo(_)
                | Primitive::PlaceDiscard(_)
                | Primitive::GraspRelease(_)
                | Primitive::ForcePull(_)
                | Primitive::SenseWeigh(_)
        )
    }

    /// If this is an `in_hand` DOF-moving op, its `*_inadmissible` failure name and whether the
    /// moved DOF is rotational (`spec/01` § DOF-admissibility). `None` for non-`in_hand` primitives.
    #[must_use]
    pub fn in_hand_move_kind(&self) -> Option<(&'static str, bool)> {
        match self {
            Primitive::InHandRotate(_) => Some(("rotation_inadmissible", true)),
            Primitive::InHandRoll(_) => Some(("roll_inadmissible", true)),
            Primitive::InHandPivot(_) => Some(("pivot_inadmissible", true)),
            Primitive::InHandTranslate(_) => Some(("translation_inadmissible", false)),
            Primitive::InHandSlide(_) => Some(("slide_inadmissible", false)),
            _ => None,
        }
    }

    /// The let-bound grasp handle this primitive references, if its `grasp_handle` is a let-ref
    /// (not `active`). Read by the GraspRef-supersession check (`spec/01` § 387).
    #[must_use]
    pub fn grasp_handle_ref(&self) -> Option<&str> {
        let handle = match self {
            Primitive::GraspRelease(p) => p.grasp_handle.as_ref(),
            Primitive::InHandRegrasp(p) => p.grasp_handle.as_ref(),
            Primitive::InHandPivot(p) => p.grasp_handle.as_ref(),
            Primitive::InHandRotate(p) => p.grasp_handle.as_ref(),
            Primitive::InHandTranslate(p) => p.grasp_handle.as_ref(),
            Primitive::InHandRoll(p) => p.grasp_handle.as_ref(),
            Primitive::InHandSlide(p) => p.grasp_handle.as_ref(),
            Primitive::TransportHandoff(p) => p.grasp_handle.as_ref(),
            Primitive::ForcePull(p) => p.grasp_handle.as_ref(),
            Primitive::SenseWeigh(p) => p.grasp_handle.as_ref(),
            _ => None,
        };
        match handle {
            Some(GraspHandle::Ref(name)) => Some(name.as_str()),
            _ => None,
        }
    }
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

/// `grasp.power` parameters (v0 subset of `$defs/GraspPowerParams`, § 2.2). Whole-volume
/// force-closure enclosure; lowers like `grasp.pinch` (force closure, friction_held) over the
/// `payload_grasp_power` payload. `enclosure_completeness` / `grasp_pose` carry spec defaults and
/// are not lowered in v0.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPower {
    /// The object target to enclose (a let-reference in the reference skill).
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

/// `grasp.lateral` parameters (v0 subset of `$defs/GraspLateralParams`, § 2.5). Key-grip force
/// closure across a thin dimension; lowers like `grasp.pinch` over `payload_grasp_lateral`. The
/// `grasp_edge` feature carries its spec default and is not lowered in v0.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspLateral {
    /// The object target to clamp (a let-reference in the reference skill).
    pub target: Ref,
    /// Max clamp force.
    pub force_budget: Quantity,
    /// Contact-confirmation criterion (default auto).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
    /// Reaction to detected slip.
    #[serde(default)]
    pub slip_response: Option<SlipResponse>,
}

/// `grasp.precision_tripod` parameters (v0 subset of `$defs/GraspPrecisionTripodParams`, § 2.4).
/// Three-point force closure resisting rotation about the grasp axis (`rotation_constrained`).
/// Lowers like `grasp.pinch` over `payload_grasp_tripod`. The triangle non-degeneracy (STB1) is
/// blocked (ε-table + un-reported contact geometry); v0 ships the `rotation_constrained` flag only.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPrecisionTripod {
    /// The small object target to grasp (a let-reference in the reference skill).
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

/// `grasp.hook` parameters (v0 subset of `$defs/GraspHookParams`, § 2.3). `target` + `load_budget`
/// required. Form-closure on a hookable feature: directional retention along `load_direction`, no
/// opposing squeeze (`load_budget` replaces `force_budget`), clamped to `hook_load_capacity`. The
/// `hook_feature` / `seating_force` carry spec defaults and are not lowered in v0.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspHook {
    /// The object target exposing a hookable feature (a let-reference in the reference skill).
    pub target: Ref,
    /// The load the engagement must support (replaces force_budget; clamped to hook_load_capacity).
    pub load_budget: Quantity,
    /// Primary supported load direction (default auto = anticipated load; carried symbolic in v0).
    #[serde(default)]
    pub load_direction: Option<Direction>,
    /// Contact-confirmation criterion (default auto = hook inner-curve seating).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}

/// `grasp.envelope` sub-mode (`$defs/GraspEnvelopeParams.mode`, § 2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeMode {
    /// Compliant contact following the object shape.
    Conform,
    /// A geometric trap with clearance (residual mobility).
    Cage,
}

fn envelope_mode_conform() -> EnvelopeMode {
    EnvelopeMode::Conform
}

/// `grasp.envelope` parameters (v0 subset of `$defs/GraspEnvelopeParams`, § 2.8). `target` +
/// `force_budget` required. Compliant / caging form-closure enclosure for fragile or imprecisely
/// localized objects on compliant / underactuated hands. The `mode` discriminates the two stability
/// classes (`conform` -> compliant friction_held, `cage` -> trapped with `residual_mobility`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspEnvelope {
    /// The object target to enclose (a let-reference in the reference skill).
    pub target: Ref,
    /// Gentle distributed enclosure force (clamped to grip_force_max).
    pub force_budget: Quantity,
    /// Enclosure sub-mode (default conform).
    #[serde(default = "envelope_mode_conform")]
    pub mode: EnvelopeMode,
    /// (`cage` mode) allowed residual object mobility inside the enclosure (carried symbolic).
    #[serde(default)]
    pub cage_clearance: Option<Quantity>,
    /// Contact-confirmation criterion (default auto).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}

impl GraspEnvelope {
    /// The grasp mode this envelope establishes (`conform` -> `EnvelopeConform`, `cage` ->
    /// `EnvelopeCage`).
    #[must_use]
    pub fn grasp_mode(&self) -> GraspMode {
        match self.mode {
            EnvelopeMode::Conform => GraspMode::EnvelopeConform,
            EnvelopeMode::Cage => GraspMode::EnvelopeCage,
        }
    }

    /// True for the `cage` sub-mode.
    #[must_use]
    pub fn is_cage(&self) -> bool {
        matches!(self.mode, EnvelopeMode::Cage)
    }
}

/// `grasp.pin` parameters (v0 subset of `$defs/GraspPinParams`, § 2.7). `target` +
/// `against_surface` + `force_budget` required. `against_surface` is the external
/// surface the object is pinned against; v0 floors the SurfaceTarget (point + normal)
/// to a frame name, as `force.wipe` floors its `surface`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPin {
    /// The object target to pin (a let-reference in the reference skill).
    pub target: Ref,
    /// The external surface to pin the object against (v0: a frame ref).
    pub against_surface: FrameRef,
    /// Normal force pressing the object onto the surface.
    pub force_budget: Quantity,
    /// Contact-confirmation criterion (default auto = effector contact + surface reaction).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}

/// `grasp.platform` parameters (v0 subset of `$defs/GraspPlatformParams`, § 2.6). Requires
/// `target` and `load_budget`. Support closure: the object is borne in balance over a support
/// polygon. The support-polygon geometry (`support_pose` / `support_normal`) is v0-deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GraspPlatform {
    /// The object target to support (a let-reference in the reference skill).
    pub target: Ref,
    /// Max supported weight (clamped to `payload_support`).
    pub load_budget: Quantity,
    /// Contact-confirmation criterion (default auto = distributed load + CoM in polygon).
    #[serde(default = "tactile_auto")]
    pub tactile_target: TactileTargetArg,
}

/// `in_hand.regrasp` parameters (v0 subset of `$defs/InHandRegraspParams`, § 3.3). Requires
/// `target_mode` (the grasp mode to transition to). Transitions a held object to a different
/// stable grasp without releasing it, using make-before-break (the new grasp is confirmed
/// before the old is released). The remaining params (contacts / force budget / preserve_pose /
/// tolerances) carry their spec defaults and are not lowered in v0 (serde ignores them).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandRegrasp {
    /// The grasp mode to transition to (v0: a mode name; may equal the current mode with
    /// different contacts).
    pub target_mode: String,
    /// The current grasp to transition from (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

impl InHandRegrasp {
    /// Map the `target_mode` name to a known v0 `GraspMode` (`pin` / `platform` / else `pinch`).
    /// The unknown-mode fallback to `pinch` keeps a force-closure contact swap (the common
    /// regrasp); the richer mode vocabulary lands with those modes.
    #[must_use]
    pub fn target_grasp_mode(&self) -> GraspMode {
        match self.target_mode.as_str() {
            "pin" => GraspMode::Pin,
            "platform" => GraspMode::Platform,
            _ => GraspMode::Pinch,
        }
    }
}

/// `in_hand.pivot` parameters (v0 subset of `$defs/InHandPivotParams`, § 3.5). Requires
/// `pivot_axis` (the released rotational DOF) and `angle`. Pivots a held object about a single
/// contact, releasing exactly the pivot-axis DOF while the others secure it (controlled
/// under-actuation, not a release); grasp identity is preserved. The drive mode and tolerances
/// carry their spec defaults and are not lowered in v0 (serde ignores them).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandPivot {
    /// The axis of the pivot rotation — the single DOF deliberately released.
    pub pivot_axis: Direction,
    /// Target swing angle about `pivot_axis` (carried symbolic in v0).
    pub angle: Quantity,
    /// The established grasp providing the pivot contact (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `in_hand.rotate` parameters (v0 subset of `$defs/InHandRotateParams`, § 3.1). Reorient a held
/// object about `axis` without releasing it, preserving grasp identity. `axis` + `angle` required;
/// `angle` is carried symbolic in v0. The rotation-admissibility check (reject a `form_held` /
/// `rotation_constrained` axis) is a class-1 validate check (wave 8), not lowered here.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandRotate {
    /// Rotation axis, in the grasp frame.
    pub axis: Direction,
    /// Signed rotation magnitude (carried symbolic in v0).
    pub angle: Quantity,
    /// The established grasp to manipulate within (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `in_hand.translate` parameters (v0 subset of `$defs/InHandTranslateParams`, § 3.2). Shift a held
/// object within the grasp envelope along `direction` by `distance`, preserving grasp identity.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandTranslate {
    /// Translation direction, in the grasp frame.
    pub direction: Direction,
    /// Translation magnitude.
    pub distance: Quantity,
    /// The established grasp to manipulate within (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `in_hand.roll` parameters (v0 subset of `$defs/InHandRollParams`, § 3.4). Continuously roll a
/// held object about `roll_axis` through rolling contact, preserving grasp identity. `angle` may
/// exceed 2π (continuous roll); carried symbolic in v0.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandRoll {
    /// Rolling axis, in the grasp frame (the object's rolling axis).
    pub roll_axis: Direction,
    /// Signed roll magnitude (carried symbolic in v0).
    pub angle: Quantity,
    /// The established grasp to roll within (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `in_hand.slide` parameters (v0 subset of `$defs/InHandSlideParams`, § 3.6). Slide a held object
/// along one contact surface (controlled slip along `slide_direction`), then re-secure at
/// `stop_condition`. The stop condition lowers into a Monitor; grasp identity is preserved.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandSlide {
    /// Direction of slip along the contact surface, in the grasp frame.
    pub slide_direction: Direction,
    /// `SlideStop` — what ends the slide; lowered into a Monitor.
    pub stop_condition: serde_yaml::Value,
    /// The established grasp to slide within (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `transport.handoff` parameters (v0 subset of `$defs/TransportHandoffParams`, § 4.3). Requires
/// `receiver` (the partner effector). Transfers a held object from the giver's grasp to the
/// receiver's, using two-party make-before-break. The handoff pose / tolerances carry their spec
/// defaults and are not lowered in v0 (serde ignores them).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportHandoff {
    /// The partner effector receiving the object (v0: a control-frame name).
    pub receiver: String,
    /// The grasp mode the receiver should form (v0: a mode name; `auto`/absent -> pinch).
    #[serde(default)]
    pub receiver_mode: Option<String>,
    /// Max combined force during the dual-grasp window (the co-grasp ceiling).
    #[serde(default)]
    pub cograsp_force_budget: Option<Quantity>,
    /// The giver's current grasp (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

impl TransportHandoff {
    /// The receiver's grasp mode (`pin` / `platform` / else `pinch`; `auto`/absent -> `pinch`).
    #[must_use]
    pub fn receiver_grasp_mode(&self) -> GraspMode {
        match self.receiver_mode.as_deref() {
            Some("pin") => GraspMode::Pin,
            Some("platform") => GraspMode::Platform,
            _ => GraspMode::Pinch,
        }
    }
}

/// `transport.lift` parameters (v0 subset of `$defs/TransportLiftParams`, § 4.5). Raise a held
/// object vertically by `height` along `up_direction` (default −gravity, declared not assumed).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportLift {
    /// Vertical lift distance.
    pub height: Quantity,
    /// The "up" direction (default −gravity); carried symbolic in v0.
    #[serde(default)]
    pub up_direction: Option<Direction>,
}

/// `transport.lower` parameters (v0 subset of `$defs/TransportLowerParams`, § 4.6). Lower a held
/// object vertically with controlled deceleration, keeping the grasp (release is separate).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportLower {
    /// Stop after a fixed descent (`height`) or on touchdown (default touchdown); carried symbolic.
    #[serde(default)]
    pub stop_mode: Option<String>,
    /// (`height` mode) descent distance; absent = until touchdown.
    #[serde(default)]
    pub height: Option<Quantity>,
    /// Descent direction (default gravity); carried symbolic in v0.
    #[serde(default)]
    pub down_direction: Option<Direction>,
}

/// `transport.follow_trajectory` parameters (v0 subset of `$defs/TransportFollowTrajectoryParams`,
/// § 4.2). Transport a held object tracking a caller-owned parameterized path. The `trajectory` is
/// carried opaque in v0; `timing_mode` (default time_scalable) selects the timing semantics.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportFollowTrajectory {
    /// The parameterized path of the held object (carried opaque in v0).
    pub trajectory: serde_yaml::Value,
    /// `strict` | `time_scalable` (default time_scalable — may slow for dynamic stability).
    #[serde(default)]
    pub timing_mode: Option<String>,
}

/// `transport.move_to_pose` parameters (v0 subset of `$defs/TransportMoveToPoseParams`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportMoveToPose {
    /// `Pose6D` floored as an inline object (a frame-relative offset) or a ref.
    pub target_pose: serde_yaml::Value,
}

/// `place.put_down` parameters (v0 subset of `$defs/PlacePutDownParams`, § 5.1). Place a held
/// object on a surface and release it. All fields default; v0 carries the placement target
/// symbolic and emits the GC1 held-leg floor + a controlled-release marker.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PlacePutDown {
    /// Where to place (default auto = the surface below); carried symbolic in v0.
    #[serde(default)]
    pub target_surface: Option<serde_yaml::Value>,
}

/// `place.stack` parameters (v0 subset of `$defs/PlaceStackParams`, § 5.2). Place a held object on
/// top of `support_object`, releasing after stack-stability confirmation (the recursive predicate
/// is deferred world-state geometry).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlaceStack {
    /// The object / stack to place on top of.
    pub support_object: Ref,
    /// How to align over the support's top face (default centered; carried symbolic in v0).
    #[serde(default)]
    pub alignment: Option<String>,
}

/// `place.insert_loose` parameters (v0 subset of `$defs/PlaceInsertLooseParams`, § 5.3). Drop a
/// held object into a container with clearance, releasing after containment confirmation (the
/// contained predicate is deferred world-state geometry).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlaceInsertLoose {
    /// The container / receptacle to insert into.
    pub container: Ref,
}

/// `place.orient` parameters (v0 subset of `$defs/PlaceOrientParams`, § 5.4). Place a held object
/// in a required orientation, verifying it before release.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlaceOrient {
    /// The orientation the placed object must have (carried symbolic in v0).
    pub required_orientation: serde_yaml::Value,
    /// Where to place (default auto); carried symbolic in v0.
    #[serde(default)]
    pub target_surface: Option<serde_yaml::Value>,
}

/// `place.hand_to` parameters (v0 subset of `$defs/PlaceHandToParams`, § 5.5). Hand a held object
/// to a human, releasing on weight transfer. The conjunctive `human_collaboration_safety` gate is
/// enforced at retarget; the HAZ3 weight-transfer bench is class 4 (deferred).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PlaceHandTo {
    /// Fraction of the object's weight the human must take before release (carried symbolic in v0).
    #[serde(default)]
    pub weight_transfer_threshold: Option<serde_yaml::Value>,
    /// Hard cap on force exchanged with the human (carried symbolic in v0).
    #[serde(default)]
    pub max_interaction_force: Option<Quantity>,
}

/// `place.discard` parameters (v0 subset of `$defs/PlaceDiscardParams`, § 5.6). Release a held
/// object into a coarse `discard_zone` without a precise final pose, guaranteeing the object lands
/// within the zone (the zone-containment thought shared with `in_hand.flip`'s safe_drop_zone).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PlaceDiscard {
    /// The region the object must land within.
    pub discard_zone: serde_yaml::Value,
    /// Cap on release height above the landing surface (carried symbolic in v0).
    #[serde(default)]
    pub max_drop_height: Option<Quantity>,
}

/// `disturbance_budget` argument (`$defs`, § 4.4): `Force | auto`. v0 lowers the explicit
/// `Force`; `auto` (derive from the min_holding_force margin) is deferred — no normative
/// formula exists (same posture as `coverage_overlap: auto`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum DisturbanceArg {
    /// The literal `auto` (deferred derivation).
    Auto(AutoLiteral),
    /// An explicit force budget.
    Force(Quantity),
}

/// `stability_margin` argument (§ 4.4): `Ratio | auto`. `auto` resolves to the descriptor's
/// `limits.stability_margin` (the M2-required embodiment default); an explicit ratio overrides.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum StabilityMarginArg {
    /// The literal `auto` (= the embodiment default limit).
    Auto(AutoLiteral),
    /// An explicit headroom ratio.
    Ratio(f64),
}

/// `transport.carry` parameters (v0 subset of `$defs/TransportCarryParams`, § 4.4). The
/// `MoveSpec` `motion` is carried opaquely — v0 lowers the `{to_pose: Pose6D}` form,
/// `{trajectory: T}` is deferred. The remaining § 4.4 params (grasp_handle / frame /
/// position_tolerance / max_velocity / max_acceleration / contact_response / timeout) carry
/// their spec defaults and are not emitted in v0 (serde ignores them).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransportCarry {
    /// `MoveSpec`: `{to_pose: Pose6D}` (lowered) or `{trajectory: T}` (deferred).
    pub motion: serde_yaml::Value,
    /// External perturbation the carry must reject (`Force | auto`).
    #[serde(default)]
    pub disturbance_budget: Option<DisturbanceArg>,
    /// Required holding-capacity headroom (`Ratio | auto`).
    #[serde(default)]
    pub stability_margin: Option<StabilityMarginArg>,
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

/// `reach.to_pose` parameters (v0 subset of `$defs/ReachToPoseParams`, § 1.1). The base free-space
/// motion to an absolute target pose, terminating at rest without task contact.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachToPose {
    /// `Pose6D` target, carried as an inline frame-relative object or a ref.
    pub target_pose: serde_yaml::Value,
}

/// `reach.approach` parameters (v0 subset of `$defs/ReachApproachParams`, § 1.2). Move to a
/// standoff pose offset from a target surface along its outward normal, terminating at rest.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachApproach {
    /// The surface to approach (v0: a frame ref; the point + normal is floored to a frame name).
    pub target: FrameRef,
    /// Terminal distance from the surface along its outward normal.
    pub standoff: Quantity,
    /// Controlled-frame axis aligned anti-parallel to the surface normal (carried symbolic in v0).
    #[serde(default)]
    pub approach_axis: Option<Direction>,
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
/// the target frame, the standoff, and the duration (carried, not lowered). The § 1.5
/// settling pair (`station_tolerance` / `settling_time`) is parsed and lowered when an
/// explicit `settling_time` opts in (ENV3); the remaining tracking parameters carry
/// spec defaults and are not emitted.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReachHover {
    /// The surface / frame to hover over (v0: a frame name).
    pub target: FrameRef,
    /// Maintained distance from `target` along its outward normal.
    pub standoff: Quantity,
    /// Hold duration (`Duration | until`; carried, symbolic in v0).
    #[serde(default)]
    pub duration: Option<serde_yaml::Value>,
    /// Allowed positional excursion from the hover setpoint (`spec/01` § 1.5, default 2 mm).
    #[serde(default)]
    pub station_tolerance: Option<Quantity>,
    /// Max time to return within `station_tolerance` after a disturbance (`Duration | auto`;
    /// an explicit Duration opts the hover into the ENV3 settling contract, `spec/01` § 1.5).
    #[serde(default)]
    pub settling_time: Option<serde_yaml::Value>,
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

/// `sense.probe` parameters (v0 subset of `$defs/SenseProbeParams`, § 7.1). A single light tactile
/// contact at `target_pose` to measure local surface properties — a measurement, no state change.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseProbe {
    /// Where to probe (carried as an inline frame-relative object or a ref).
    pub target_pose: serde_yaml::Value,
    /// The light force at which contact is registered (a measurement threshold, not actuation).
    pub contact_force: Quantity,
    /// What to measure on contact (default {presence, location}); carried as a marker.
    #[serde(default)]
    pub measure: Option<Vec<String>>,
    /// Hard cap guaranteeing non-disturbance (carried symbolic in v0).
    #[serde(default)]
    pub max_probe_force: Option<Quantity>,
}

/// `sense.verify` parameters (v0 subset of `$defs/SenseVerifyParams`, § 7.5). Check whether a
/// stated predicate over external state holds — the `sense` implementation of the algebra's
/// Predicate. The predicate is carried opaque in v0.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseVerify {
    /// The condition to check (a caller-defined criterion over observable state).
    pub predicate: serde_yaml::Value,
    /// Min confidence to assert true / false rather than indeterminate (carried symbolic).
    #[serde(default)]
    pub confidence_threshold: Option<serde_yaml::Value>,
}

/// `sense.weigh` parameters (v0 subset of `$defs/SenseWeighParams`, § 7.3). Estimate the mass (and
/// optionally CoM) of a held object from the measured force/torque reaction — the only `sense`
/// primitive that requires a held object; it produces the `estimated_mass` other categories consume.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SenseWeigh {
    /// `static` | `dynamic` | `auto` estimation method (default auto); carried as a marker.
    #[serde(default)]
    pub method: Option<String>,
    /// What to estimate (default {mass}); carried as a marker.
    #[serde(default)]
    pub measure: Option<Vec<String>>,
    /// The grasp holding the object to weigh (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
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

/// `force.push` parameters (v0 subset of `$defs/ForcePushParams`, § 6.2). Apply a controlled
/// directional force against a surface to hold / brace / press, without displacing it. The
/// static-force counterpart: v0 emits the `target_force` as the force-trajectory bound + the push
/// direction. `max_displacement` / `hold_duration` carry symbolic.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForcePush {
    /// The surface to push against (v0: a frame ref).
    pub target: FrameRef,
    /// The contact force to establish and hold (the force-trajectory bound).
    pub target_force: Quantity,
    /// Direction to apply force (default −target.normal, into the surface); carried symbolic.
    #[serde(default)]
    pub push_direction: Option<Direction>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `force.pull` parameters (v0 subset of `$defs/ForcePullParams`, § 6.3). Apply tensile force to
/// draw a held target toward the effector within a force budget, handling the breakaway moment.
/// Requires a grip / hook (a held object). The `stop_condition` lowers into a Monitor.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForcePull {
    /// Direction of tensile force, in the grasp frame.
    pub pull_direction: Direction,
    /// Max tensile force to apply (the force-trajectory bound).
    pub force_budget: Quantity,
    /// `PullStop` — distance / breakaway / tension; lowered into a Monitor.
    pub stop_condition: serde_yaml::Value,
    /// On a sudden resistance drop: `arrest` (default) or `continue`; carried as a marker.
    #[serde(default)]
    pub breakaway_response: Option<String>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
    /// The grasp on the target being pulled (default active).
    #[serde(default)]
    pub grasp_handle: Option<GraspHandle>,
}

/// `force.scrub` parameters (v0 subset of `$defs/ForceScrubParams`, § 6.9). Oscillating tangential
/// motion over a surface while regulating normal contact force (scrub / polish / sand). v0 emits
/// the `normal_force` band (the force-trajectory leg) + the oscillation amplitude marker; the
/// `completion` lowers into a Monitor. The tangential trajectory tracking is deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceScrub {
    /// The surface to scrub (v0: a frame ref).
    pub surface: FrameRef,
    /// Regulated contact force normal to the surface (the band setpoint).
    pub normal_force: Quantity,
    /// Tangential oscillation amplitude (carried as a marker).
    pub amplitude: Quantity,
    /// `ScrubStop` — duration / passes / state_change; lowered into a Monitor.
    pub completion: serde_yaml::Value,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `force.press_button` parameters (v0 subset of `$defs/ForcePressButtonParams`, § 6.6).
/// `target` + `actuation` + `force_budget` are required. v0 lowers the force_budget (the
/// force-trajectory leg) + the detent actuation marker; `press_direction` / `max_travel` /
/// `release_after` are schema-carried but symbolic in v0 (the over-travel guard is deferred).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForcePressButton {
    /// The button surface / frame to press (v0: a frame ref).
    pub target: FrameRef,
    /// `ActuationSpec` — what marks actuation: `detent` (v0) | `effort_rise(force_threshold)`.
    pub actuation: serde_yaml::Value,
    /// Max press force (over-travel / mechanism-damage limit) — the force-trajectory bound.
    pub force_budget: Quantity,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `force.wipe` parameters (v0 subset of `$defs/ForceWipeParams`, § 6.8). `surface` +
/// `wipe_path` + `normal_force` are required. v0 lowers the normal-force band (the
/// contact-maintenance leg); `wipe_path` is carried symbolic (the tangential position-tracking
/// leg is deferred). `normal_force_tolerance` auto defers (no band emitted).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceWipe {
    /// The surface / frame to wipe over (v0: a frame ref).
    pub surface: FrameRef,
    /// `Trajectory` — the tangential path (carried symbolic in v0; position-tracking deferred).
    pub wipe_path: serde_yaml::Value,
    /// The contact force to maintain normal to the surface (the band setpoint).
    pub normal_force: Quantity,
    /// Allowed deviation of the maintained normal force (`Force | auto`; explicit → the band).
    #[serde(default)]
    pub normal_force_tolerance: Option<serde_yaml::Value>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `force.snap_engage` parameters (v0 subset of `$defs/ForceSnapEngageParams`, § 6.10).
/// `mate_feature` + `engage_direction` + `force_budget` are required. v0 lowers the force_budget
/// (the force-trajectory leg) + the detent snap-in marker (reusing press_button's detent) + the
/// confirm_held engagement-confirmation marker. `mate_feature` is carried symbolic; the
/// documented reverse path (snap_disengage) is deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceSnapEngage {
    /// The bistable mechanism / receptacle to engage (carried symbolic in v0).
    pub mate_feature: serde_yaml::Value,
    /// Direction to drive engagement, in `frame`.
    pub engage_direction: Direction,
    /// Max engagement force (mechanism-break / over-force limit) — the force-trajectory bound.
    pub force_budget: Quantity,
    /// `ActuationSpec` — what marks snap-in: `detent` (default) | `effort_rise(force_threshold)`.
    #[serde(default)]
    pub snap_signature: Option<serde_yaml::Value>,
    /// Verify the bistable connection holds after engagement (release-test; default true).
    #[serde(default)]
    pub confirm_held: Option<bool>,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `force.cut` parameters (v0 subset of `$defs/ForceCutParams`, § 6.7). `cut_path` +
/// `shear_force_budget` + `completion` are required. v0 lowers the shear budget (the
/// force-trajectory leg) + an irreversible marker; `cut_path` is carried opaque (the
/// path-bounding leg is deferred); on_separation / tool_safety are deferred.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ForceCut {
    /// `Trajectory` — the path along which to cut (carried opaque in v0; path-bounding deferred).
    pub cut_path: serde_yaml::Value,
    /// Max shear force (tool-damage / over-cut / kickback limit) — the force-trajectory bound.
    pub shear_force_budget: Quantity,
    /// `CutStop` completion (path_complete / separation / depth), lowered into a Monitor.
    pub completion: serde_yaml::Value,
    /// Required compliance mode.
    #[serde(default)]
    pub compliance: Option<Compliance>,
}

/// `in_hand.flip` parameters (v0 subset of `$defs/InHandFlipParams`, § 3.7). The only
/// continuity-suspending primitive. `flip_axis` + `angle` + `max_release_time` + `safe_drop_zone`
/// are required. v0 lowers a symbolic reorientation about `flip_axis`; angle / max_release_time /
/// safe_drop_zone are carried symbolic (the bounded-window envelope geometry is deferred). The
/// momentary_release audit (AUD2) is verified in conformance, not lowered.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InHandFlip {
    /// Reorientation axis, in the grasp frame.
    pub flip_axis: Direction,
    /// Reorientation magnitude (carried symbolic in v0).
    pub angle: Quantity,
    /// Hard upper bound on the unsecured window (carried symbolic in v0).
    pub max_release_time: Quantity,
    /// Region below the operation where an uncaught object lands safely (carried symbolic; mandatory).
    pub safe_drop_zone: serde_yaml::Value,
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

    /// Validate the composition beyond parsing (conformance Test Class 1, embodiment-independent;
    /// the `rfl validate` CLI). Walks the AST tracking the active grasp's mode + binding name and
    /// the set of superseded grasp handles, enforcing the `spec/01` § Composition-validity rules
    /// (all decidable from `StabilityMetadata::for_mode`, no time trace):
    /// 1. unique `let`-binding names;
    /// 2. **DOF-admissibility** (§ 379): an `in_hand` DOF move is rejected if the active grasp has a
    ///    `form_held` DOF (any in-hand move) or the `rotation_constrained` flag (the rotational
    ///    moves: rotate / roll / pivot) — `*_inadmissible`. Conservative (no per-axis geometry);
    /// 3. **lifecycle `no_active_grasp`** (§ 383): a held-requiring primitive with no active grasp;
    /// 4. **STB3** (§ Composition validity by stability class): a `surface_bound` / `support` grasp
    ///    forbids a free-transport successor (`transport_inadmissible`);
    /// 5. **GraspRef-supersession** (§ 387): a `grasp_handle` naming a let-binding that a prior
    ///    `regrasp` / `handoff` superseded (`grasp_ref_superseded`).
    ///
    /// # Errors
    /// Returns `Error::SkillIsa` on any of the violations above.
    pub fn validate(&self) -> crate::Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        // The active grasp's mode (None when no grasp is held) and the let-name it is bound to.
        let mut active: Option<GraspMode> = None;
        let mut active_handle: Option<String> = None;
        // Let-bound grasp handles invalidated by a regrasp / handoff (spec/01 § 387).
        let mut stale: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for stmt in &self.body.sequence {
            // The operating primitive, and the let-name it binds (Some for a LetBind).
            let (prim, bind_name): (&Primitive, Option<&str>) = match stmt {
                Statement::LetBind(lb) => {
                    if !seen.insert(lb.r#let.as_str()) {
                        return Err(crate::Error::SkillIsa(format!(
                            "duplicate let-binding '{}'",
                            lb.r#let
                        )));
                    }
                    (lb.from.as_ref(), Some(lb.r#let.as_str()))
                }
                Statement::Primitive(p) => (p, None),
            };

            // 2. DOF-admissibility: an in-hand DOF move on a form_held / rotation_constrained grasp.
            if let Some((failure, rotational)) = prim.in_hand_move_kind() {
                if let Some(mode) = active {
                    let m = StabilityMetadata::for_mode(mode);
                    let form_held = m
                        .secured_dof
                        .values()
                        .any(|d| matches!(d, DofSecuring::FormHeld));
                    if form_held {
                        return Err(crate::Error::SkillIsa(format!(
                            "{failure}: a form_held grasp ({mode:?}) is not movable in-hand \
                             (spec/01 § DOF-admissibility)"
                        )));
                    }
                    if rotational && m.flags.rotation_constrained {
                        return Err(crate::Error::SkillIsa(format!(
                            "{failure}: a rotation_constrained grasp ({mode:?}) resists in-hand \
                             rotation (spec/01 § DOF-admissibility)"
                        )));
                    }
                }
            }

            // 3. Lifecycle: a held-requiring primitive with no active grasp.
            if prim.requires_held_grasp() && active.is_none() {
                return Err(crate::Error::SkillIsa(
                    "no_active_grasp: a held-requiring primitive has no active grasp \
                     (spec/01 § Lifecycle transitions)"
                        .to_string(),
                ));
            }

            // 4. STB3: a surface_bound (pin) or support-closure (platform) grasp cannot be freely
            // transported — both lose the object if moved freely.
            if prim.is_free_transport() {
                if let Some(mode) = active {
                    let m = StabilityMetadata::for_mode(mode);
                    if m.flags.surface_bound || m.closure == Closure::Support {
                        return Err(crate::Error::SkillIsa(format!(
                            "transport_inadmissible: a non-transportable grasp ({mode:?}) cannot \
                             be freely transported (spec/05 STB3)"
                        )));
                    }
                }
            }

            // 5. GraspRef-supersession: a grasp_handle naming a superseded let-binding.
            if let Some(handle) = prim.grasp_handle_ref() {
                if stale.contains(handle) {
                    return Err(crate::Error::SkillIsa(format!(
                        "grasp_ref_superseded: grasp handle '{handle}' was invalidated by a prior \
                         regrasp / handoff (spec/01 § GraspRef supersession)"
                    )));
                }
            }

            // State update. A regrasp / handoff supersedes the prior active grasp's handle.
            if prim.supersedes_grasp() {
                if let Some(old) = active_handle.take() {
                    stale.insert(old);
                }
            }
            if let Some(mode) = prim.establishes_grasp() {
                active = Some(mode);
                active_handle = bind_name.map(str::to_string);
            } else if prim.releases_grasp() {
                active = None;
                active_handle = None;
            }
        }
        Ok(())
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
    fn validate_accepts_unique_lets_and_rejects_duplicates() {
        assert!(cable_skill().validate().is_ok());
        let dup = "\
skill: dup
body:
  sequence:
    - let: x
      from:
        sense.locate:
          target_ref: connector
          modality: auto
    - let: x
      from:
        sense.locate:
          target_ref: connector
          modality: auto
";
        let err = Skill::parse_yaml(dup)
            .expect("parses")
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate let-binding 'x'"), "got: {err}");
    }

    #[test]
    fn stb3_rejects_free_transport_of_a_surface_bound_pin() {
        // grasp.pin (surface_bound) -> transport.move_to_pose: transport_inadmissible.
        let bad = "\
skill: pin-then-carry
body:
  sequence:
    - grasp.pin:
        target: part
        against_surface: workbench
        force_budget: 8 N
    - transport.move_to_pose:
        target_pose: { ref: dest }
";
        let err = Skill::parse_yaml(bad)
            .expect("parses")
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("transport_inadmissible"), "got: {err}");
        assert!(err.contains("Pin"), "names the surface_bound mode: {err}");
    }

    #[test]
    fn stb3_accepts_release_before_transport_and_a_pinch_transport() {
        // a pin released before the transport is legal (active grasp cleared).
        let released = "\
skill: pin-release-move
body:
  sequence:
    - grasp.pin:
        target: part
        against_surface: workbench
        force_budget: 8 N
    - grasp.release: {}
    - transport.move_to_pose:
        target_pose: { ref: dest }
";
        assert!(
            Skill::parse_yaml(released)
                .expect("parses")
                .validate()
                .is_ok()
        );
        // a pinch (NOT surface_bound) is freely transportable.
        let pinch = "\
skill: pinch-move
body:
  sequence:
    - grasp.pinch:
        target: part
        force_budget: 8 N
    - transport.move_to_pose:
        target_pose: { ref: dest }
";
        assert!(Skill::parse_yaml(pinch).expect("parses").validate().is_ok());
    }

    #[test]
    fn stb3_rejects_free_transport_of_a_support_grasp() {
        // grasp.platform (support closure) -> transport: a balanced object is not
        // freely transportable (spec/05 STB3 row 4).
        let bad = "\
skill: support-then-move
body:
  sequence:
    - grasp.platform:
        target: tray
        load_budget: 10 N
    - transport.move_to_pose:
        target_pose: { ref: dest }
";
        let err = Skill::parse_yaml(bad)
            .expect("parses")
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("transport_inadmissible"), "got: {err}");
        assert!(err.contains("Platform"), "names the support mode: {err}");
        // released first -> legal.
        let ok = "\
skill: support-release-move
body:
  sequence:
    - grasp.platform:
        target: tray
        load_budget: 10 N
    - grasp.release: {}
    - transport.move_to_pose:
        target_pose: { ref: dest }
";
        assert!(Skill::parse_yaml(ok).expect("parses").validate().is_ok());
    }

    #[test]
    fn dof_admissibility_rejects_in_hand_on_form_held_and_rotation_constrained() {
        // hook (form_held) -> ANY in-hand move is inadmissible.
        let hook_rotate = "\
skill: hook-rotate
body:
  sequence:
    - grasp.hook: { target: part, load_budget: 12 N }
    - in_hand.rotate: { axis: +z, angle: 90 deg }
";
        let err = Skill::parse_yaml(hook_rotate)
            .unwrap()
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("rotation_inadmissible"), "got: {err}");
        assert!(err.contains("form_held"), "names the reason: {err}");
        // hook (form_held) also rejects a translation.
        let hook_translate = "\
skill: hook-translate
body:
  sequence:
    - grasp.hook: { target: part, load_budget: 12 N }
    - in_hand.translate: { direction: +x, distance: 10 mm }
";
        assert!(
            Skill::parse_yaml(hook_translate)
                .unwrap()
                .validate()
                .unwrap_err()
                .to_string()
                .contains("translation_inadmissible")
        );
        // tripod (rotation_constrained) -> rotational moves inadmissible.
        let tripod_rotate = "\
skill: tripod-rotate
body:
  sequence:
    - grasp.precision_tripod: { target: part, force_budget: 5 N }
    - in_hand.roll: { roll_axis: +y, angle: 180 deg }
";
        let err = Skill::parse_yaml(tripod_rotate)
            .unwrap()
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("roll_inadmissible"), "got: {err}");
        assert!(
            err.contains("rotation_constrained"),
            "names the reason: {err}"
        );
    }

    #[test]
    fn dof_admissibility_accepts_friction_held_and_tripod_translation() {
        // pinch (friction_held, no flags) -> in-hand rotation is admissible.
        let pinch_rotate = "\
skill: pinch-rotate
body:
  sequence:
    - grasp.pinch: { target: part, force_budget: 8 N }
    - in_hand.rotate: { axis: +z, angle: 90 deg }
";
        assert!(Skill::parse_yaml(pinch_rotate).unwrap().validate().is_ok());
        // tripod permits TRANSLATION in-hand (only rotation is constrained).
        let tripod_translate = "\
skill: tripod-translate
body:
  sequence:
    - grasp.precision_tripod: { target: part, force_budget: 5 N }
    - in_hand.translate: { direction: +x, distance: 10 mm }
";
        assert!(
            Skill::parse_yaml(tripod_translate)
                .unwrap()
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn lifecycle_rejects_held_op_with_no_active_grasp() {
        let no_grasp = "\
skill: rotate-nothing
body:
  sequence:
    - in_hand.rotate: { axis: +z, angle: 90 deg }
";
        let err = Skill::parse_yaml(no_grasp)
            .unwrap()
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("no_active_grasp"), "got: {err}");
        // a free move of the empty effector (transport.move_to_pose) is NOT held-requiring.
        let free_move = "\
skill: free-move
body:
  sequence:
    - transport.move_to_pose: { target_pose: { ref: dest } }
";
        assert!(Skill::parse_yaml(free_move).unwrap().validate().is_ok());
    }

    #[test]
    fn grasp_ref_supersession_rejects_a_stale_handle() {
        // a let-bound grasp, superseded by a regrasp, then referenced -> grasp_ref_superseded.
        let stale = "\
skill: stale-handle
body:
  sequence:
    - let: g
      from:
        grasp.pinch: { target: part, force_budget: 8 N }
    - in_hand.regrasp: { target_mode: pinch }
    - in_hand.rotate: { axis: +z, angle: 90 deg, grasp_handle: g }
";
        let err = Skill::parse_yaml(stale)
            .unwrap()
            .validate()
            .unwrap_err()
            .to_string();
        assert!(err.contains("grasp_ref_superseded"), "got: {err}");
        assert!(err.contains("'g'"), "names the handle: {err}");
        // the active handle (no supersession) is fine.
        let live = "\
skill: live-handle
body:
  sequence:
    - let: g
      from:
        grasp.pinch: { target: part, force_budget: 8 N }
    - in_hand.rotate: { axis: +z, angle: 90 deg, grasp_handle: g }
";
        assert!(Skill::parse_yaml(live).unwrap().validate().is_ok());
    }

    #[test]
    fn connector_declares_estimated_mass() {
        let s = cable_skill();
        assert_eq!(
            s.objects["connector"].estimated_mass.as_ref().unwrap().0,
            "1.45 N"
        );
        assert!(s.objects["receptacle"].estimated_mass.is_none());
    }

    #[test]
    fn body_has_eight_statements_in_order() {
        let s = cable_skill();
        let stmts = &s.body.sequence;
        assert_eq!(stmts.len(), 8);
        assert!(matches!(&stmts[0], Statement::LetBind(_))); // let connector_t
        assert!(matches!(
            &stmts[1],
            Statement::Primitive(Primitive::GraspPinch(_))
        ));
        assert!(matches!(
            &stmts[2],
            Statement::Primitive(Primitive::TransportMoveToPose(_))
        ));
        assert!(matches!(&stmts[3], Statement::LetBind(_))); // let receptacle_t
        assert!(matches!(
            &stmts[4],
            Statement::Primitive(Primitive::ReachAlign(_))
        ));
        assert!(matches!(
            &stmts[5],
            Statement::Primitive(Primitive::ForceInsertFit(_))
        ));
        assert!(matches!(
            &stmts[6],
            Statement::Primitive(Primitive::GraspRelease(_))
        ));
        assert!(matches!(
            &stmts[7],
            Statement::Primitive(Primitive::ReachRetract(_))
        ));
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
        let Statement::Primitive(Primitive::ReachScan(p)) = &s.body.sequence[0] else {
            panic!()
        };
        assert_eq!(p.standoff.0, "100 mm");
        assert!(matches!(p.pattern, Some(ScanPattern::Raster)));
        assert!(matches!(
            &s.body.sequence[1],
            Statement::Primitive(Primitive::SenseInspect(_))
        ));
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

    #[test]
    fn transport_carry_parses() {
        let yaml = "skill: t\nbody:\n  sequence:\n    - transport.carry:\n        motion: { to_pose: { frame: staging, offset: { along: +z, distance: 100 mm } } }\n        disturbance_budget: 0.4 N\n        stability_margin: auto\n";
        let s = Skill::parse_yaml(yaml).expect("parse");
        let Statement::Primitive(Primitive::TransportCarry(p)) = &s.body.sequence[0] else {
            panic!("expected transport.carry");
        };
        assert!(p.motion.get("to_pose").is_some());
        assert!(matches!(
            p.stability_margin,
            Some(StabilityMarginArg::Auto(_))
        ));
        let Some(DisturbanceArg::Force(q)) = &p.disturbance_budget else {
            panic!("explicit force")
        };
        assert_eq!(q.0, "0.4 N");
    }
}
