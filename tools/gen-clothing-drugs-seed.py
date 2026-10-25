"""Builds the clothing styles and street drugs seed data (ru and en content snapshots).

    python tools/gen-clothing-drugs-seed.py

Clothing: the style x item price matrix is printed twice, p. 102 and p. 356 (the copies agree), the
price categories are printed next to every price; the script fails when the category derived from the
price differs from the printed one. Street drugs: table p. 357 and descriptions p. 357-358. Texts are
short summaries in our own words (RTG licence); English names are chosen without the English edition.
"""
import json
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED = ROOT / "crates/rules-core/data/seed"

TIERS = [(10, "Дешевка"), (20, "Обычное"), (50, "Ценное"), (100, "Премиум"), (500, "Дорогое"),
         (1000, "Очень дорогое"), (5000, "Роскошь"), (10000, "Супер Роскошь")]


def category_of(price: int) -> str:
    return [name for minimum, name in TIERS if price >= minimum][-1]


# The namespace string below is not the product name: it only has to stay the same so that uuids stay stable.
def uid(collection: str, code: str) -> str:
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"favoterm:{collection}:{code}"))


# Item order of the columns (the locale keys `clothing.item.<id>` name them).
ITEMS = ["pants", "top", "jacket", "shoes", "jewelry", "mirrorshades", "glasses", "contacts", "hats"]

# code, printed name, ru tags, en tags, prices in ITEMS order
CLOTHING = [
    ("bag_lady_chic", "Bag Lady Chic", "Бездомный, оборванный, бродяга, анти-фешн", "Homeless, ragged, drifter, anti-fashion",
     [20, 10, 20, 20, 20, 20, 10, 10, 10]),
    ("gang_colors", "Gang Colors", "Опасный, жестокий, бунтарь", "Dangerous, brutal, rebellious",
     [50, 20, 50, 20, 50, 20, 20, 10, 10]),
    ("generic_chic", "Generic Chic", "Стандартный, цветной, модульный", "Standard, colourful, modular",
     [50, 20, 50, 20, 50, 20, 20, 10, 10]),
    ("bohemian", "Bohemian", "Народное, ретро, фристайл", "Folk, retro, freestyle",
     [50, 20, 50, 50, 100, 50, 50, 10, 10]),
    ("leisurewear", "Leisurewear", "Комфорт, ловкость, атлетизм", "Comfort, agility, athleticism",
     [100, 20, 100, 50, 100, 50, 50, 20, 50]),
    ("nomad_leathers", "Nomad Leathers", "Вестерн, грубый, племенной", "Western, rugged, tribal",
     [100, 20, 100, 100, 100, 50, 50, 20, 100]),
    ("asia_pop", "Asia Pop", "Яркий, костюмированный, молодёжный", "Bright, costume-like, youthful",
     [100, 20, 100, 100, 100, 100, 100, 100, 100]),
    ("urban_flash", "Urban Flash", "Яркий, технологичный, уличный", "Bright, techy, street",
     [100, 20, 100, 100, 100, 100, 100, 100, 100]),
    ("businesswear", "Businesswear", "Лидерство, осанка, авторитет", "Leadership, poise, authority",
     [500, 50, 500, 500, 5000, 500, 500, 100, 500]),
    ("high_fashion", "High Fashion", "Эксклюзив, дизайнерская, кутюр", "Exclusive, designer, couture",
     [1000, 500, 1000, 5000, 5000, 1000, 1000, 1000, 5000]),
]
# Category printed next to each price (p. 356), in the same order as the prices above.
PRINTED = [
    "Обычное Дешевка Обычное Обычное Обычное Обычное Дешевка Дешевка Дешевка",
    "Ценное Обычное Ценное Обычное Ценное Обычное Обычное Дешевка Дешевка",
    "Ценное Обычное Ценное Обычное Ценное Обычное Обычное Дешевка Дешевка",
    "Ценное Обычное Ценное Ценное Премиум Ценное Ценное Дешевка Дешевка",
    "Премиум Обычное Премиум Ценное Премиум Ценное Ценное Обычное Ценное",
    "Премиум Обычное Премиум Премиум Премиум Ценное Ценное Обычное Премиум",
    "Премиум Обычное Премиум Премиум Премиум Премиум Премиум Премиум Премиум",
    "Премиум Обычное Премиум Премиум Премиум Премиум Премиум Премиум Премиум",
    "Дорогое Ценное Дорогое Дорогое Роскошь Дорогое Дорогое Премиум Дорогое",
    "Очень_дорогое Дорогое Очень_дорогое Роскошь Роскошь Очень_дорогое Очень_дорогое Очень_дорогое Роскошь",
]

# code, price, printed category, hours, secondary DV, ru name, ru primary, ru secondary, en name, en primary, en secondary
DRUGS = [
    ("black_lace", 50, "Ценное", 24, 17,
     "Черное Кружево",
     "При приёме теряется 2d6 ЧЕЛ (возвращаются, если не сработал вторичный эффект). На время действия игнорируются эффекты тяжёлого ранения.",
     "Потеря ЧЕЛ от первичного эффекта не возвращается. Пользователь становится зависимым; пока он не под действием, его РЕА снижается на 2.",
     "Black Lace",
     "Taking a dose costs 2d6 HUM (returned if the secondary effect does not take hold). The effects of serious wounds are ignored while it lasts.",
     "The HUM lost to the primary effect is not returned. The user becomes addicted; while not under the primary effect their REF drops by 2."),
    ("blue_glass", 20, "Обычное", 4, 15,
     "Синее Стекло",
     "Время от времени мастер сообщает, что вы «вспыхиваете»: галлюцинации в виде вихрей ярких цветов, и в этот момент действие в ходе сделать нельзя.",
     "Пользователь зависим и «вспыхивает» примерно раз в час (у разных людей по-разному). Пока зависимость не излечена, основной эффект меняется: при приёме вспышки нет и состояние стабильное, но воспринимается не как естественное; вспышки в ходе действия остаются.",
     "Blue Glass",
     "From time to time the GM tells you that you \"flash\": hallucinations of swirling bright colours, and you cannot take an action that turn.",
     "The user is addicted and flashes about once an hour (it varies from person to person). While the addiction lasts the primary effect changes: no flash on taking a dose and a stable state, which still does not feel natural; flashes during the effect remain."),
    ("boost", 50, "Ценное", 24, 17,
     "Буст",
     "На время действия ИНТ повышается на 2, даже выше 8.",
     "Пользователь зависим; пока зависимость не излечена, его ИНТ снижается на 2.",
     "Boost",
     "While it lasts INT rises by 2, even above 8.",
     "The user is addicted; until the addiction is cured their INT drops by 2."),
    ("smash", 10, "Дешевка", 4, 15,
     "Смэш",
     "Эйфория, расслабление и готовность к вечеринке: +2 к Танцу, Акробатике, Общению, Проницательности, Убеждению и Актёрскому мастерству.",
     "Пользователь зависим; пока зависимость не излечена, те же навыки снижаются на 2. Мастер время от времени напоминает о тяге, её нужно отыгрывать.",
     "Smash",
     "Euphoria, relaxation and readiness to party: +2 to Dance, Acrobatics, Conversation, Human Perception, Persuasion and Acting.",
     "The user is addicted; until it is cured the same skills drop by 2. The GM now and then reminds them of the craving, and it should be roleplayed."),
    ("syntococaine", 20, "Обычное", 4, 15,
     "Синтококс",
     "На время действия РЕА повышается на 1, даже выше 8; пользователь склонен к паранойе, мастер подсказывает, когда её отыгрывать.",
     "Пользователь зависим; пока зависимость не излечена, его РЕА снижается на 2. Мастер время от времени напоминает о тяге, её нужно отыгрывать.",
     "Syntococaine",
     "While it lasts REF rises by 1, even above 8; the user is prone to paranoia and the GM says when to roleplay it.",
     "The user is addicted; until it is cured their REF drops by 2. The GM now and then reminds them of the craving, and it should be roleplayed."),
]


def write(path: Path, name: str, entries: list) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"name": name, "version": 1, "entries": entries}, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8")


def main() -> None:
    # Category check of the whole matrix.
    for (code, _, _, _, prices), printed in zip(CLOTHING, PRINTED):
        cats = [c.replace("_", " ") for c in printed.split()]
        assert len(prices) == len(ITEMS) == len(cats), code
        for item, price, cat in zip(ITEMS, prices, cats):
            assert category_of(price) == cat, f"{code}/{item}: {price} is {category_of(price)}, printed {cat}"
    for code, price, printed, *_ in DRUGS:
        assert category_of(price) == printed, code

    for lang in ("ru", "en"):
        clothing = [
            {"uuid": uid("clothing", code), "code": code, "name": name,
             "tags": ru if lang == "ru" else en, "prices": prices, "pack": "core", "source": {"page": 356}}
            for code, name, ru, en, prices in CLOTHING
        ]
        drugs = []
        for code, price, _, hours, dv, *texts in DRUGS:
            name, primary, secondary = texts[:3] if lang == "ru" else texts[3:]
            drugs.append({"uuid": uid("street_drugs", code), "code": code, "name": name, "price": price,
                          "duration_hours": hours, "secondary_dv": dv, "primary": primary, "secondary": secondary,
                          "pack": "core", "source": {"page": 357}})
        write(SEED / lang / "clothing.json", "clothing", clothing)
        write(SEED / lang / "street_drugs.json", "street_drugs", drugs)
    print(f"{len(CLOTHING)} styles x {len(ITEMS)} items, {len(DRUGS)} drugs")


main()
