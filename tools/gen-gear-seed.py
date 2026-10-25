"""Builds the equipment seed data (ru and en content snapshots plus the two core tables).

    python tools/gen-gear-seed.py

Numbers come from the book pages (two independent readings of the images agreed): the gear list
p. 351 with descriptions p. 352-355, modifications p. 343-344, the magazine table p. 344, ammunition
p. 345-347, price categories p. 385, armor and shield prices p. 97 / p. 350. Texts are short summaries
in our own words (RTG licence, docs/ROADMAP.md section 7), not the book's wording; English names are
chosen without the English edition (see docs/GLOSSARY.md).

Every price line also records the category the book prints next to it; the script fails when the
category derived from the price differs, which catches a misread digit.
"""
import json
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED = ROOT / "crates/rules-core/data/seed"

# Price categories, p. 385: id, minimum price, name as printed (the check below compares ids).
CATEGORIES = [
    ("cheap", 10, "Дешевка"),
    ("everyday", 20, "Обычное"),
    ("costly", 50, "Ценное"),
    ("premium", 100, "Премиум"),
    ("expensive", 500, "Дорогое"),
    ("very_expensive", 1000, "Очень дорогое"),
    ("luxury", 5000, "Роскошь"),
    ("super_luxury", 10000, "Супер Роскошь"),
]
PRINTED = {"О. Дорогое": "Очень дорогое"}


def category_of(price: int) -> str:
    return [name for _, minimum, name in CATEGORIES if price >= minimum][-1]


def check_category(code: str, price: int, printed: str) -> None:
    printed = PRINTED.get(printed, printed)
    assert category_of(price) == printed, f"{code}: {price} eb is {category_of(price)}, the book prints {printed}"


# ---- gear list. (code, price, printed category, description page,
#                  ru name, ru summary, en name, en summary)
GEAR = [
    ("agent", 100, "Премиум", 352,
     "Агент", "Смартфон с обучающимся ИИ: звонки, поиск в Дата Пуле, расписание, заказ товаров, записи. Даёт +2 к «Поиску информации» и «Гардеробу и стилю», если носить одежду, которую он подобрал; несколько Агентов не складываются.",
     "Agent", "Smartphone with a learning assistant AI: calls, Data Pool search, schedule, shopping, recording. Gives +2 to Library Search and Wardrobe & Style while you wear what it suggests; several Agents do not stack."),
    ("airhypo", 50, "Ценное", 352,
     "Аэрогиппо", "Вводит препарат под кожу сжатым воздухом; доза — действие. Против сопротивляющейся цели Медтех делает атаку ближнего боя, и при успехе вместо урона вводит дозу.",
     "Airhypo", "Injects a drug with compressed air; one dose takes an action. Against an unwilling target a Medtech makes a melee attack and on a hit delivers the dose instead of damage."),
    ("gas_mask", 20, "Обычное", 352,
     "Маска-противогаз", "Фильтрует токсины и дым: надевший невосприимчив к газам и испарениям, которые нужно вдохнуть.",
     "Gas mask", "Filters the air: the wearer is immune to toxic gases and fumes that must be inhaled."),
    ("audio_recorder", 100, "Премиум", 352,
     "Диктофон", "Записывает до 24 часов звука на стандартную щепку памяти.",
     "Audio recorder", "Records up to 24 hours of audio onto a standard memory chip."),
    ("active_earmuffs", 1000, "Очень дорогое", 352,
     "Активные наушники", "Защита ушей: пользователя не оглушают громкие звуки, например светошумовые гранаты.",
     "Active earmuffs", "Ear protection: the user cannot be deafened by loud noises such as flashbangs."),
    ("binoculars", 50, "Ценное", 352,
     "Бинокль", "Увеличивает видимое в два-три раза.",
     "Binoculars", "Magnifies what you see two to three times."),
    ("braindance_viewer", 1000, "Очень дорогое", 352,
     "Очки для брейнданса", "Проигрывают брейнданс — запись пережитого опыта, который ощущаешь от лица исполнителя.",
     "Braindance viewer", "Plays braindances — recordings of lived experience felt from the performer's point of view."),
    ("bug_detector", 500, "Дорогое", 352,
     "Детектор жучков", "Пищит, когда в 2 м есть прослушивающий жучок или похожее устройство.",
     "Bug detector", "Beeps when a listening bug or similar device is within 2 m."),
    ("carryall", 20, "Обычное", 352,
     "Сумка", "Прочная нервущаяся сумка любого размера, от наплечной до дорожной.",
     "Carryall", "A tough bag of any size, from a shoulder bag to a duffel."),
    ("chemical_analyzer", 1000, "Очень дорогое", 352,
     "Химический анализатор", "За действие определяет химический состав вещества по обширной базе образцов.",
     "Chemical analyzer", "Takes an action to identify a substance's chemical makeup against a large sample database."),
    ("computer", 50, "Ценное", 352,
     "Компьютер", "Ноутбук или настольный компьютер для работы с текстом и сёрфинга по Дата Пулам.",
     "Computer", "A laptop or desktop for documents and browsing the Data Pools."),
    ("cryopump", 5000, "Роскошь", 353,
     "Крионасос", "Кейс с мешком и насосом: пациент в стазисе не делает спасбросков от смерти неделю, пока у мешка есть хоть 1 ПЗ. 1 заряд (заправка 50 eb), один взрослый; мешок — укрытие на 15 ПЗ. Только Медтехи.",
     "Cryopump", "Case with a bag and a pump: a patient in stasis makes no death saves for a week while the bag has at least 1 HP. 1 charge (refill 50 eb), one adult; the bag is cover with 15 HP. Medtechs only."),
    ("cryotank", 5000, "Роскошь", 353,
     "Криокамера", "Капсула для человека: после успешной проверки Медицинских технологий СЛ 13 пациент в стазисе сколько нужно; внутри ПЗ восстанавливаются вдвое быстрее. Укрытие на 30 ПЗ. Только Медтехи.",
     "Cryotank", "A human-sized capsule: after a successful Medical Tech check (DV 13) the patient stays in stasis as long as needed; HP recover twice as fast inside. Cover with 30 HP. Medtechs only."),
    ("cyberdeck_excellent", 1000, "Очень дорогое", 353,
     "Кибердека (отличного качества)", "9 слотов под программы и оборудование для работы в СЕТИ; нужны личный порт и нейролинк.",
     "Cyberdeck (excellent quality)", "9 slots for programs and hardware to work in the NET; needs an interface plug and a neurolink."),
    ("cyberdeck_poor", 100, "Премиум", 353,
     "Кибердека (плохого качества)", "5 слотов под программы и оборудование для работы в СЕТИ; нужны личный порт и нейролинк.",
     "Cyberdeck (poor quality)", "5 slots for programs and hardware to work in the NET; needs an interface plug and a neurolink."),
    ("cyberdeck_standard", 500, "Дорогое", 353,
     "Кибердека (обычного качества)", "7 слотов под программы и оборудование для работы в СЕТИ; нужны личный порт и нейролинк.",
     "Cyberdeck (standard quality)", "7 slots for programs and hardware to work in the NET; needs an interface plug and a neurolink."),
    ("burner_phone", 50, "Ценное", 353,
     "Одноразовый телефон", "Дешёвый телефон, за которым не следят: любимец Фиксеров и всех, кто не хочет слежки.",
     "Burner phone", "A cheap phone nobody tracks, favoured by Fixers and anyone avoiding surveillance."),
    ("drum_synth", 500, "Дорогое", 353,
     "Барабанный синтезатор", "Пластиковые пэды разного размера, имитирующие барабаны; для звука нужен усилитель.",
     "Drum synthesizer", "Plastic pads of various sizes that imitate drums; needs an amplifier to be heard."),
    ("duct_tape", 20, "Обычное", 353,
     "Скотч", "Клейкая лента, бывает светящейся в темноте — удобно метить тоннели, опасные места и тайники.",
     "Duct tape", "Sticky tape, sometimes glow-in-the-dark — handy for marking tunnels, hazards and caches."),
    ("electric_guitar", 500, "Дорогое", 353,
     "Электрогитара или другой инструмент", "Электронный инструмент на ваш вкус; чтобы его было слышно, нужен усилитель.",
     "Electric guitar or other instrument", "An electronic instrument of your choice; it needs an amplifier to be heard."),
    ("flashlight", 20, "Обычное", 353,
     "Фонарик", "Светит на 100 м в течение 10 часов, затем нужна зарядка.",
     "Flashlight", "Shines 100 m for 10 hours, then needs recharging."),
    ("food_stick", 10, "Дешевка", 353,
     "Пищевая палочка", "Злаковый батончик на перекус; вкус обычно отвратительный.",
     "Food stick", "A cereal snack bar; the taste is usually awful."),
    ("glow_paint", 20, "Обычное", 353,
     "Светящаяся краска", "Светящаяся в темноте краска в баллончике — для меток и граффити.",
     "Glow paint", "Glow-in-the-dark spray paint for marks and graffiti."),
    ("glowstick", 10, "Дешевка", 353,
     "Глоустик", "Одноразовая трубка: освещает площадку со стороной 4 м в течение 10 часов.",
     "Glow stick", "A disposable tube that lights a 4 m square for 10 hours."),
    ("grapple_gun", 100, "Премиум", 353,
     "Пистолет-крюк", "Действием выстреливает крюк на 30 м; трос (10 ПЗ) держит вдвое больше веса владельца, лазание без штрафа, отпустить можно без действия. Как оружие бесполезен, для захвата не годится.",
     "Grapple gun", "Fires a hook up to 30 m as an action; the line (10 HP) holds twice the user's weight, no climbing penalty, can be let go without an action. Useless as a weapon, cannot be used to grab."),
    ("handcuffs", 50, "Ценное", 353,
     "Наручники", "Легко ломаются, если ТЕЛ больше 10.",
     "Handcuffs", "Easily broken by anyone with BODY above 10."),
    ("tracker", 500, "Дорогое", 354,
     "Трекер (маячок)", "Отслеживает привязанный маячок размером с пуговицу на расстоянии до 1 км; дополнительный маячок стоит 50 eb.",
     "Tracker", "Tracks its paired button-sized beacon up to 1 km; an extra beacon costs 50 eb."),
    ("inflatable_bed", 20, "Обычное", 354,
     "Надувной матрас и спальник", "Самонадувающаяся кровать с тонким спальным мешком; в сложенном виде пакет 15×15 см.",
     "Inflatable bed and bedroll", "A self-inflating bed with a thin sleeping bag; folds into a 15×15 cm pack."),
    ("kibble_pack", 10, "Дешевка", 354,
     "Пачка Киббла", "Фольговая упаковка сухих хлопьев или вафель — одна порция еды.",
     "Kibble pack", "A foil pack of dry flakes or wafers — one meal."),
    ("exoskeleton_beta", 5000, "Роскошь", 354,
     "Внешний экзоскелет ß (бета)", "Пока подключён, поднимает ТЕЛ пользователя до 12 (не выше); ПЗ и спасбросок от смерти не меняются. Нужен 1 личный порт.",
     "External exoskeleton ß (beta)", "While connected, raises the user's BODY to 12 (no higher); HP and death save stay the same. Needs 1 interface plug."),
    ("exoskeleton_sigma", 1000, "Очень дорогое", 354,
     "Внешний экзоскелет ∑ (сигма)", "Пока подключён, поднимает ТЕЛ пользователя до 14 (не выше); ПЗ и спасбросок от смерти не меняются.",
     "External exoskeleton ∑ (sigma)", "While connected, raises the user's BODY to 14 (no higher); HP and death save stay the same."),
    ("lockpick_set", 20, "Обычное", 354,
     "Набор взломщика", "Небольшой набор инструментов для вскрытия механических замков.",
     "Lock-picking set", "A small kit of tools for opening mechanical locks."),
    ("medscanner", 1000, "Очень дорогое", 354,
     "Медсканер", "Диагностирует травмы и болезни, не требуя операции. +2 к «Первой помощи» и «Парамедику»; несколько медсканеров не складываются.",
     "Medscanner", "Diagnoses injuries and illness without surgery. +2 to First Aid and Paramedic; several medscanners do not stack."),
    ("medtech_bag", 100, "Премиум", 354,
     "Сумка Медтеха", "Набор медицинских инструментов — от хирургических степлеров до скальпелей; всё нужное, чтобы спасать жизни.",
     "Medtech bag", "A kit of medical tools from surgical staplers to scalpels — all you need to save lives."),
    ("memory_chip", 10, "Дешевка", 354,
     "Щепка", "Компактный накопитель для цифровых данных, бывает разных объёмов.",
     "Memory chip", "A compact digital storage chip, available in various sizes."),
    ("mre", 10, "Дешевка", 354,
     "Сухпай", "Саморазогревающийся набор еды: залей воды — и через пару минут будет сытное горячее блюдо.",
     "MRE", "A self-heating ration: add water and in a couple of minutes you have a hot meal."),
    ("personal_care", 20, "Обычное", 354,
     "Набор личной гигиены", "Зубная щётка и паста, салфетки, расчёска, крем для депиляции и т. п.",
     "Personal care pack", "Toothbrush and paste, wipes, a comb, depilatory cream and the like."),
    ("pocket_amp", 50, "Ценное", 354,
     "Карманный усилитель", "Размером с толстую книгу; звук на 100 м около 6 часов, тянет два инструмента сразу; нужна зарядка.",
     "Pocket amplifier", "The size of a thick book; sound carries 100 m for about 6 hours, drives two instruments at once; needs charging."),
    ("radar_detector", 500, "Дорогое", 354,
     "Радар-детектор", "Пищит, когда в радиусе 100 м работает радар.",
     "Radar detector", "Beeps when radar is active within 100 m."),
    ("radio_communicator", 100, "Премиум", 354,
     "Рация или коммуникатор", "Гарнитура радиосвязи на расстояние до 1 км.",
     "Radio communicator", "A headset for radio contact up to 1 km."),
    ("radio_scanner", 50, "Ценное", 354,
     "Радиосканер / музыкальный проигрыватель", "Играет музыку с щепки или из Дата Пула и находит работающие радиодиапазоны в радиусе 1 км; для шифрованных станций нужен дешифратор.",
     "Radio scanner / music player", "Plays music from a chip or the Data Pool and finds active radio bands within 1 km; encrypted stations need a descrambler."),
    ("flare", 10, "Дешевка", 354,
     "Сигнальная ракета", "Одноразовая, освещает квадрат со стороной 100 м в течение часа; бывает разных цветов.",
     "Flare", "Disposable, lights a 100 m square for an hour; comes in several colours."),
    ("rope", 20, "Обычное", 354,
     "Верёвка (60 м)", "Нейлоновая верёвка, выдерживает до 360 кг.",
     "Rope (60 m)", "Nylon rope that holds up to 360 kg."),
    ("scrambler", 500, "Дорогое", 354,
     "Шифратор / дешифратор", "Шифрует исходящие сообщения; дешифратор входит в комплект.",
     "Scrambler / descrambler", "Encrypts outgoing messages; the descrambler is included."),
    ("smart_glasses", 500, "Дорогое", 354,
     "Умные очки", "Два слота под модули кибер-глаза: даёт их преимущества, модули считаются парными, цена как за один глаз. Носить можно только одну пару.",
     "Smart glasses", "Two slots for cybereye options that give their benefits; options always count as a pair, priced as one eye. Only one pair can be worn."),
    ("tech_bag", 500, "Дорогое", 354,
     "Сумка Техника", "Набор для ремонта электроники и механизмов: мультитул, расходники для электрики, винты и болты, горелка, модули, монтировки, молоток.",
     "Tech bag", "A repair kit for electronics and machinery: multitool, electrical supplies, screws and bolts, torch, modules, pry bars and a hammer."),
    ("techscanner", 1000, "О. Дорогое", 355,
     "Техсканер", "Диагностирует технику и подсказывает ремонт. +2 к Знанию техники, Кибертехнике, Автомобильным, Морским и Авиационным технологиям, Электронике/Безопасности и Оружейнику; сам с собой не складывается.",
     "Techscanner", "Diagnoses machinery and suggests repairs. +2 to Basic Tech, Cybertech, Land, Sea and Air Vehicle Tech, Electronics/Security Tech and Weaponstech; does not stack with itself."),
    ("multitool", 100, "Премиум", 355,
     "Мультитул", "Универсальный инструмент: лезвие, плоскогубцы, отвёртки, напильники и щипцы в одном чехле.",
     "Multitool", "An all-in-one tool: a blade, pliers, screwdrivers, files and tongs in one case."),
    ("tent_kit", 50, "Ценное", 355,
     "Палатка и оборудование для кемпинга", "Одноместная палатка, самонагревающийся котелок (аккумулятора хватает на 2 часа, зарядка 5 минут) и дешёвая вилка.",
     "Tent and camping gear", "A one-person tent, a self-heating kettle (the battery lasts 2 hours, recharges in 5 minutes) and a cheap fork."),
    ("biotoxin_vial", 500, "Дорогое", 355,
     "Флакон биотоксина", "Действием наносится на лёгкое оружие ближнего боя на 30 минут: вместо урона «мясная» цель проверяет Сопротивление пыткам/наркотикам СЛ 15, при провале получает 3d6 мимо брони (броня цела).",
     "Biotoxin vial", "Applied to a light melee weapon as an action for 30 minutes: instead of damage a fleshy target makes a Resist Torture/Drugs check (DV 15) and on a failure takes 3d6 ignoring armor (armor is not damaged)."),
    ("poison_vial", 100, "Премиум", 355,
     "Флакон с ядом", "Действием наносится на лёгкое оружие ближнего боя на 30 минут: вместо урона «мясная» цель проверяет Сопротивление пыткам/наркотикам СЛ 13, при провале получает 2d6 мимо брони (броня цела).",
     "Poison vial", "Applied to a light melee weapon as an action for 30 minutes: instead of damage a fleshy target makes a Resist Torture/Drugs check (DV 13) and on a failure takes 2d6 ignoring armor (armor is not damaged)."),
    ("video_camera", 100, "Премиум", 355,
     "Видеокамера", "Записывает до 12 часов видео со звуком на стандартную щепку памяти.",
     "Video camera", "Records up to 12 hours of video with sound onto a standard memory chip."),
    ("virtuality_goggles", 100, "Премиум", 355,
     "Очки виртуальности", "Гарнитура, накладывающая киберпространство на реальный мир. Нужна не только нетраннерам: ею удобно видеть панели управления техники.",
     "Virtuality goggles", "A headset that overlays cyberspace on the real world. Not only for netrunners: handy for seeing the control panels of gear you plug into."),
]

# ---- ammunition. (code, price, printed category, page, ru name, ru usable, ru summary, en name, en usable, en summary)
AMMO = [
    ("standard", 10, "Дешевка", 345,
     "Стандартные боеприпасы", "Все, кроме гранат и ракет", "Обычные боеприпасы без особых свойств.",
     "Standard ammo", "All except grenades and rockets", "Plain ammunition with no special properties."),
    ("armor_piercing", 100, "Премиум", 345,
     "Бронебойные боеприпасы", "Все, кроме дроби", "Пробитая броня теряет 2 ОС вместо 1. Для гранат и взрывчатки — стандартные.",
     "Armor-piercing ammo", "All except shot", "Armor that is penetrated loses 2 SP instead of 1. Standard for grenades and explosives."),
    ("biotoxin", 500, "Дорогое", 345,
     "Боеприпасы с биотоксином", "Только стрелы и гранаты", "Урона нет: «мясная» цель проверяет Сопротивление пыткам/наркотикам СЛ 15, при провале получает 3d6 мимо брони (броня цела).",
     "Biotoxin ammo", "Arrows and grenades only", "No damage: a fleshy target makes a Resist Torture/Drugs check (DV 15) and on a failure takes 3d6 ignoring armor (armor is not damaged)."),
    ("emp", 500, "Дорогое", 345,
     "ЭМИ-боеприпасы", "Только гранаты", "Урона нет: цель проверяет Кибертехнику СЛ 15; при провале ГМ выбирает два импланта или электронных предмета, они отключаются на 1 минуту.",
     "EMP ammo", "Grenades only", "No damage: the target makes a Cybertech check (DV 15); on a failure the GM picks two implants or electronic items and they shut down for 1 minute."),
    ("expansive", 100, "Премиум", 345,
     "Экспансивные боеприпасы", "Стрелы, пули и жаканы", "При попадании наносят критическое повреждение «Инородное тело». Если попадание и так вызвало крит, жертва перебрасывает таблицу, пока не получит ещё одну иную травму; бонусного урона у второй и следующих нет.",
     "Expansive ammo", "Arrows, bullets and slugs", "A hit deals the Foreign Object critical injury. If the hit also caused a critical injury, the victim rolls the table again until they get one more different injury; the second and later ones deal no bonus damage."),
    ("flashbang", 100, "Премиум", 346,
     "Светошумовые боеприпасы", "Только гранаты", "Урона нет: цель проверяет Сопротивление пыткам/наркотикам СЛ 15; при провале на минуту получает «Повреждение глаза» и «Повреждённое ухо» без бонусного урона.",
     "Flashbang ammo", "Grenades only", "No damage: the target makes a Resist Torture/Drugs check (DV 15); on a failure they suffer Damaged Eye and Damaged Ear for a minute with no bonus damage."),
    ("incendiary", 100, "Премиум", 346,
     "Зажигательные боеприпасы", "Стрелы, патроны, гранаты, жаканы и дробь", "При пробитии брони поджигают цель: в конце каждого её хода 2 ПЗ мимо брони, пока она не потратит действие на тушение. Не складывается.",
     "Incendiary ammo", "Arrows, cartridges, grenades, slugs and shot", "Penetrating armor sets the target on fire: 2 HP ignoring armor at the end of each of their turns until they spend an action putting it out. Does not stack."),
    ("poison", 100, "Премиум", 346,
     "Боеприпасы с ядом", "Только стрелы и гранаты", "Урона нет: «мясная» цель проверяет Сопротивление пыткам/наркотикам СЛ 13, при провале получает 2d6 мимо брони (броня цела).",
     "Poison ammo", "Arrows and grenades only", "No damage: a fleshy target makes a Resist Torture/Drugs check (DV 13) and on a failure takes 2d6 ignoring armor (armor is not damaged)."),
    ("rubber", 10, "Дешевка", 346,
     "Резиновые боеприпасы", "Стрелы, пули и жаканы", "Урон не вызывает критов, не портит броню и не опускает ПЗ ниже 1. Для экзотического оружия — стандартные.",
     "Rubber ammo", "Arrows, bullets and slugs", "Damage cannot cause critical injuries, does not damage armor and cannot take HP below 1. Standard for exotic weapons."),
    ("sleep", 500, "Дорогое", 346,
     "Усыпляющие боеприпасы", "Стрелы и гранаты", "Урона нет: «мясная» цель проверяет Сопротивление пыткам/наркотикам СЛ 13; при провале падает и теряет сознание на 1 минуту (просыпается от урона или чужого действия).",
     "Sleep ammo", "Arrows and grenades", "No damage: a fleshy target makes a Resist Torture/Drugs check (DV 13); on a failure they fall unconscious for 1 minute (damage or another's action wakes them)."),
    ("smart", 500, "Дорогое", 346,
     "Умные боеприпасы", "Стрелы, пули, жаканы и ракеты", "Нужен имплант «Тактический прицел». Одиночный выстрел, промахнувшийся на 4 или меньше, получает второй шанс: вместо СТАТ + навык + 1d10 бросается 1d10 + 10 (Удачу добавлять можно, цель может уклоняться).",
     "Smart ammo", "Arrows, bullets, slugs and rockets", "Needs the Tactical Sight implant. A single shot that misses by 4 or less gets a second chance: roll 1d10 + 10 instead of STAT + skill + 1d10 (Luck can be added, the target may still dodge)."),
    ("smoke", 50, "Ценное", 347,
     "Дымовые боеприпасы", "Только гранаты", "Урона нет: при попадании облако дыма 10×10 на 1 минуту, −4 к действиям, которым мешает дым.",
     "Smoke ammo", "Grenades only", "No damage: on impact a 10×10 cloud of smoke for 1 minute, −4 to actions the smoke hinders."),
    ("tear_gas", 50, "Ценное", 347,
     "Слезоточивый газ", "Только гранаты", "Урона нет: цель с глазами проверяет Сопротивление пыткам/наркотикам СЛ 13; при провале на минуту получает «Повреждение глаза» без бонусного урона.",
     "Tear gas", "Grenades only", "No damage: a target with eyes makes a Resist Torture/Drugs check (DV 13); on a failure they suffer Damaged Eye for a minute with no bonus damage."),
]

# ---- weapon modifications. (code, price, printed category, page, slots, skill ids,
#                             ru name, ru summary, en name, en summary)
TACTICAL = ["shoulder_arms"]
NO_BOWS = ["handgun", "shoulder_arms", "heavy_weapons"]
MODS = [
    ("bayonet", 100, "Премиум", 343, 1, TACTICAL,
     "Штык", "Оружие можно использовать в рукопашной как лёгкое оружие ближнего боя. С ним нельзя спрятать оружие под одеждой.",
     "Bayonet", "The weapon can be used in melee as a light melee weapon. It cannot be hidden under clothes."),
    ("drum_magazine", 500, "Дорогое", 343, 1, NO_BOWS,
     "Барабанный магазин", "Вмещает больше патронов (таблица магазинов); на оружие ставится один магазин. Спрятать оружие нельзя.",
     "Drum magazine", "Holds more rounds (see the magazine table); only one magazine per weapon. The weapon cannot be hidden."),
    ("extended_magazine", 100, "Премиум", 343, 1, NO_BOWS,
     "Увеличенный магазин", "Вмещает больше патронов (таблица магазинов); на оружие ставится один магазин. Спрятать оружие нельзя.",
     "Extended magazine", "Holds more rounds (see the magazine table); only one magazine per weapon. The weapon cannot be hidden."),
    ("underbarrel_grenade_launcher", 500, "Дорогое", 343, 2, TACTICAL,
     "Подствольный гранатомёт", "Если держать оружие двумя руками, из него можно стрелять как из гранатомёта, одна граната в подствольнике. Спрятать нельзя. Занимает 2 слота.",
     "Underbarrel grenade launcher", "Held in two hands the weapon can be used as a grenade launcher with 1 grenade in the underbarrel. Cannot be hidden. Takes 2 slots."),
    ("infrared_night_scope", 500, "Дорогое", 343, 1, TACTICAL,
     "Инфракрасный прицел ночного видения", "Убирает штрафы за темноту, дым, туман и т. п.; видно тепло тела, но не то, какой именно киберимплант у цели.",
     "Infrared night-vision scope", "Removes penalties for darkness, smoke, fog and the like; shows body heat, but not which cyberware the target has."),
    ("underbarrel_shotgun", 500, "Дорогое", 343, 2, TACTICAL,
     "Подствольный дробовик", "Если держать оружие двумя руками, из него можно стрелять как из дробовика, магазин на 2 боеприпаса. Спрятать нельзя. Занимает 2 слота.",
     "Underbarrel shotgun", "Held in two hands the weapon can be used as a shotgun with a 2-round magazine. Cannot be hidden. Takes 2 slots."),
    ("smartlink_connector", 500, "Дорогое", 344, 2, NO_BOWS,
     "Коннектор смартлинка", "Делает оружие умным: +1 к попаданию при дальней атаке. Нужны специальные импланты (личный порт или подкожный смартлинк плюс нейролинк); установка и снятие — час. Занимает 2 слота.",
     "Smartlink connector", "Makes the weapon smart: +1 to hit on a ranged attack. Needs special cyberware (an interface plug or a subdermal smartlink plus a neurolink); installing or removing takes an hour. Takes 2 slots."),
    ("sniper_scope", 100, "Премиум", 344, 1, NO_BOWS,
     "Снайперский прицел", "Позволяет разглядеть детали до 800 м; при одиночном или прицельном выстреле на 51 м и дальше +1 к проверке. Не складывается с имплантом ТелеОптика.",
     "Sniper scope", "Lets you see details up to 800 m; +1 to the check on a single or aimed shot at 51 m or farther. Does not stack with the TeleOptics implant."),
]

# ---- magazine table p. 344: weapon code, standard, extended, drum
MAGAZINES = [
    ("medium_pistol", 12, 18, 36),
    ("heavy_pistol", 8, 14, 28),
    ("very_heavy_pistol", 8, 14, 28),
    ("submachine_gun", 30, 40, 50),
    ("heavy_submachine_gun", 40, 50, 60),
    ("shotgun", 4, 8, 16),
    ("assault_rifle", 25, 35, 45),
    ("sniper_rifle", 4, 8, 12),
    ("grenade_launcher", 2, 4, 6),
    ("rocket_launcher", 1, 2, 3),
]

# ---- prices of armor and shields (p. 97 and p. 350 agree): code -> (price, printed category)
ARMOR_PRICES = {
    "leather": (20, "Обычное"),
    "kevlar": (50, "Ценное"),
    "light_armorjack": (100, "Премиум"),
    "bodyweight_suit": (1000, "Очень дорогое"),
    "medium_armorjack": (100, "Премиум"),
    "heavy_armorjack": (500, "Дорогое"),
    "flak": (500, "Дорогое"),
    "metalgear": (5000, "Роскошь"),
}
SHIELD_PRICES = {"bulletproof_shield": (100, "Премиум")}


# The namespace string below is not the product name: it only has to stay the same so that uuids stay stable.
def uid(collection: str, code: str) -> str:
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"favoterm:{collection}:{code}"))


def write(path: Path, name: str, entries: list, version: int = 1) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    doc = {"name": name, "version": version, "entries": entries}
    path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    for lang in ("ru", "en"):
        i = 0 if lang == "ru" else 1
        gear, ammo, mods = [], [], []
        for row in GEAR:
            code, price, printed, page = row[:4]
            check_category(code, price, printed)
            name, summary = (row[4], row[5]) if i == 0 else (row[6], row[7])
            gear.append({"uuid": uid("gear", code), "code": code, "name": name, "price": price,
                         "summary": summary, "pack": "core", "source": {"page": page}})
        for row in AMMO:
            code, price, printed, page = row[:4]
            check_category(code, price, printed)
            name, usable, summary = row[4:7] if i == 0 else row[7:10]
            ammo.append({"uuid": uid("ammo", code), "code": code, "name": name, "price": price,
                         "usable_with": usable, "summary": summary, "pack": "core", "source": {"page": page}})
        for row in MODS:
            code, price, printed, page, slots, skills = row[:6]
            check_category(code, price, printed)
            name, summary = row[6:8] if i == 0 else row[8:10]
            mods.append({"uuid": uid("weapon_mods", code), "code": code, "name": name, "price": price,
                         "slots": slots, "skills": skills, "summary": summary, "pack": "core",
                         "source": {"page": page}})
        write(SEED / lang / "gear.json", "gear", gear)
        write(SEED / lang / "ammo.json", "ammo", ammo)
        write(SEED / lang / "weapon_mods.json", "weapon_mods", mods)

        # Prices of armor and shields go into the existing files.
        for fname, prices in (("armor", ARMOR_PRICES), ("shields", SHIELD_PRICES)):
            path = SEED / lang / f"{fname}.json"
            doc = json.loads(path.read_text(encoding="utf-8"))
            for e in doc["entries"]:
                price = prices.get(e["code"])
                if price:
                    check_category(e["code"], *price)
                e["price"] = price[0] if price else None
                if fname == "armor":
                    assert price, e["code"]
                    # keep a stable key order: price after penalty
                    e.update({k: e.pop(k) for k in ("price", "pack", "source") if k in e})
                else:
                    e.update({k: e.pop(k) for k in ("price", "pack", "source") if k in e})
            doc["version"] = max(doc["version"], 2)
            path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    write(SEED / "magazines.json", "magazines",
          [{"id": c, "standard": s, "extended": e, "drum": d, "pack": "core", "source": {"page": 344}}
           for c, s, e, d in MAGAZINES])
    write(SEED / "price_categories.json", "price_categories",
          [{"id": cid, "min_price": minimum, "pack": "core", "source": {"page": 385}}
           for cid, minimum, _ in CATEGORIES])
    print(f"gear {len(GEAR)}, ammo {len(AMMO)}, mods {len(MODS)}, magazines {len(MAGAZINES)}")


main()
