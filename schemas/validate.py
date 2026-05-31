#!/usr/bin/env python3
"""Conformance test class 1 (parser conformance) for the RFL schemas.

Validates the JSON Schemas themselves, the worked-example reference instances
against them, and the cross-schema consistency invariants that keep
`skill-isa.schema.json` and `embodiment-descriptor.schema.json` from drifting
apart. Exits non-zero on any failure.

Run from anywhere (paths resolve relative to this file):

    uv run --with jsonschema --with pyyaml python schemas/validate.py

`jsonschema` and `pyyaml` are not vendored; the ephemeral `uv run` above
supplies them without polluting any environment.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    import yaml
    from jsonschema import Draft202012Validator
except ModuleNotFoundError as exc:  # pragma: no cover - environment guard
    sys.exit(
        f"missing dependency {exc.name!r}; run via:\n"
        "    uv run --with jsonschema --with pyyaml python schemas/validate.py"
    )

ROOT = Path(__file__).resolve().parent.parent
SCHEMAS = ROOT / "schemas"
EXAMPLES = ROOT / "examples" / "01-cable-insertion"

# The four reach.* ids carry no capability key (reach is the assumed-present
# baseline), and these four category keys are the manifest category gates.
REACH_PREFIX = "reach."
CATEGORY_GATES = {"force", "in_hand", "sense", "transport"}

failures: list[str] = []


def check(label: str, ok: bool, detail: str = "") -> None:
    mark = "ok  " if ok else "FAIL"
    print(f"  [{mark}] {label}" + (f" — {detail}" if detail and not ok else ""))
    if not ok:
        failures.append(label + (f": {detail}" if detail else ""))


def load_schema(name: str) -> dict:
    return json.loads((SCHEMAS / name).read_text())


def main() -> int:
    skill = load_schema("skill-isa.schema.json")
    descriptor = load_schema("embodiment-descriptor.schema.json")
    driver = load_schema("driver-interface.schema.json")

    print("Schema well-formedness (JSON Schema Draft 2020-12)")
    for label, schema in (
        ("skill-isa", skill),
        ("embodiment-descriptor", descriptor),
        ("driver-interface", driver),
    ):
        try:
            Draft202012Validator.check_schema(schema)
            check(f"check_schema {label}", True)
        except Exception as exc:  # noqa: BLE001 - report any schema defect
            check(f"check_schema {label}", False, repr(exc))

    print("\nReference instances validate")
    sk_validator = Draft202012Validator(skill)
    desc_validator = Draft202012Validator(descriptor)

    skill_yaml = EXAMPLES / "skill.yaml"
    errs = list(sk_validator.iter_errors(yaml.safe_load(skill_yaml.read_text())))
    check(f"skill.yaml vs skill-isa", not errs, errs[0].message if errs else "")

    for emb in sorted((EXAMPLES / "embodiments").glob("*.yaml")):
        errs = list(desc_validator.iter_errors(yaml.safe_load(emb.read_text())))
        check(f"{emb.name} vs embodiment-descriptor", not errs, errs[0].message if errs else "")

    drv_validator = Draft202012Validator(driver)
    for msg in sorted((EXAMPLES / "driver-messages").glob("*.yaml")):
        errs = list(drv_validator.iter_errors(yaml.safe_load(msg.read_text())))
        check(f"{msg.name} vs driver-interface", not errs, errs[0].message if errs else "")

    print("\nCross-schema consistency (anti-drift)")

    # C1 — the descriptor's capability-key enum is exactly the skill-isa primitive
    # set minus reach.* plus the four category gates. Derived from skill-isa at
    # check time so the two schemas cannot drift apart silently.
    prim = set(skill["$defs"]["PrimitiveId"]["enum"])
    derived = {p for p in prim if not p.startswith(REACH_PREFIX)} | CATEGORY_GATES
    skills_items = descriptor["$defs"]["Capabilities"]["properties"]["skills"]["items"]
    desc_enum = next((set(b["enum"]) for b in skills_items["oneOf"] if "enum" in b), set())
    drift = (derived ^ desc_enum)
    check("C1 capability enum == skill-isa PrimitiveId − reach.* + gates", not drift,
          f"symmetric difference {sorted(drift)}")

    # C2 — the extension-key pattern is shared between the two schemas.
    sk_ext = skill["$defs"]["ExtensionCall"]["propertyNames"]["pattern"]
    desc_ext = next((b["pattern"] for b in skills_items["oneOf"] if "pattern" in b), None)
    check("C2 extension pattern shared", sk_ext == desc_ext, f"{desc_ext!r} != {sk_ext!r}")

    # C3 — the closed-core tactile feature set is identical in both schemas
    # (skill-isa's TactileFeature and the descriptor's contact-sensor features).
    sk_core = set(skill["$defs"]["TactileFeature"]["oneOf"][0]["enum"])
    desc_core = set(
        descriptor["$defs"]["TactileEntry"]["properties"]["features"]["items"]["oneOf"][0]["enum"]
    )
    tactile_drift = sk_core ^ desc_core
    check("C3 tactile closed-core features identical", not tactile_drift,
          f"symmetric difference {sorted(tactile_drift)}")

    # C4 — the driver-interface telemetry tactile-feature core is identical to
    # skill-isa's, so telemetry feature keys cannot drift from the manifold
    # taxonomy. Reuses sk_core from C3.
    drv_core = set(driver["$defs"]["TactileFeature"]["oneOf"][0]["enum"])
    drv_drift = sk_core ^ drv_core
    check("C4 tactile closed-core identical (driver-interface telemetry)", not drv_drift,
          f"symmetric difference {sorted(drv_drift)}")

    print()
    if failures:
        print(f"FAILED — {len(failures)} check(s):")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("PASS — all conformance test class 1 checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
