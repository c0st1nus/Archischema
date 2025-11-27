# Быстрый старт Diagramix

Это краткое руководство для быстрого запуска проекта.

## Предварительные требования

Убедитесь, что у вас установлены:

- **Rust** (stable) — [установить](https://rustup.rs/)
- **Node.js** 18+ — [установить](https://nodejs.org/)
- **cargo-leptos** — инструмент для Leptos проектов

## Установка за 5 минут

### 1. Установите Rust и инструменты

```bash
# Установите Rust (если ещё не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Добавьте WASM target
rustup target add wasm32-unknown-unknown

# Установите cargo-leptos
cargo install cargo-leptos --locked
```

### 2. Клонируйте и настройте проект

```bash
# Перейдите в директорию проекта
cd diagramix

# Установите NPM зависимости (только Tailwind CSS)
npm install

# Скомпилируйте CSS
npm run build:css
```

### 3. Запустите dev-сервер

```bash
# Запуск с hot-reload
cargo leptos watch
```

Откройте браузер: **http://127.0.0.1:3000**

## Что дальше?

### Режим разработки с автообновлением CSS

Откройте два терминала:

**Терминал 1** — сервер Leptos:
```bash
cargo leptos watch
```

**Терминал 2** — Tailwind в watch режиме:
```bash
npm run watch:css
```

### Сборка для production

```bash
# 1. Скомпилировать CSS
npm run build:css

# 2. Собрать проект
cargo leptos build --release
```

Готовые файлы:
- Сервер: `target/server/release/diagramix`
- WASM + JS: `target/site/`

Запуск production сервера:
```bash
./target/server/release/diagramix
```

## Структура команд

| Команда | Описание |
|---------|----------|
| `cargo leptos watch` | Dev-сервер с hot-reload |
| `cargo leptos build --release` | Production сборка |
| `cargo check` | Проверка серверного кода |
| `cargo check --lib --features hydrate --target wasm32-unknown-unknown --no-default-features` | Проверка WASM кода |
| `npm run build:css` | Скомпилировать Tailwind CSS |
| `npm run watch:css` | Tailwind в watch режиме |

## Возможные проблемы

### Ошибка "cargo-leptos not found"

```bash
cargo install cargo-leptos --locked
```

### Ошибка "target wasm32-unknown-unknown not found"

```bash
rustup target add wasm32-unknown-unknown
```

### CSS не применяется

```bash
# Убедитесь, что CSS скомпилирован
npm run build:css

# Проверьте, что существует файл
ls -lh style/output.css
```

### Порт 3000 уже занят

Измените порт в `Cargo.toml`:
```toml
[package.metadata.leptos]
site-addr = "127.0.0.1:3001"  # Ваш порт
reload-port = 3002
```

## Полезные ссылки

- [README.md](README.md) — полная документация
- [ARCHITECTURE.md](ARCHITECTURE.md) — описание архитектуры
- [Leptos Book](https://book.leptos.dev/) — документация Leptos
- [Tailwind Docs](https://tailwindcss.com/docs) — документация Tailwind

## Минимальный пример

Создать свою схему:

```rust
use diagramix::core::*;

let mut graph = SchemaGraph::new();

let users = graph.add_node(
    TableNode::new("users")
        .with_position(100.0, 100.0)
        .add_column(Column::new("id", "INTEGER").primary_key())
        .add_column(Column::new("name", "VARCHAR(255)").not_null())
);

let posts = graph.add_node(
    TableNode::new("posts")
        .with_position(400.0, 100.0)
        .add_column(Column::new("id", "INTEGER").primary_key())
        .add_column(Column::new("user_id", "INTEGER").not_null())
);

graph.add_edge(
    users,
    posts,
    Relationship::new("user_posts", RelationshipType::OneToMany, "id", "user_id")
);
```

---

**Готово! Теперь вы можете начать работу с Diagramix.**