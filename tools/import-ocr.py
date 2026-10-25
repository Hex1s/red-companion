"""Splits the OCR result of the book (tools of the pdf-reader project) into one file per printed page.

    python tools/import-ocr.py D:/projects/pdf-reader/output/cyberpunkRedBook.json

Writes `.cache/book-ocr/p<N>.txt` (N = printed page = PDF page - 1, as everywhere in this project)
and `.cache/book-ocr/index.json` with the confidence and the "check by a human" reason of each page.
The folder is git-ignored: the book is copyrighted, only structured data goes into the repository.

The OCR text is good for prose and unreliable for tables and dice notation (see the read-rulebook
skill): use it to find things and to draft descriptions, take numbers from the page images.
"""

import json
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / ".cache" / "book-ocr"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    pages = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    OUT.mkdir(parents=True, exist_ok=True)
    index = []
    for page in pages:
        printed = page["page"] - 1
        (OUT / f"p{printed}.txt").write_text(page["text"], encoding="utf-8")
        index.append(
            {
                "printed": printed,
                "pdf": page["page"],
                "conf": page.get("conf"),
                "review": page.get("review") or "",
                "edited": bool(page.get("edited")),
            }
        )
    (OUT / "index.json").write_text(json.dumps(index, ensure_ascii=False, indent=1), encoding="utf-8")
    flagged = sum(1 for p in index if p["review"])
    print(f"{len(index)} pages -> {OUT} ({flagged} marked for review)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
