"""OCR pages of the rulebook PDF (Russian + English) with Tesseract.

The PDF text layer is unusable, so text is recovered from rendered pages.
Output: .cache/ocr/p<N>.txt (N is the PDF page; printed page = N - 1).
OCR output of tables and sidebars needs manual verification against the page image.

Usage:
    python tools/ocr-pages.py <first-pdf-page> [<last-pdf-page>] [--dpi 250] [--psm 3]

Requires: pip install pymupdf; Tesseract (winget install UB-Mannheim.TesseractOCR).
Language data is downloaded to .cache/tessdata on first run.
"""
import argparse
import pathlib
import shutil
import subprocess
import sys
import urllib.request

try:
    import pymupdf
except ImportError:
    sys.exit("PyMuPDF is missing: pip install pymupdf")

ROOT = pathlib.Path(__file__).resolve().parent.parent
TESSDATA = ROOT / ".cache" / "tessdata"
BEST = "https://github.com/tesseract-ocr/tessdata_best/raw/main/{}.traineddata"

parser = argparse.ArgumentParser()
parser.add_argument("first", type=int)
parser.add_argument("last", type=int, nargs="?")
parser.add_argument("--dpi", type=int, default=250)
parser.add_argument("--psm", type=int, default=3, help="3 = auto layout; 6 = single block; 4 = single column")
parser.add_argument("--pdf", type=pathlib.Path, default=ROOT / "docs" / "cyberpunkRedBook.pdf")
args = parser.parse_args()

exe = shutil.which("tesseract") or r"C:\Program Files\Tesseract-OCR\tesseract.exe"
if not pathlib.Path(exe).exists():
    sys.exit("Tesseract not found: winget install UB-Mannheim.TesseractOCR")

TESSDATA.mkdir(parents=True, exist_ok=True)
for lang in ("rus", "eng"):
    target = TESSDATA / f"{lang}.traineddata"
    if not target.exists():
        print(f"downloading {lang}.traineddata ...")
        urllib.request.urlretrieve(BEST.format(lang), target)

out = ROOT / ".cache" / "ocr"
img = ROOT / ".cache" / "ocr-img"
out.mkdir(parents=True, exist_ok=True)
img.mkdir(parents=True, exist_ok=True)

doc = pymupdf.open(args.pdf)
last = args.last or args.first
if not 1 <= args.first <= last <= len(doc):
    sys.exit(f"pages must be within 1..{len(doc)}")

for page in range(args.first, last + 1):
    png = img / f"p{page}.png"
    doc[page - 1].get_pixmap(dpi=args.dpi).save(png)
    base = out / f"p{page}"
    subprocess.run(
        [exe, str(png), str(base), "-l", "rus+eng", "--tessdata-dir", str(TESSDATA), "--psm", str(args.psm)],
        check=True, capture_output=True,
    )
    png.unlink()
    print(f"{base}.txt")
