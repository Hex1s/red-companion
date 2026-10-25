"""Collects everything a session in the server repository needs into one folder.

    python tools/export-server-bundle.py <target-dir>

Run `cargo test --workspace` first: it refreshes `schemas/*.schema.json` and `schemas/hash-vectors.json`.
The target is the (possibly empty) clone of the server repository; nothing is committed. See
docs/server-bootstrap/README.md.
"""
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOOT = ROOT / "docs" / "server-bootstrap"


def copy_tree(src: Path, dst: Path) -> int:
    """Replaces `dst` with a copy of `src`; returns the number of files."""
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst)
    return sum(1 for p in dst.rglob("*") if p.is_file())


def roadmap_excerpt() -> str:
    """The decisions, phases 7 and 8 and the tasks the server work depends on."""
    lines = (ROOT / "docs" / "ROADMAP.md").read_text(encoding="utf-8").split("\n")

    def block(start_prefix: str, end_prefix: str) -> list[str]:
        a = next(i for i, l in enumerate(lines) if l.startswith(start_prefix))
        b = next(i for i, l in enumerate(lines) if i > a and l.startswith(end_prefix))
        return lines[a:b]

    out = ["# Выдержка из роадмапа приложения", "",
           "Полный файл: https://github.com/Hex1s/red-companion/blob/main/docs/ROADMAP.md", ""]
    out += block("## 0. Принятые решения", "## 1. Механики")
    out += block("### Phase 7.", "### Phase 8.")
    out += block("### Phase 8.", "### Phase 3.")
    out += ["### Задачи приложения, от которых зависит сервер", ""]
    wanted = ("- [~] 2.0 ", "- [ ] 2.0b ", "- [ ] 2.12 ", "- [ ] 2.22 ", "- [~] 2.10 ")
    out += [l for l in lines if l.startswith(wanted)]
    return "\n".join(out) + "\n"


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    target = Path(sys.argv[1]).resolve()
    target.mkdir(parents=True, exist_ok=True)

    (target / "docs").mkdir(exist_ok=True)
    for name in ("SERVER-SPEC.md", "GLOSSARY.md", "BOOK-PLAN.md"):
        shutil.copy2(ROOT / "docs" / name, target / "docs" / name)
    (target / "docs" / "ROADMAP-excerpt.md").write_text(roadmap_excerpt(), encoding="utf-8")

    # CLAUDE.md and README.md are the server's own: written once, never overwritten.
    for name in ("CLAUDE.md", "README.md"):
        if not (target / name).exists():
            shutil.copy2(BOOT / name, target / name)

    schemas = copy_tree(ROOT / "schemas", target / "contracts" / "schemas")
    seed = copy_tree(ROOT / "crates" / "rules-core" / "data" / "seed", target / "contracts" / "seed")
    ref = target / "reference" / "rules-core"
    ref.mkdir(parents=True, exist_ok=True)
    for name in ("catalog.rs", "data_source.rs", "error_codes.rs"):
        shutil.copy2(ROOT / "crates" / "rules-core" / "src" / name, ref / name)

    print(f"{target}: docs, {schemas} schema files, {seed} seed files, 3 reference sources")


main()
