#!/usr/bin/env python3
"""Fail if any .rs file under rust/src/ is unreachable from the module tree.

An orphaned file compiles to nothing and is invisible to every other check:
`cargo build`, `clippy`, `cargo test` and code coverage all skip it, because as
far as rustc is concerned it does not exist. Reviewers see a diff full of real
code and assume it ships.

SmooAI/client-shared shipped exactly that: 174 lines of auth token-refresh code
in `rust/src/auth/refresh.rs` that `auth/mod.rs` never declared, so it never
compiled once (fixed upstream in a21c06b). This is the check that would have
caught it the same day.

Deliberately textual rather than a real parse: a `mod foo;` behind a `#[cfg(...)]`
still counts as declared, which is what we want — the question is whether the
file is wired in at all, not whether it is wired in for every feature set.
"""

import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "rust" / "src"


def declares(parent: Path, name: str) -> bool:
    """Whether `parent` contains a `mod <name>;` / `mod <name> { … }` declaration."""
    if not parent.is_file():
        return False
    return re.search(rf"\bmod\s+{re.escape(name)}\s*[;{{]", parent.read_text()) is not None


ROOTS = [SRC / "lib.rs", SRC / "main.rs"]


def module_of(rs: Path) -> tuple[str, list[Path]]:
    """The module name `rs` defines, and the file(s) that could declare it."""
    if rs.name == "mod.rs":
        # `src/a/mod.rs` defines module `a`, declared one level up.
        name, parent_dir = rs.parent.name, rs.parent.parent
    else:
        # `src/a/b.rs` defines module `b`, declared in `src/a`.
        name, parent_dir = rs.stem, rs.parent

    if parent_dir == SRC:
        return name, ROOTS
    # Either module style: `src/a/mod.rs` or the 2018-edition `src/a.rs`.
    return name, [parent_dir / "mod.rs", parent_dir.with_suffix(".rs")]


def main() -> int:
    if not SRC.is_dir():
        print(f"no {SRC} — nothing to check")
        return 0

    repo = SRC.parent.parent
    orphans = []

    for rs in sorted(SRC.rglob("*.rs")):
        if rs in ROOTS:
            continue
        name, candidates = module_of(rs)
        if not any(declares(c, name) for c in candidates):
            where = " or ".join(str(c.relative_to(repo)) for c in candidates)
            orphans.append(f"  {rs.relative_to(repo)} — no `mod {name};` in {where}")

    if orphans:
        print("Orphaned module files (present on disk, never compiled):", file=sys.stderr)
        print("\n".join(orphans), file=sys.stderr)
        print(
            "\nDeclare each with `mod <name>;` / `pub mod <name>;` in its parent, "
            "or delete the file.",
            file=sys.stderr,
        )
        return 1

    print(f"✓ every .rs file under {SRC.relative_to(repo)} is in the module tree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
