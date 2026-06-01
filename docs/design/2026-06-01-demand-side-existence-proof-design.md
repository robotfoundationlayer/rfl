# Design: demand-side existence proof — a foundation model emits RFL Skill ISA

Status: approved design, pre-implementation (2026-06-01). The brainstorm settled three
decisions: (i) **hybrid** approach — a planner-adapter contract + a deterministic mock +
one real Claude-backed emitter; (ii) **proof depth** = emit → schema-validate → retarget
onto a real descriptor, reporting a pass rate over novel tasks; (iii) **placement** = a
separate repo depending on the published RFL binding, with this (`rfl`) repo gaining only
this design doc. Working codename **`cobel`** (the demand-side counterpart to **Milchick**;
personal namespace `masterleopold/cobel`, Private until the proof runs).

This is the **demand-side** existence proof for RFL, the matched half of the supply-side
proof ([Milchick](./2026-05-31-pincherx-existence-proof-design.md)). Milchick proves the
retarget contract is constructible against a real *embodiment*; `cobel` proves a real
*foundation model* can emit the Skill ISA the contract consumes. Together they bracket the
README roadmap's v0.0.1 milestone ("single (VLA, embodiment) pair existence proof"): Milchick
holds the embodiment fixed and the planner trivial; `cobel` holds the planner real and the
embodiment a set of descriptors. The only gap left after both stand is the closed loop
(`cobel`'s emitted skill → Milchick's arm), which is then a trivial composition.

## 1. Why this proof, and what it does / does not prove

Every prior artifact — the 25 in-process conformance increments and the Milchick
real-embodiment proof — is on the **supply** side (embodiment / driver). The one untested
core hypothesis is the demand side: **can a real foundation model actually emit the RFL
Skill ISA?** Milchick hand-authored its skill; this proof makes the emitter real.

**Proves:** a real frontier foundation model (Claude, via the Anthropic API), given the Skill
ISA spec + a task + a scene, emits **valid, schema-conformant, retargetable** Skill ISA
across **novel** tasks (not the `examples/` skills), at a **measured pass rate** — including
**spec-only / blind** emission (the model is shown the grammar but no example skills). It
establishes the Skill ISA as a *targetable emission surface* for the planner tier, and closes
the **demand → neutral** seam (the mirror of Milchick's neutral → supply seam).

**Does not prove:** that *action-token* VLAs (π0 / OpenVLA / Octo) emit RFL — they operate
**below** the Skill ISA. Their output is low-level continuous motor action (≈ end-effector
deltas + gripper at tens of Hz), which is structurally the **canonical-action / Driver
Interface** layer (`spec/03`), not the symbolic Skill ISA (`spec/01`). The Skill ISA's
natural emitter is the **planner / VLM tier** — the Code-as-Policies / SayCan / "System-2
VLM" archetype that emits a typed skill program. A general frontier LLM/VLM is exactly that
archetype. Also not proven: that emitted skills are behaviorally optimal or execute on real
hardware (that is the closed-loop capstone / Milchick); semantic correctness beyond schema +
retarget + light checks; that any specific deployed robot product emits RFL.

**Stating the layer boundary honestly is part of the artifact.** The claim is narrow and
defensible: the planner tier can target the ISA. It is not "today's deployed motor-control
VLAs speak RFL."

## 2. Design principle: the planner never sees the embodiment

Principle 1 (embodiment-agnostic) is honored *inside the proof itself*. The model is given
**task + scene only** and emits an embodiment-agnostic skill. The **embodiment descriptor
enters only at the harness's retarget step.** Consequences:

- A legitimate skill that a given embodiment cannot satisfy is **correctly rejected by the
  capability gate** (`translation error: capability_absent: …`). The harness records that as a
  *valid emission with a documented embodiment mismatch*, **distinct** from a *malformed
  emission* (a real failure). This is on-thesis: it exercises the exact seam RFL mediates.
- One emitted skill is retargeted against **several** descriptors (Allegro / LEAP / pneumatic
  from `examples/01`, and `pincherx-100` from Milchick), demonstrating the agnostic→bound
  fan-out that is the whole point of the layer.

## 3. The planner-adapter contract (demand-side `ReferenceDriver` analogue)

A minimal Python interface — the demand-side counterpart of the in-process `ReferenceDriver`:

```python
class Planner(Protocol):
    def plan(self, task: TaskSpec, scene: Scene) -> str:   # returns Skill ISA YAML text
        ...
```

- `TaskSpec` = `{ instruction: str, success_criteria: str | None }`
- `Scene` = `{ objects: [Object], frames: [str], notes: str | None }`
- `Object` = `{ ref: str, kind: str, est_mass: str | None, geometry: str | None,
  pose_hint: str | None, features: [str] | None }` — mirrors the `objects:` block of the
  `examples/*/skill.yaml`. The emitted skill must reference only these `ref`s.

The return value is **Skill ISA YAML text**, so it feeds straight into the published binding
`rfl.retarget(skill_yaml, descriptor_yaml) -> str`. String-in/string-out keeps the contract
stable as primitive coverage / the canonical format evolve (same property the binding relies
on).

This interface is a *reference within `cobel`*, **not** a new normative RFL spec concept. A
"planner / emitter conformance" tier in `spec/` would be a future spec increment and is
explicitly deferred (§ 12) — the spec/conformance/crates tree is frozen.

## 4. The two emitters

**`MockPlanner` (deterministic, offline).** Returns a hand-authored *reference skill* per task
id from a static map. No network, no key. It is the CI gate and the "reference planner": it
proves the contract is implementable, the harness runs end-to-end offline, and the reference
skills schema-validate + retarget. Its canned skills are **novel** (fresh tasks, not the
`examples/` skills).

**`ClaudePlanner` (the real existence proof).** Calls the Anthropic API.

- **System prompt** = a distilled Skill ISA spec: the 50-primitive catalogue (7 categories),
  the compositional algebra (`sequence` / `parallel` / `reactive` / `repeat` / `branch` /
  `let`-bind), the type-system rules (SI canonical units; target types via `let` from
  `sense.*`), and a strict output contract ("emit ONLY a Skill ISA YAML document, no prose").
- **Two modes**, reported separately:
  - *spec-only (blind)* — grammar + primitives + type rules, **no example skills**. The
    strongest claim; this is the headline number.
  - *few-shot* — additionally shows 1–2 `examples/*/skill.yaml` as format anchors. Easier,
    weaker; reported as the upper bound.
- **Stochasticity** — LLM output is non-deterministic. Sample N completions per task, gate
  each, and report **pass@1 and pass@k** (an honest measurement, not a binary). Malformed or
  un-fenced output counts as a failure in the rate.
- **Prompt caching** — the large spec system prompt is reused across every task → cache it
  (`cache_control`). Drive the SDK details via the `claude-api` skill at implementation time.
- **Model + SDK** — default to the latest capable Claude; the exact model ID, the `anthropic`
  package surface, the Messages API shape, and `cache_control` usage are **confirmed in-session
  from official docs at implementation time** (never written from memory — product-facts rule).

## 5. Scene + task input format

Each task is a YAML file the harness loads and the planner consumes:

```yaml
task:
  instruction: "Pour the contents of the cup into the bowl, then set the cup down upright."
  success_criteria: "cup emptied into bowl; cup placed upright on the table"
scene:
  objects:
    - { ref: cup,  kind: open_container, est_mass: 2.0 N, features: [handle, rim] }
    - { ref: bowl, kind: open_container }
    - { ref: table, kind: surface }
  frames: [world, table]
  notes: "cup starts on the table, ~150 mm from the bowl"
```

The planner emits a skill whose `objects:` reference only `{cup, bowl, table}` and whose body
composes existing primitives — the **model chooses** the composition (there is no `pour`
primitive; pouring is expressed as e.g. `sense.locate` → `grasp.power` →
`transport.move_to_pose` to a tilted pour pose → `place.put_down`). The scene gives the model
the world; it does not give it the embodiment.

## 6. The validation harness (the gates)

For each emitted `skill_yaml`, against each target descriptor:

1. **Schema (secondary gate).** `jsonschema` Draft 2020-12 against a **vendored, commit-pinned**
   copy of `schemas/skill-isa.schema.json` (the source commit hash recorded in the vendored
   file's header). Redundant with — but independent of — the engine; also exercises the
   schema's typed slices (e.g. `grasp.pinch`).
2. **Retarget (primary gate).** `rfl.retarget(skill_yaml, descriptor_yaml)` via the published
   binding. Outcomes:
   - **retarget_ok** — produces canonical-action JSONL (one `execute` per line; correlation
     field `action_id`). The skill is well-formed AND retargetable: the proof.
   - **capability_rejected** — `RetargetError` whose message matches `capability_absent`. The
     emission is *valid*; this descriptor simply lacks a capability the task needs. Recorded as
     an embodiment mismatch, **not** an emission failure.
   - **malformed** — any other parse/retarget error. A real emission failure.
3. **Light semantic checks.** Emitted `objects:` ⊆ scene objects; body references resolve;
   primitive category is plausible for the task. Soft signals, not hard gates (we do not
   over-claim automated semantic verification).

**Output = a pass-rate table** (task × descriptor × mode × {pass@1, pass@k, rejected,
malformed}). That table, committed under `results/`, is the published artifact.

## 7. Task suite (~6 novel tasks, spanning categories)

Chosen to (a) avoid the `examples/` skills (cable-insertion / surface-scan / screw-fasten) so
nothing is template-copied, and (b) span categories incl. tasks some descriptors must reject:

| Task | Exercises | Note |
|---|---|---|
| pour | grasp + transport + `in_hand.rotate` + place | reorientation |
| open-drawer | grasp handle + `force.pull` | rejected by no-F/T descriptors → mismatch path |
| stack-blocks | grasp + transport + `place.stack` | alignment |
| press-button-sequence | reach + `force.press_button` ×N in `sequence` | repetition |
| wipe-surface | `force.wipe` | contact maintenance; rejected by no-F/T descriptors |
| pick-place-light | `grasp.pinch` + transport + `place.put_down` | cross-validates Milchick |

The `force.*` tasks deliberately retarget-reject on `pincherx-100` (no F/T) and Allegro-minus
descriptors — proving the negative-space gate from the demand side.

## 8. Honest caveats (the substance of the proof)

- **General model, not a deployed robotics VLA.** Claude is a frontier LLM/VLM, i.e. the
  *planner* archetype — not a motor-control VLA. The claim is scoped accordingly (§ 1).
- **Pass rate, not "it works".** The honest unit is *how often* a real model hits a valid,
  retargetable emission, reported as pass@1 / pass@k across modes. Variance is the result.
- **Spec-only is the real claim.** Few-shot numbers are an upper bound; the headline is the
  blind/spec-only rate.
- **Schema is vendored + pinned.** The proof is a snapshot against a recorded `rfl` commit;
  the vendored schema can drift from `main` until refreshed (recorded, not hidden).
- **No hardware, no semantics-beyond-retarget.** Execution and behavioral optimality are out
  of scope (Milchick / the capstone own those).

## 9. Repos and components

The work lives in a **separate repo** to keep RFL provider-neutral (a real Anthropic
integration is a *provider* integration → not in the neutral org) and to respect the frozen
`rfl` spec/conformance/crates tree. The pattern mirrors Milchick (Principle 4).

**`cobel` (separate repo — the demand-side proof).** Personal namespace `masterleopold/cobel`,
Private until the proof runs. Depends on the published RFL binding.

```
cobel/
  pyproject.toml            # deps: rfl (path/git → pip post-PyPI), anthropic, pyyaml, jsonschema; dev: pytest
  planner/
    contract.py             # TaskSpec, Scene, Object, Planner Protocol
    mock.py                 # MockPlanner (canned reference skills)
    claude.py               # ClaudePlanner (Anthropic SDK; spec-only + few-shot; prompt caching)
    prompts/skill-isa-system.md   # distilled spec system prompt
  harness/
    validate.py             # schema (vendored) + retarget gates; pass-rate report
    schema/skill-isa.schema.json  # vendored, header records source rfl commit hash
    descriptors/            # copies of example descriptors (+ pincherx-100), source commit recorded
  tasks/*.yaml              # task + scene per task
  reference_skills/*.yaml   # the mock's canned novel skills
  run.py                    # --planner mock|claude --mode spec-only|few-shot --tasks ...
  tests/                    # offline CI: every mock skill schema-validates + retargets
  results/                  # generated pass-rate tables (the published artifact)
  README.md
```

**CI runs the mock path offline** (no API key). The Claude path is run manually (needs a key)
and its results table is committed.

**`rfl` (this repo — the neutral standard).** Gains **only this design doc**. The binding
(`bindings/python/`) and `schemas/skill-isa.schema.json` are **consumed**, never edited;
`bindings/python/` is the Milchick lane and is untouched.

## 10. Data flow

1. `run.py` loads a task file → `(TaskSpec, Scene)`.
2. The planner (`mock` or `claude`) emits `skill_yaml`. For `claude`: build messages (cached
   spec system prompt + task/scene), sample N, extract YAML from each completion.
3. For each target descriptor: `harness.validate` runs the schema gate, then
   `rfl.retarget(skill_yaml, descriptor_yaml)` (the published binding) → classify
   {retarget_ok | capability_rejected | malformed}, plus the light semantic checks.
4. Aggregate into the pass-rate table under `results/`.

The harness is the demand-side embodiment of the supply-side `ReferenceDriver`: same retarget
contract, but the *input* skill now comes from a real model instead of a fixture.

## 11. Division of labor / sessions (Milchick mirror)

- **This session:** write this design doc → commit + push to `rfl` `main`. `rfl` gets only
  the doc.
- **New session (`cobel` repo):** create `masterleopold/cobel`; implement
  contract → mock + offline harness/tests (CI-green plumbing first) → `ClaudePlanner` + task
  suite → run the proof → commit the `results/` table. `writing-plans` → `executing-plans`,
  inline TDD; plans LOCAL-ONLY under the repo's `docs/plans/`.
- **Publish gate (when the proof runs):** flip `cobel` Public + Apache 2.0 + add an `rfl`
  README "reference planner: masterleopold/cobel" link. Not before (same discipline as
  Milchick — no link while Private).

## 12. Out of scope (deferred)

- **Action-token VLA → Skill ISA mapping** (π0 / OpenVLA / Octo). Targets the wrong layer
  (sub-ISA motor actions); lifting continuous trajectories to typed symbolic skills is a
  lossy, research-grade inverse-abstraction problem and would demonstrate *us bucketing*, not
  *the model targeting RFL*. Documented here as the honest reason it is excluded.
- **The closed loop** (`cobel` emit → Milchick execute on Gazebo/physical). The capstone;
  becomes trivial once both halves stand. Couples to Milchick hardware bringup (not ready).
- **A normative "planner / emitter conformance" tier in `spec/`.** A real, on-thesis spec
  improvement (the demand-side analogue of `spec/05`), but the spec/conformance/crates tree is
  frozen; the `cobel` adapter is a reference, not a spec artifact.
- **Shipping the schema in the `rfl` wheel.** Would touch `bindings/python/` (Milchick lane);
  the proof vendors a pinned copy instead.
- **Multi-provider emitters** (other foundation models). One real emitter suffices for the
  existence proof; the contract is provider-agnostic so others are additive later.

## 13. Confirm during implementation (transcribe from source / verify in-session)

- **Anthropic SDK** — the `anthropic` package, client init, the Messages API shape, the
  **current model ID**, `cache_control` prompt-caching usage, and response/text extraction:
  verified in-session via the `claude-api` skill + official docs (never from memory —
  product-facts rule). API key from the environment, never committed.
- **The published binding** — `rfl.retarget(skill_yaml, descriptor_yaml) -> str` and
  `rfl.RetargetError`, from `bindings/python/src/lib.rs`; the canonical correlation field is
  `action_id` (confirmed at `canonical.rs`).
- **The schema + descriptors** — copy `schemas/skill-isa.schema.json` and the
  `examples/01-cable-insertion/embodiments/*.yaml` (+ Milchick's `pincherx-100.yaml`) from a
  recorded `rfl` commit hash, written into the vendored files' headers.
- **The example skills** — `examples/*/skill.yaml` as the few-shot anchors and the "novelty"
  exclusion set for the task suite.
