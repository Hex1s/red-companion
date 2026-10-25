# Глоссарий

> Файл собирается скриптом `python tools/gen-glossary.py` из локалей и seed-данных; руками правятся только синонимы (словарь `SYNONYMS` в скрипте).

Термины правил имеют **устойчивые ID** (английские, snake_case). В коде, данных и ключах локализации используются только ID; переводы живут в `src/i18n/locales/{ru,en}.json`.

Русская книга называет одно и то же по-разному (например, «Сломанная нога» в тексте и «Перелом ноги» в таблице; СЛ и DV вперемешку). Правило: один ID, одно отображаемое название в локали, остальные варианты — в колонке «Синонимы книги».

**Английские названия** подобраны без английского издания книги: колонка «en» — это то, что показывает приложение, а не сверенный официальный текст. Когда издание будет под рукой, сверить и поправить в `en.json`.

## Общие термины

| ID | ru | en | Синонимы книги |
| --- | --- | --- | --- |
| `hp` | ПЗ — Пункты здоровья | HP — Hit Points |  |
| `sp` | ОС — Очки стойкости брони | SP — Stopping Power | очки стойкости, стойкость брони |
| `dv` | СЛ — Сложность | DV — Difficulty Value |  |
| `humanity` | ЧЕЛ — Человечность | HUM — Humanity |  |
| `death_save` | Спасбросок от смерти | Death save |  |
| `critical_injury` | Критическое повреждение | Critical injury |  |
| `cover` | Укрытие | Cover |  |
| `ablation` | Абляция | Ablation |  |

## Ранение, броня, область

| ID | ru | en | Синонимы книги |
| --- | --- | --- | --- |
| `wound.healthy` | Здоров | Healthy |  |
| `wound.light` | Лёгкое ранение | Lightly wounded |  |
| `wound.serious` | Тяжёлое ранение | Seriously wounded |  |
| `wound.mortal` | Смертельное ранение | Mortally wounded |  |
| `wound.dead` | Мёртв | Dead |  |
| `armor_mode.full` | Полная броня | Full armor |  |
| `armor_mode.half` | Половина брони | Half armor |  |
| `armor_mode.ignored` | Броня игнорируется | Armor ignored |  |
| `area.head` | Голова | Head |  |
| `area.body` | Тело | Body |  |

## Характеристики

Аббревиатуры русской редакции; ID — английские аббревиатуры оригинала.

| ID | ru | en | Синонимы книги |
| --- | --- | --- | --- |
| `INT` | ИНТ | INT |  |
| `REF` | РЕА | REF |  |
| `DEX` | ЛВК | DEX |  |
| `TECH` | ТЕХ | TECH |  |
| `COOL` | ХАР | COOL |  |
| `WILL` | ВОЛЯ | WILL |  |
| `LUCK` | УДЧ | LUCK |  |
| `MOVE` | СКО | MOVE |  |
| `BODY` | ТЕЛ | BODY |  |
| `EMP` | ЭМП | EMP |  |

## Критические повреждения

ID — как в `critical_injuries.json`; у «Инородного тела» два ID по областям. «Бросок» — 2d6.

| ID | Область | Бросок | ru | en | Синонимы книги | Стр. |
| --- | --- | --- | --- | --- | --- | --- |
| `dismembered_arm` | тело | 2 | Оторванная рука | Dismembered Arm |  | 187 |
| `dismembered_hand` | тело | 3 | Оторванное запястье | Dismembered Hand |  | 187 |
| `collapsed_lung` | тело | 4 | Разрыв лёгкого | Collapsed Lung |  | 187 |
| `broken_ribs` | тело | 5 | Перелом рёбер | Broken Ribs |  | 187 |
| `broken_arm` | тело | 6 | Перелом руки | Broken Arm | «Сломаной рукой» — опечатка книги (стр. 221) | 187 |
| `foreign_object_body` | тело | 7 | Инородное тело | Foreign Object |  | 187 |
| `broken_leg` | тело | 8 | Перелом ноги | Broken Leg | «Сломанная нога» (текст про прицельный выстрел, стр. 170) | 187 |
| `torn_muscle` | тело | 9 | Разрыв мышцы | Torn Muscle |  | 187 |
| `spinal_injury` | тело | 10 | Травма позвоночника | Spinal Injury |  | 187 |
| `crushed_fingers` | тело | 11 | Раздробленные пальцы | Crushed Fingers |  | 187 |
| `dismembered_leg` | тело | 12 | Оторванная нога | Dismembered Leg |  | 187 |
| `lost_eye` | голова | 2 | Потеря глаза | Lost Eye |  | 188 |
| `brain_injury` | голова | 3 | Черепно-мозговая травма | Brain Injury |  | 188 |
| `damaged_eye` | голова | 4 | Повреждение глаза | Damaged Eye |  | 188 |
| `concussion` | голова | 5 | Сотрясение мозга | Concussion |  | 188 |
| `broken_jaw` | голова | 6 | Перелом челюсти | Broken Jaw |  | 188 |
| `foreign_object_head` | голова | 7 | Инородное тело | Foreign Object |  | 188 |
| `broken_neck` | голова | 8 | Перелом шеи | Whiplash | смысл ru и en расходится: ru «Перелом шеи», en показывает Whiplash (хлыстовая травма) — сверить с английским изданием | 188 |
| `cracked_skull` | голова | 9 | Перелом черепа | Cracked Skull |  | 188 |
| `damaged_ear` | голова | 10 | Повреждение слуха | Damaged Ear |  | 188 |
| `crushed_windpipe` | голова | 11 | Травма трахеи | Crushed Windpipe |  | 188 |
| `lost_ear` | голова | 12 | Оторванное ухо | Lost Ear |  | 188 |
