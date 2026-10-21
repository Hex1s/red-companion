"""Render pages of the rulebook PDF to PNG so they can be read as images.

The PDF text layer is unusable (custom font encoding), so pages must be read
visually or OCR'd. Printed page N is PDF page N + 1.

Usage:
    python tools/render-pages.py <first-pdf-page> [<last-pdf-page>] [--dpi 85] [--out DIR] [--pdf FILE]

Requires: pip install pymupdf
"""
import argparse
import pathlib
import sys

try:
    import pymupdf
except ImportError:
    sys.exit("PyMuPDF is missing: pip install pymupdf")

ROOT = pathlib.Path(__file__).resolve().parent.parent

parser = argparse.ArgumentParser()
parser.add_argument("first", type=int, help="first PDF page (1-based)")
parser.add_argument("last", type=int, nargs="?", help="last PDF page (defaults to first)")
parser.add_argument("--dpi", type=int, default=85, help="85 is enough to read body text and tables")
parser.add_argument("--out", type=pathlib.Path, default=ROOT / ".cache" / "book-pages")
parser.add_argument("--pdf", type=pathlib.Path, default=ROOT / "docs" / "cyberpunkRedBook.pdf")
args = parser.parse_args()

last = args.last or args.first
args.out.mkdir(parents=True, exist_ok=True)
doc = pymupdf.open(args.pdf)
if not 1 <= args.first <= last <= len(doc):
    sys.exit(f"pages must be within 1..{len(doc)}")

for page in range(args.first, last + 1):
    target = args.out / f"p{page}.png"
    doc[page - 1].get_pixmap(dpi=args.dpi).save(target)
    print(target)
