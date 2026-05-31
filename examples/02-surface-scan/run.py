#!/usr/bin/env python3
"""Entry script for example 02 — surface scan.

Loads the embodiment-agnostic skill and one embodiment descriptor, then asks the
reference CLI to retarget the skill onto that embodiment and print the canonical
action sequence. The same skill.yaml is retargeted onto each of the three
structurally distinct hands in embodiments/ — that invariance is the point of the
example (Principle 1).

The retargeting itself lives in the `rfl-cli` crate (crates/rfl-cli), which is the
reference implementation still in progress. Until it builds, this script explains
what it would run; the skill and embodiment files are complete and reviewable now.
"""

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
DEFAULT_SKILL = HERE / "skill.yaml"
EMBODIMENTS = HERE / "embodiments"


def main() -> int:
    parser = argparse.ArgumentParser(description="Retarget the surface-scan skill onto an embodiment.")
    parser.add_argument(
        "--embodiment",
        default="allegro",
        help="embodiment descriptor stem in embodiments/ (allegro | leap | pneumatic-6f)",
    )
    parser.add_argument("--skill", type=Path, default=DEFAULT_SKILL, help="path to the Skill ISA file")
    args = parser.parse_args()

    embodiment_path = EMBODIMENTS / f"{args.embodiment}.yaml"
    if not embodiment_path.exists():
        available = ", ".join(sorted(p.stem for p in EMBODIMENTS.glob("*.yaml")))
        print(f"unknown embodiment '{args.embodiment}'. available: {available}", file=sys.stderr)
        return 2

    cmd = [
        "cargo", "run", "-q", "-p", "rfl-cli", "--",
        "retarget", str(args.skill), "--embodiment", str(embodiment_path),
    ]

    if shutil.which("cargo") is None:
        print("cargo not found — install the Rust toolchain (see rust-toolchain.toml) to run retarget.", file=sys.stderr)
        return 1

    # rfl-cli is the in-progress reference implementation; until it builds, surface the
    # intended invocation rather than failing opaquely.
    print(f"$ {' '.join(cmd)}")
    completed = subprocess.run(cmd, cwd=REPO_ROOT)
    if completed.returncode != 0:
        print(
            "\nretarget did not run — the rfl-cli reference implementation is not yet built.\n"
            "The skill and embodiment descriptors are complete; see README.md for the\n"
            "expected per-embodiment retargeting and the conformance envelope classes that fire.",
            file=sys.stderr,
        )
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
