# Как начать сессию в репозитории сервера

Эта папка — заготовка для нового репозитория `Hex1s/red-companion-server`. Другая сессия Claude **не видит файлов репозитория приложения**, поэтому всё нужное для старта собирается в «пакет» одной командой.

## 1. Собрать пакет

В репозитории приложения (`D:\projects\red-companion`):

```
cargo test --workspace            # обновит schemas/*.schema.json и schemas/hash-vectors.json
python tools/export-server-bundle.py D:\projects\red-companion-server
```

Скрипт кладёт в целевую папку (она может быть пустым клоном нового репозитория):

| Куда | Что |
|---|---|
| `CLAUDE.md` | инструкции для сессии сервера (из `docs/server-bootstrap/CLAUDE.md`) |
| `docs/SERVER-SPEC.md` | ТЗ (главный документ) |
| `docs/GLOSSARY.md`, `docs/BOOK-PLAN.md` | термины и план данных: для понимания, не для правки |
| `docs/ROADMAP-excerpt.md` | выдержка из роадмапа приложения: решения, фаза 7 и 8, задачи 2.0, 2.0b, 2.12, 2.22 |
| `contracts/schemas/` | JSON Schema всех коллекций и `hash-vectors.json` |
| `contracts/seed/` | эталонные данные (ядро и содержимое для `ru`, `en`) |
| `reference/rules-core/` | Rust-код приложения для справки: типы и валидаторы (`catalog.rs`), слой данных (`data_source.rs`), коды ошибок (`error_codes.rs`) |
| `README.md` | этот файл |

Целевая папка — репозиторий **сервера**, поэтому скрипт ничего не коммитит; коммиты делает пользователь.

## 2. Открыть сессию

Открой Claude Code в папке нового репозитория и начни так (подставь нужное):

> Прочитай `CLAUDE.md` и `docs/SERVER-SPEC.md`. Начнём с этапа S0: каркас Next.js + Prisma + Docker Compose, CI и скрипт синхронизации контрактов. Вопросы по ТЗ задай мне опросом перед началом.

## 3. Когда схемы или данные в приложении изменились

Повтори шаг 1. Скрипт перезапишет `contracts/` и `reference/` (остальные файлы пакета он не трогает, если они уже есть, кроме `docs/SERVER-SPEC.md`: его всегда обновляет). Если ТЗ правится в сессии сервера, не забудь перенести правки обратно в `docs/SERVER-SPEC.md` приложения — **источник ТЗ один: сервер после старта** (решение: с момента начала S0 правки ТЗ делаются в репозитории сервера, а копия в репозитории приложения становится архивной; предложение, подтвердите).

## 4. Ссылки на приложение (если репозиторий доступен сессии)

- Репозиторий: https://github.com/Hex1s/red-companion
- Роадмап: https://github.com/Hex1s/red-companion/blob/main/docs/ROADMAP.md
- План данных и лицензия: https://github.com/Hex1s/red-companion/blob/main/docs/BOOK-PLAN.md
- Схемы: https://github.com/Hex1s/red-companion/tree/main/schemas
- Seed-данные: https://github.com/Hex1s/red-companion/tree/main/crates/rules-core/data/seed
- Типы и валидаторы: https://github.com/Hex1s/red-companion/blob/main/crates/rules-core/src/catalog.rs
