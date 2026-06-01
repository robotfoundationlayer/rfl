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
EXTENSIONS = ROOT / "extensions"

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
    adapter = load_schema("tactile-manifold/adapter.schema.json")
    certificate = load_schema("certificate.schema.json")
    extension = load_schema("extension-registry.schema.json")
    epsilon = load_schema("epsilon-tolerance.schema.json")

    print("Schema well-formedness (JSON Schema Draft 2020-12)")
    for label, schema in (
        ("skill-isa", skill),
        ("embodiment-descriptor", descriptor),
        ("driver-interface", driver),
        ("tactile-manifold adapter", adapter),
        ("certificate", certificate),
        ("extension-registry", extension),
        ("epsilon-tolerance", epsilon),
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

    adapter_validator = Draft202012Validator(adapter)
    for ad in sorted((SCHEMAS / "tactile-manifold").glob("*.yaml")):
        errs = list(adapter_validator.iter_errors(yaml.safe_load(ad.read_text())))
        check(f"{ad.name} vs tactile-manifold adapter", not errs, errs[0].message if errs else "")

    cert_validator = Draft202012Validator(certificate)
    for cert_file in sorted((ROOT / "examples").glob("*/certificate*.json")):
        errs = list(cert_validator.iter_errors(json.loads(cert_file.read_text())))
        label = f"{cert_file.parent.name}/{cert_file.name}"
        check(f"{label} vs certificate-schema", not errs, errs[0].message if errs else "")

    eps_validator = Draft202012Validator(epsilon)
    eps_table = yaml.safe_load((SCHEMAS / "epsilon-tolerances.yaml").read_text())
    errs = list(eps_validator.iter_errors(eps_table))
    check("epsilon-tolerances.yaml vs epsilon-tolerance", not errs, errs[0].message if errs else "")

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

    # C5 — the tactile-manifold adapter's feature core is identical to skill-isa's,
    # so an adapter cannot declare it produces a feature outside the shared core.
    ad_core = set(adapter["$defs"]["TactileFeature"]["oneOf"][0]["enum"])
    ad_drift = sk_core ^ ad_core
    check("C5 tactile closed-core identical (tactile-manifold adapter)", not ad_drift,
          f"symmetric difference {sorted(ad_drift)}")

    # C6 — every reference embodiment's declared tactile features are a subset of
    # the features its bound adapter produces (the descriptor-binding rule,
    # 04 § The adapter mapping, TM30c). Instance-level: validates the binding on
    # the worked example against the canonical adapters.
    produced_by_class = {}
    for ad in sorted((SCHEMAS / "tactile-manifold").glob("*.yaml")):
        a = yaml.safe_load(ad.read_text())
        produced_by_class[a["sensor_class"]] = {fp["feature"] for fp in a["feature_production"]}
    for emb in sorted((EXAMPLES / "embodiments").glob("*.yaml")):
        tactile = (yaml.safe_load(emb.read_text()).get("embodiment", {}) or {}).get("tactile", {}) or {}
        for frame, entry in tactile.items():
            sc = entry.get("sensor_class")
            declared = set(entry.get("features", []))
            produced = produced_by_class.get(sc)
            ok = produced is not None and declared <= produced
            detail = (f"unknown adapter {sc!r}" if produced is None
                      else f"{sorted(declared - produced)} not produced by {sc}")
            check(f"C6 {emb.name}:{frame} features subset of adapter {sc}", ok, detail)

    # C7 — the TactileFeature extension-feature pattern is identical across every
    # schema that carries it, so the ext seam cannot drift while the closed core
    # (C3/C4/C5) stays pinned. Complements C2 (which pins the skills ext pattern).
    sk_ext = skill["$defs"]["TactileFeature"]["oneOf"][1]["pattern"]
    ext_locs = {
        "descriptor": next(b["pattern"] for b in
            descriptor["$defs"]["TactileEntry"]["properties"]["features"]["items"]["oneOf"] if "pattern" in b),
        "driver-interface": driver["$defs"]["TactileFeature"]["oneOf"][1]["pattern"],
        "tactile-manifold": adapter["$defs"]["TactileFeature"]["oneOf"][1]["pattern"],
    }
    ext_drift = sorted(k for k, v in ext_locs.items() if v != sk_ext)
    check("C7 TactileFeature ext-pattern identical across schemas", not ext_drift,
          f"differ from skill-isa: {ext_drift}")

    # C8 — every extension-registry entry is structurally consistent. Each entry
    # (every real one under extensions/**/*.json — none at v0.1 by design — plus
    # the schema's bundled worked example, so the check is non-vacuous while the
    # live registry is empty) validates against the schema, has an `identifier`
    # consistent with its namespace/name/version, and a `name` that collides with
    # no reserved core token. Reserved tokens are derived from skill-isa at check
    # time (the C1 anti-drift discipline): the primitive leaf names, the category
    # gates, and the closed-core tactile features (spec/06 § Namespace rules,
    # reserved names). Real entries also pin to their extensions/<ns>/v<MAJOR>/ path.
    ext_validator = Draft202012Validator(extension)
    reserved = {p.split(".", 1)[1] for p in prim} | CATEGORY_GATES | sk_core
    registry_files = sorted(EXTENSIONS.glob("*/v*/*.json")) if EXTENSIONS.is_dir() else []
    entries = [(f"{f.parent.parent.name}/{f.parent.name}/{f.name}", json.loads(f.read_text()), f)
               for f in registry_files]
    entries += [(f"schema example[{i}]", ex, None)
                for i, ex in enumerate(extension.get("examples", []))]
    for label, entry, path in entries:
        errs = list(ext_validator.iter_errors(entry))
        check(f"C8 {label} vs extension-registry", not errs, errs[0].message if errs else "")
        if errs:
            continue
        want_id = f"ext.{entry['namespace']}.{entry['name']}.v{entry['version']}"
        check(f"C8 {label} identifier consistent", entry["identifier"] == want_id,
              f"identifier {entry['identifier']!r} != derived {want_id!r}")
        check(f"C8 {label} name not reserved", entry["name"] not in reserved,
              f"name {entry['name']!r} collides with a reserved core token")
        if path is not None:
            check(f"C8 {label} path matches namespace/version",
                  path.parent.parent.name == entry["namespace"]
                  and path.parent.name == f"v{entry['version']}",
                  "directory does not match namespace/v<MAJOR>")

    # C9 — the ε-tolerance table's key set is exactly the contact-dynamics
    # primitive set (every force.* + in_hand.pivot passive drive, spec/02 RD2c),
    # derived from skill-isa at check time. So a new force primitive forces a
    # table entry (completeness) and a stray key is rejected (no orphan rows).
    contact_dynamics = {p for p in prim if p.startswith("force.")} | {"in_hand.pivot"}
    eps_keys = set(eps_table.get("epsilon_tolerances", {}))
    eps_drift = contact_dynamics ^ eps_keys
    check("C9 epsilon-table keys == contact-dynamics primitives", not eps_drift,
          f"symmetric difference {sorted(eps_drift)}")

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
