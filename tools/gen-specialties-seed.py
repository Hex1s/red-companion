"""Builds the specialty suggestions of skills (ru and en content snapshots).

    python tools/gen-specialties-seed.py

The book gives lists only for two skills: languages by culture region (p. 45, 52 distinct languages in
10 regions; two independent readings of the image agree) and instruments (p. 137: singing, guitar, drums,
violin, piano "and so on"). Science and Local Expert have no list in the book (the area list belongs to
Night City, phase 6), martial arts have their own collection. These are suggestions: the sheet keeps a
free-text specialty next to them (the book allows a language that is not in the list, p. 45).

The printed list has truncated forms ("непальски", "урд", "йоруб", "наваха"); the real names are stored
(docs/GLOSSARY.md).
"""
import json
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED = ROOT / "crates/rules-core/data/seed"

# (code, ru, en) in the order of the regions on p. 45, first appearance.
LANGUAGES = [
    ("chinese", "Китайский", "Chinese"),
    ("cree", "Кри", "Cree"),
    ("creole", "Креольский", "Creole"),
    ("english", "Английский", "English"),
    ("french", "Французский", "French"),
    ("navajo", "Навахо", "Navajo"),
    ("spanish", "Испанский", "Spanish"),
    ("german", "Немецкий", "German"),
    ("guarani", "Гуарани", "Guarani"),
    ("maya", "Майя", "Maya"),
    ("portuguese", "Португальский", "Portuguese"),
    ("quechua", "Кечуа", "Quechua"),
    ("dutch", "Голландский", "Dutch"),
    ("italian", "Итальянский", "Italian"),
    ("norwegian", "Норвежский", "Norwegian"),
    ("finnish", "Финский", "Finnish"),
    ("polish", "Польский", "Polish"),
    ("romanian", "Румынский", "Romanian"),
    ("russian", "Русский", "Russian"),
    ("ukrainian", "Украинский", "Ukrainian"),
    ("arabic", "Арабский", "Arabic"),
    ("berber", "Берберский", "Berber"),
    ("farsi", "Фарси", "Farsi"),
    ("hebrew", "Иврит", "Hebrew"),
    ("turkish", "Турецкий", "Turkish"),
    ("hausa", "Хауса", "Hausa"),
    ("lingala", "Лингала", "Lingala"),
    ("oromo", "Оромо", "Oromo"),
    ("swahili", "Суахили", "Swahili"),
    ("twi", "Тви", "Twi"),
    ("yoruba", "Йоруба", "Yoruba"),
    ("bengali", "Бенгальский", "Bengali"),
    ("dari", "Дари", "Dari"),
    ("hindi", "Хинди", "Hindi"),
    ("nepali", "Непальский", "Nepali"),
    ("sinhala", "Сингальский", "Sinhala"),
    ("tamil", "Тамильский", "Tamil"),
    ("urdu", "Урду", "Urdu"),
    ("burmese", "Бирманский", "Burmese"),
    ("filipino", "Филиппинский", "Filipino"),
    ("indonesian", "Индонезийский", "Indonesian"),
    ("khmer", "Кхмерский", "Khmer"),
    ("malay", "Малайский", "Malay"),
    ("vietnamese", "Вьетнамский", "Vietnamese"),
    ("cantonese", "Кантонский китайский", "Cantonese Chinese"),
    ("japanese", "Японский", "Japanese"),
    ("korean", "Корейский", "Korean"),
    ("mongolian", "Монгольский", "Mongolian"),
    ("hawaiian", "Гавайский", "Hawaiian"),
    ("maori", "Маори", "Maori"),
    ("pama_nyungan", "Пама-Ньюнган", "Pama-Nyungan"),
    ("tahitian", "Таитянский", "Tahitian"),
]
assert len(LANGUAGES) == 52 and len({c for c, _, _ in LANGUAGES}) == 52

INSTRUMENTS = [
    ("singing", "Пение", "Singing"),
    ("guitar", "Гитара", "Guitar"),
    ("drums", "Барабаны", "Drums"),
    ("violin", "Скрипка", "Violin"),
    ("piano", "Фортепиано", "Piano"),
]


# The namespace string below is not the product name: it only has to stay the same so that uuids stay stable.
def uid(code: str) -> str:
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"favoterm:specialties:{code}"))


def main() -> None:
    rows = [("language", 45, *l) for l in LANGUAGES] + [("play_instrument", 137, *i) for i in INSTRUMENTS]
    for lang, col in (("ru", 3), ("en", 4)):
        entries = [
            {"uuid": uid(f"{skill}:{code}"), "code": code, "skill": skill, "name": row[col],
             "pack": "core", "source": {"page": page}}
            for row in rows
            for skill, page, code in [(row[0], row[1], row[2])]
        ]
        path = SEED / lang / "specialties.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"name": "specialties", "version": 1, "entries": entries},
                                   ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{len(LANGUAGES)} languages, {len(INSTRUMENTS)} instruments")


main()
