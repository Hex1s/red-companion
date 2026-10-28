# Changelog

Формат нужен CI релиза (`tools/upload-release.mjs`): раздел версии `## X.Y.Z`, внутри подразделы `### ru` и `### en`. Текст подраздела уходит на сервер как есть и показывается в приложении («Что нового») и на сайте. Релиз по тегу `vX.Y.Z` не соберётся, пока в этом файле нет раздела версии.

## 0.9.4

### ru

- Справочники (оружие, броня, снаряжение, NPC и другой контент) теперь приходят с сервера: при первом запуске предлагается выбрать язык и режим (по запросу или только скачанное), потом всё доступно кнопкой «Справочники с сервера» в шапке. Подлинность данных проверяется подписью. Механики правил (травмы, навыки, роли, таблицы дальности) остаются встроенными и работают без сети.
- В базе появились таблицы для сохранённых справочников (перед обновлением базы создаётся резервная копия).
- Уведомление об обновлении при запуске не повторяется чаще раза в 6 часов, метка на кнопке «О приложении» остаётся.

### en

- References (weapons, armor, gear, NPCs and other content) now come from the server: the first start offers to choose the language and mode (on request or downloaded only), and everything is then available with the "Server references" button in the header. The data is checked by a signature. The rules mechanics (injuries, skills, roles, range tables) stay built in and work offline.
- The database gets tables for the saved references (it is backed up before the upgrade).
- The start-up update notice is not repeated more often than every 6 hours; the mark on the About button stays.

## 0.9.3

### ru

- Проверка обновления при запуске: если вышла новая версия, в углу появляется уведомление, а на кнопке «О приложении» горит метка.

### en

- Update check at start: when a newer version is out, a notice appears and the About button gets a mark.

## 0.9.2

### ru

- Пробная сборка для проверки обновления.

### en

- Rehearsal build to test the update flow.

## 1.0.0

### ru

- Первый выпуск: персонажи и сессии, бой с ПЗ, ОС, укрытиями и спасбросками, энкаунтеры, справочники, экспорт и импорт.
- Справочники (оружие, броня, снаряжение, NPC и другой контент) приходят с сервера по запросу или целиком, подлинность данных проверяется подписью. Механики правил (травмы, навыки, роли, таблицы дальности) встроены и работают без сети.
- Windows, macOS и Android; на планшете интерфейс адаптируется под касание.
- Обновление приложения по кнопке в «О приложении» и уведомление при запуске.
- Перед обновлением базы создаётся резервная копия; база от более новой версии не открывается и остаётся нетронутой.

### en

- First release: characters and sessions, combat with HP, SP, cover and death saves, encounters, reference pages, export and import.
- References (weapons, armor, gear, NPCs and other content) come from the server on request or all at once, and the data is checked by a signature. The rules mechanics (injuries, skills, roles, range tables) are built in and work offline.
- Windows, macOS and Android; the interface adapts to touch on tablets.
- In-app update from the About dialog and a notice at start.
- The database is backed up before it is upgraded; a database from a newer version is not opened and stays untouched.

## 0.9.1

### ru

- Пробная сборка для проверки обновления.

### en

- Rehearsal build to test the update flow.

## 0.9.0

### ru

- Пробная сборка для проверки обновления.

### en

- Rehearsal build to test the update flow.

## 0.1.0

### ru

- Первая сборка.

### en

- First build.
