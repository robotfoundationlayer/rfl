# LetBind measured-value-supply flow check — design + golden-coupling finding

Status: design-complete; implementation **deferred** (golden-coupling, see § Risk)

## Spec obligation

`spec/01` § Composition validity → `LetBind` flow checks (lines 397–402):

> **Measured-value supply (the observe–act loop).** A `Measurement`'s `mass` /
> `center_of_mass` / `pose` fields populate the matching `ObjectTarget` fields
> via the binding, after which the downstream primitive's preconditions (e.g.
> `estimated_mass ≤ payload`) become checkable.

The mass-consuming preconditions are the per-grasp payload bounds:
`target.estimated_mass ≤ embodiment.limits.payload_grasp_{pinch,power,tripod,lateral,support,envelope}`
(`spec/01` lines 842, 894, 999, 1053, 1104, 1203). `grasp.pin` is exempt (the
external surface bears the load); `sense.weigh` (§ 7.3) is the in-composition
*producer* of `estimated_mass`.

## Intended check (class-1, in `Skill::validate`)

Extend the existing sequential validate walk with a per-statement resolution:

1. Build `let_source: name → &producer` incrementally (lets precede uses).
2. For a primitive carrying a mass precondition (the six grasps above), resolve
   its `target` (a `Ref` = `String`):
   - a let-name bound from `sense.weigh` → mass measured, supplied;
   - a let-name bound from `sense.locate { target_ref: o }` → supplied iff
     `objects[o].estimated_mass` is set;
   - a direct object name → supplied iff `objects[name].estimated_mass` is set.
3. If unsupplied, the `estimated_mass ≤ payload` precondition is uncheckable —
   a composition error (the dangling-reference sibling of GraspRef-supersession,
   which `spec/01` § 387 explicitly groups it with).

`Skill::validate` is not called from `retarget`, so the *check itself* touches no
retarget golden.

## Risk: the FIX is not golden-safe (the deferral reason)

Making the check strict requires every mass-precondition grasp's target to
**supply** `estimated_mass`. A survey of the example corpus shows at least
`examples/03-screw-fasten/skill-envelope.yaml` uses `grasp.envelope` with **no
`estimated_mass`** declared. Satisfying a strict check means adding
`estimated_mass` to that skill's objects — and `estimated_mass` feeds the
GF1c–GF4c grasp-force derivations (`spec/02` § 133–147), which set
`min_holding_force` in the **lowered canonical action**. So the fix would churn
that skill's retarget goldens (and any cert embedding them). This is the
opposite of a golden-safe class-1 increment.

There is also a genuine spec-ambiguity: `spec/01` frames the supply as what makes
the precondition *checkable* (the L1 Data loop), not unambiguously as a hard
declaration requirement — so a strict "every grasp must declare mass" error may
be over-strict. `Skill::validate` has no soft-warning channel (it returns
`Result`), so the choice is hard-error or nothing.

## Decision

Defer. The correct increment is: (a) decide hard-vs-soft with the spec owner,
(b) if hard, complete the example corpus's `estimated_mass` declarations and
re-bless the affected GF-derived goldens as one deliberate change, (c) then land
the resolution check. Bundling (b) blindly under "class-1, golden-safe" would
silently rewrite force-derivation goldens, so it is recorded here rather than
forced. The narrower **dangling-target-reference guard** (target resolves to a
declared object or a bound let, mass aside) is a clean future sub-increment that
shares the resolution machinery.
