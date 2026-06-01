# Composition-validity class-1 checks — DOF-admissibility + lifecycle + GraspRef-supersession

**Status**: design-complete, 2026-06-01. Implementation track wave 8 (the first class-1 checks
beyond STB3 / unique-let — now unblocked because the `form_held` (hook) and `rotation_constrained`
(tripod) modes and the `in_hand` manipulation primitives all exist).

## Why now

`Skill::validate()` (conformance Test Class 1, embodiment-independent, the `rfl validate` CLI) is
**separate from `retarget()`** — it walks the parsed AST and decides composition legality from
`StabilityMetadata` alone, with no time trace and no wire output. So extending it touches **no
retarget golden and no cert**; the only consumers are validate()'s own unit tests + the CLI.
Today it enforces only unique let-names and STB3 (free transport of a non-transportable grasp).
This wave adds the three `spec/01` § Composition-validity rules that the new primitives make
non-vacuous.

## The checks (all decidable from the AST + `StabilityMetadata::for_mode`)

### DOF-admissibility (`spec/01` § 379)

An `in_hand` op that moves a DOF is admissible only if that DOF is `friction_held` — never
`form_held` or `rotation_constrained`. v0 rule (conservative, no per-axis geometry):

- if the active grasp has **any `form_held` secured_dof** → reject every in-hand DOF move
  (`rotation_inadmissible` / `translation_inadmissible` / `roll_inadmissible` /
  `slide_inadmissible` / `pivot_inadmissible`). Bites `grasp.hook` (load_direction form_held).
- if the active grasp sets the **`rotation_constrained` flag** → reject the *rotational* in-hand
  moves (rotate / roll / pivot), not the translational ones (translate / slide). Bites
  `grasp.precision_tripod`.

Conservative because v0 cannot match "the moved axis" to "the constrained axis" (geometry,
Principle 4) — it rejects the whole class rather than the specific axis. Documented as such.

### Lifecycle — `no_active_grasp` (`spec/01` § 383)

A primitive requiring a held object rejects a free frame. The held-requiring set: all `in_hand.*`,
all `place.*`, all `transport.*`, `grasp.release`, `force.pull`, `sense.weigh`. Using one with no
active grasp → `no_active_grasp`. (`reach.*` / `sense.{locate,probe,verify,inspect}` / `force` non-
pull / `grasp.*` establishers don't require held.)

### GraspRef-supersession (`spec/01` § 387)

`in_hand.regrasp` / `transport.handoff` produce a new GraspState and supersede the originating
`GraspRef`. A `GraspRef` bound by a `LetBind` (`let g = grasp.pinch{...}`) is invalidated the moment
it is superseded; a later primitive whose `grasp_handle` names the stale binding is a composition
error (`grasp_ref_superseded`). The walk tracks the active grasp's binding name; on a
regrasp/handoff it marks that name stale; a `grasp_handle: <stale>` reference rejects.

## Plumbing

Three new `Primitive` helpers: `requires_held_grasp()`, `in_hand_move_kind() -> Option<(&str name,
bool rotational)>`, `grasp_handle_ref() -> Option<&str>`. `validate()`'s walk gains the active
grasp's binding name + a stale set, and applies the three checks per statement (including
grasp-establishing `LetBind`s, which now update the active grasp).

## Deferred

- Per-axis DOF matching (reject only the constrained axis) — needs contact/axis geometry.
- Last-resort admissibility (`flip` only when no continuity-preserving alternative) + reverse-
  dependency (`place.orient` reorient-first) — planner-level, not a local AST rule.
- LetBind uncertainty-matching / measured-value-supply flow checks — need uncertainty bounds.
