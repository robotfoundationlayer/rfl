#!/usr/bin/env python3
"""Markdown doc linter for the RFL repo (pure stdlib, no dependencies).

Lints outward-facing Markdown for three classes of defect that otherwise land
on `main` green:

  1. De-AI: the JP double em-dash `——` in prose (the top AI-generated tell). A
     single `—` is legitimate English punctuation and is *not* flagged. Inline
     code and fenced code blocks are stripped first, so a doc may quote `——`.
  2. Divider convention: a bare `---` thematic break (exactly three hyphens);
     the repo uses `----`. (`---` also breaks Ulysses HTML/PDF export.)
  3. Internal-link integrity: every relative `[text](target)` resolves to an
     existing file, and any `#anchor` matches a heading slug in the target.

Excludes internal/other-owned trees: docs/design, docs/plans, whitepaper.

    python3 scripts/md_lint.py

Exits non-zero (and prints every violation) on any failure.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXCLUDE_DIRS = ("docs/design", "docs/plans", "whitepaper", "node_modules", "target")

# A markdown inline link [text](target) — target up to the first space or ).
LINK_RE = re.compile(r"\[(?:[^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
INLINE_CODE_RE = re.compile(r"`[^`]*`")
ATX_HEADING_RE = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
EXTERNAL_SCHEME_RE = re.compile(r"^[a-z][a-z0-9+.-]*:", re.IGNORECASE)

failures: list[str] = []


def fail(msg: str) -> None:
    failures.append(msg)


def md_files() -> list[Path]:
    out = []
    for p in sorted(ROOT.rglob("*.md")):
        rel = p.relative_to(ROOT).as_posix()
        if any(rel == d or rel.startswith(d + "/") for d in EXCLUDE_DIRS):
            continue
        out.append(p)
    return out


def slugify(heading: str) -> str:
    """GitHub-style heading slug: strip inline code/links, lowercase, drop
    punctuation except hyphens, spaces to hyphens."""
    text = INLINE_CODE_RE.sub(lambda m: m.group(0).strip("`"), heading)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)  # [t](u) -> t
    text = text.strip().lower()
    text = re.sub(r"[^\w\s-]", "", text)  # drop punctuation (keeps word chars, spaces, hyphens)
    text = text.replace(" ", "-")
    return text


def headings_of(path: Path) -> set[str]:
    slugs: dict[str, int] = {}
    out: set[str] = set()
    in_fence = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = ATX_HEADING_RE.match(line)
        if not m:
            continue
        base = slugify(m.group(2))
        n = slugs.get(base, 0)
        slug = base if n == 0 else f"{base}-{n}"
        slugs[base] = n + 1
        out.add(slug)
    return out


def strip_code(text: str) -> str:
    """Remove fenced blocks and inline code so prose checks ignore literals."""
    out_lines = []
    in_fence = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            out_lines.append("")
            continue
        out_lines.append("" if in_fence else INLINE_CODE_RE.sub("", line))
    return "\n".join(out_lines)


def check_prose_and_dividers(path: Path, rel: str) -> None:
    raw = path.read_text(encoding="utf-8")
    stripped = strip_code(raw)
    for i, line in enumerate(stripped.splitlines(), 1):
        if "——" in line:
            fail(f"{rel}:{i}: JP double em-dash `——` in prose (de-AI); use 。/（）/：/、")
    for i, line in enumerate(raw.splitlines(), 1):
        if line.rstrip() == "---":
            fail(f"{rel}:{i}: bare `---` divider; use `----` (four hyphens)")


def check_links(path: Path, rel: str, heading_cache: dict[Path, set[str]]) -> None:
    raw = path.read_text(encoding="utf-8")
    # Drop fenced code blocks so example links inside ``` are not link-checked.
    lines, in_fence, body = raw.splitlines(), False, []
    for line in lines:
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            body.append("")
            continue
        body.append("" if in_fence else line)
    text = INLINE_CODE_RE.sub("", "\n".join(body))

    for target in LINK_RE.findall(text):
        if target.startswith("#"):
            anchor = target[1:]
            if anchor and anchor not in heading_cache.setdefault(path, headings_of(path)):
                fail(f"{rel}: link to `#{anchor}` has no matching heading")
            continue
        if EXTERNAL_SCHEME_RE.match(target):
            continue  # http(s), mailto, tel, …
        file_part, _, anchor = target.partition("#")
        if not file_part:
            continue
        dest = (path.parent / file_part).resolve()
        if not dest.exists():
            fail(f"{rel}: broken link `{target}` (no file at {file_part})")
            continue
        if anchor and dest.suffix == ".md":
            slugs = heading_cache.setdefault(dest, headings_of(dest))
            if anchor not in slugs:
                fail(f"{rel}: link `{target}` anchor `#{anchor}` not a heading in {file_part}")


def main() -> int:
    heading_cache: dict[Path, set[str]] = {}
    files = md_files()
    for path in files:
        rel = path.relative_to(ROOT).as_posix()
        check_prose_and_dividers(path, rel)
        check_links(path, rel, heading_cache)

    print(f"md_lint: scanned {len(files)} markdown file(s)")
    if failures:
        print(f"FAILED — {len(failures)} violation(s):")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("PASS — markdown lint clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
