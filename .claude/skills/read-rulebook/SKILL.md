---
name: read-rulebook
description: Use when you need rules, tables or data from docs/cyberpunkRedBook.pdf (Cyberpunk RED, Russian edition) — damage, armor, critical injuries, cover, vehicles, netrunning, Night City, prices, NPCs. Explains how to read the PDF, whose text layer is unusable, and where each chapter lives.
---

# Reading the Cyberpunk RED rulebook

The PDF (`docs/cyberpunkRedBook.pdf`, 458 pages, Russian edition) has a **broken text layer** (custom font encoding): text extraction returns garbage. Do not use pypdf/pdftotext, do not try to decode it. Read pages as images.

## How

1. Render pages: `python tools/render-pages.py <first> <last>` (PyMuPDF; `pip install pymupdf` if missing). Output goes to `.cache/book-pages/p<N>.png` (git-ignored). Default 85 dpi is enough for body text and tables; use `--dpi 120` for dense tables.
2. Open the PNGs with the Read tool. Read at most ~6 pages per batch, in parallel.
3. **Printed page N = PDF page N + 1.** Page references in `docs/ROADMAP.md` are printed numbers.

## OCR (Tesseract) for bulk prose

`python tools/ocr-pages.py <first> [<last>]` writes `.cache/ocr/p<N>.txt` (rus+eng, ~5 s per page). Measured quality:

- **Prose: good.** Use it to skim long text (Night City descriptions, rules explanations) without spending context on images.
- **Tables and sidebars: unreliable.** Dice notation is misread (`1d6` → `196`, `2d6` → `246`, `3d6` → `Зd6`), white-on-red table headers and the "ПЗ" glyph are mangled, table rows interleave, sidebars mix into the body. **Never take numbers from OCR of a table**: read the page image instead (render with `tools/render-pages.py`).
- OCR text of a page that contains a table is a hint for where to look, not a source.

## Where things are (printed pages)

| Chapter | Pages |
|---|---|
| Contents | 3 (PDF 4) |
| Roles | 27–42 |
| Stats, derived stats (HP formula table p.79, Humanity p.80), skills, gear | 71–106 |
| Cyberware and humanity loss | 107–120 |
| Skills list and role abilities | 125–166 |
| **Combat** (initiative 168, ranged tables 171–173, melee 175–176, martial arts 178–180, cover 182–183, armor 184–185, damage steps and wound states 186, critical injuries 187–188) | 167–194 |
| Vehicles (tables 189–191, maneuvers and ramming 192) | 189–192 |
| Reputation, intimidation | 193–194 |
| **Netrunning** (net actions 197–198, interface functions 198–200, net combat 201, programs 202–204, Black ICE 205–207, architecture generator 209–211) | 195–218 |
| Trauma Team (critical injury tables 221, stabilization 222, drugs 227) | 219–232 |
| Lore: Dark Future, Red Time | 233–282 |
| Night City (zones and threat ratings 295–297, map 296, key people 304, gangs 308, POI map 310, POI list 310–314) | 283–314 |
| Everyday life | 315–336 |
| Night markets, weapon quality 342, cyberware 364, price categories 385 | 337–386 |
| GM tools (rhythm chart 395, filler NPC stat blocks 412+, scream sheets 425) | 387–434 |
| "Black Dog" adventure | 435–458 |

## Rules for extracting data

- Quote numbers exactly; tables are the source of truth, prose sometimes contradicts them (the translation is inconsistent, e.g. "Сломанная нога" in text vs "Перелом ноги" in the injury table). Note such conflicts and ask the user (AskUserQuestion) rather than picking silently.
- Stat abbreviations of this edition: РЕА=REF, ЛВК=DEX, ТЕЛ=BODY, ХАР=COOL, ЭМП=EMP, СКО=MOVE, УДЧ=LUCK, ПЗ=HP, ОС=SP, ЧЕЛ=HUM, СЛ=DV, СКА=ROF, ПЗТ=vehicle HP.
- Data that goes into the app (seed/reference data) must carry stable IDs, and every text goes to `ru`/`en` locale files; see CLAUDE.md.
- The book is copyrighted (R. Talsorian Games). Do not paste long passages of it into the repo; store structured data and short names.
