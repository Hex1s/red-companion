"""Regenerates docs/GLOSSARY.md from the locales and the seed data, so the glossary cannot drift.

    python tools/gen-glossary.py

Names come from `src/i18n/locales/{ru,en}.json` and `crates/rules-core/data/seed/*.json`; the only
hand-written part is SYNONYMS below (variants the Russian book itself uses for one term).
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCALES = {lang: json.loads((ROOT / f"src/i18n/locales/{lang}.json").read_text(encoding="utf-8")) for lang in ("ru", "en")}
INJURIES = json.loads((ROOT / "crates/rules-core/data/seed/critical_injuries.json").read_text(encoding="utf-8"))["entries"]

# ID -> how the book writes the same thing elsewhere (page in brackets, printed numbering).
SYNONYMS = {
    "sp": "очки стойкости, стойкость брони",
    "injury.broken_leg": "«Сломанная нога» (текст про прицельный выстрел, стр. 170)",
    "injury.broken_arm": "«Сломаной рукой» — опечатка книги (стр. 221)",
    "injury.broken_neck": "смысл ru и en расходится: ru «Перелом шеи», en показывает Whiplash (хлыстовая травма) — сверить с английским изданием",
}


def g(lang: str, *path: str) -> str:
    node = LOCALES[lang]
    for part in path:
        node = node[part]
    return node


def row(*cells: str) -> str:
    return "| " + " | ".join(cells) + " |"


def table(head: tuple[str, ...], rows: list[tuple[str, ...]]) -> list[str]:
    return [row(*head), row(*["---"] * len(head)), *[row(*r) for r in rows], ""]


out = [
    "# Глоссарий",
    "",
    "> Файл собирается скриптом `python tools/gen-glossary.py` из локалей и seed-данных; руками правятся только синонимы (словарь `SYNONYMS` в скрипте).",
    "",
    "Термины правил имеют **устойчивые ID** (английские, snake_case). В коде, данных и ключах локализации используются только ID; переводы живут в `src/i18n/locales/{ru,en}.json`.",
    "",
    "Русская книга называет одно и то же по-разному (например, «Сломанная нога» в тексте и «Перелом ноги» в таблице; СЛ и DV вперемешку). Правило: один ID, одно отображаемое название в локали, остальные варианты — в колонке «Синонимы книги».",
    "",
    "**Английские названия** подобраны без английского издания книги: колонка «en» — это то, что показывает приложение, а не сверенный официальный текст. Когда издание будет под рукой, сверить и поправить в `en.json`.",
    "",
    "## Общие термины",
    "",
]

general = [
    ("hp", "glossary.hp"), ("sp", "glossary.sp"), ("dv", "glossary.dv"), ("humanity", "glossary.humanity"),
]
rows = []
for gid, _ in general:
    rows.append((f"`{gid}`", f"{g('ru','glossary',gid)} — {g('ru','glossary',gid+'_full')}",
                 f"{g('en','glossary',gid)} — {g('en','glossary',gid+'_full')}", SYNONYMS.get(gid, "")))
for key in ("death_save", "critical_injury", "cover", "ablation"):
    rows.append((f"`{key}`", g("ru", "glossary", key), g("en", "glossary", key), SYNONYMS.get(key, "")))
out += table(("ID", "ru", "en", "Синонимы книги"), rows)

out += ["## Ранение, броня, область", ""]
rows = []
for key in ("healthy", "light", "serious", "mortal", "dead"):
    rows.append((f"`wound.{key}`", g("ru", "glossary", "wound", key), g("en", "glossary", "wound", key), ""))
for key in ("full", "half", "ignored"):
    rows.append((f"`armor_mode.{key}`", g("ru", "glossary", "armor_mode", key), g("en", "glossary", "armor_mode", key), ""))
for key in ("head", "body"):
    rows.append((f"`area.{key}`", g("ru", "glossary", "area", key), g("en", "glossary", "area", key), ""))
out += table(("ID", "ru", "en", "Синонимы книги"), rows)

out += ["## Характеристики", "", "Аббревиатуры русской редакции; ID — английские аббревиатуры оригинала.", ""]
rows = [(f"`{k}`", g("ru", "stats", k), k, "") for k in g("ru", "stats") if isinstance(g("ru", "stats", k), str) and k.isupper()]
out += table(("ID", "ru", "en", "Синонимы книги"), rows)

out += [
    "## Критические повреждения",
    "",
    "ID — как в `critical_injuries.json`; у «Инородного тела» два ID по областям. «Бросок» — 2d6.",
    "",
]
rows = []
for e in INJURIES:
    i = e["id"]
    rows.append((
        f"`{i}`",
        "тело" if e["area"] == "body" else "голова",
        str(e["roll"]),
        g("ru", "injury", i, "name"),
        g("en", "injury", i, "name"),
        SYNONYMS.get(f"injury.{i}", ""),
        str(e["source"]["page"]),
    ))
out += table(("ID", "Область", "Бросок", "ru", "en", "Синонимы книги", "Стр."), rows)

out += [
    "## Расхождения книги в снаряжении",
    "",
    "Книга пишет одно и то же по-разному; в данных одно название, остальное здесь.",
    "",
    "| ID | Название в данных | Как пишет книга |",
    "| --- | --- | --- |",
    "| `tracker` | Трекер (маячок) | таблица цен (стр. 351): «Трекер (Маячек)», описание (стр. 354): «Трекер (Маячок)» |",
    "| `exoskeleton_beta`, `exoskeleton_sigma` | Внешний экзоскелет ß (бета) / ∑ (сигма) | список (стр. 351): «Внешний экзоскелет», описание (стр. 354): «Имплантированный экзоскелет»; в описании сигмы сказано «предыдущая вариация Сигма» вместо «бета» |",
    "| `bodyweight_suit` | Комбинезон «Бодивейт» | таблица брони: «Комбинезон Бодивейт» / «Бодивейт»; у каждого комбинезона ОС 11 на голове и теле, по частям не продаётся (стр. 350) |",
    "| `navajo`, `yoruba`, `urdu`, `nepali` | Навахо, Йоруба, Урду, Непальский | список языков (стр. 45) печатает обрезанные «наваха», «йоруб», «урд», «непальски»; в данных настоящие названия |",
    "| `shoulder_arms` | Тактическое оружие | в русской книге навык так назван; модификации «подходят к оружию с навыком Тактическое оружие» |",
    "",
]

(ROOT / "docs/GLOSSARY.md").write_text("\n".join(out), encoding="utf-8")
print(f"docs/GLOSSARY.md: {len(INJURIES)} injuries")
