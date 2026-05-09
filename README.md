# ArchiSchema

Collaborative database schema diagram editor. Draw tables, relationships and
indexes on a canvas, generate SQL for PostgreSQL / MySQL / SQLite dialects,
and share the session live with teammates.

- **Frontend**: Leptos 0.8 (WASM, SSR + hydrate)
- **Backend**: axum 0.8 on tokio 1
- **Database**: PostgreSQL 18, sqlx 0.8
- **Realtime**: WebSocket + Yjs-compatible CRDT (yrs 0.25 / y-sync 0.4)
- **Styling**: Tailwind CSS 3.4 with ArchiSchema brand tokens in `style/input.css`

## What the editor provides

- Visual schema canvas with draggable tables, relationship lines and inline table cards.
- Source/Code mode with SQL DDL editing, line gutter and diagnostics rail.
- Table and column inspectors for names, types, constraints, defaults and FK relationships.
- LiveShare rooms with presence, cursors, graph-operation sync and persisted snapshots.
- AI chat/actions routed through the same graph operation path as manual edits.

---

## Quick start

### Prerequisites

- Rust ≥ 1.94 (2024 edition)
- [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) 0.3
- [`go-task`](https://taskfile.dev) (optional but recommended)
- Node.js (only for Tailwind CLI)
- Docker + Docker Compose

Install the Rust tooling once:

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
```

### Boot the database

```bash
docker-compose up -d    # starts postgres:18.1 on localhost:5432
```

### Configure environment

```bash
cp .env.example .env
# then edit .env:
#   DATABASE_URL    — uncomment the default postgres line
#   SECRET_KEY      — openssl rand -base64 64
#   JWT_SECRET      — openssl rand -base64 64
#   OPENAPI_TOKEN   — optional; OpenRouter key for the AI chat feature
```

Migrations run automatically on server startup (see
`src/core/db/pool.rs::create_pool_with_migrations`).

### Install JS deps

```bash
npm install
```

### Run

```bash
task dev                       # compiles Tailwind + runs cargo-leptos watch
# or, without go-task:
npm run build:css
cargo leptos watch -c
```

The editor serves at `http://127.0.0.1:3000` and watches for changes.

---

## Common tasks

```bash
task dev        # dev server with hot reload
task release    # optimised build + brotli/gzip compression of static assets
task test       # unit tests (server-side)
task clippy     # lints
task fmt        # rustfmt
task check      # check + fmt + clippy + test
task clean      # remove build artefacts
```

For the WASM side alone:

```bash
cargo check --no-default-features --features hydrate --target wasm32-unknown-unknown
```

---

## Project layout

See [AGENTS.md](./AGENTS.md) for the full guided tour (module map, feature
flags, LiveShare subsystem, HTTP routing table, house rules).

For the high-level architecture and data flow, see
[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).

For current editor UI rules, redesign boundaries and component ownership, see
[docs/UI_DESIGN.md](./docs/UI_DESIGN.md). Brand tokens and voice live in
[BRAND.md](./BRAND.md), while applied CSS tokens live in `style/input.css`.

For the collaborative editing subsystem, see
[docs/liveshare/LIVESHARE_COMPLETE_GUIDE.md](./docs/liveshare/LIVESHARE_COMPLETE_GUIDE.md).

---

## API surface

| Path prefix                       | What it does                       |
| --------------------------------- | ---------------------------------- |
| `/`                               | Leptos SSR pages                   |
| `/api/auth/*`                     | Register / login / refresh / me    |
| `/api/diagrams/*`                 | Diagram CRUD                       |
| `/api/folders/*`                  | Folder CRUD                        |
| `/api/diagrams/{id}/shares/*`     | Per-diagram share tokens           |
| `/api/ai/chat`                    | Proxy to OpenRouter                |
| `/room/{room_id}` (REST + WS)     | LiveShare room + WebSocket         |

All REST endpoints (except `/api/auth/register|login` and `/api/ai/chat`)
require a `Bearer` JWT access token. Access tokens live 15 minutes;
refresh tokens 7 days.

---

## Tests

```bash
cargo test --features ssr                       # unit tests only (fast)
docker-compose up -d
DATABASE_URL=postgres://postgres:postgres@localhost:5432/archischema \
  cargo test --features ssr -- --ignored        # integration tests (~56, need DB)
```

For UI/CSS changes, use the focused check sequence:

```bash
npm run build:css
cargo check --features ssr
cargo check --no-default-features --features hydrate --target wasm32-unknown-unknown
```

---

## License

MIT — see [LICENSE](./LICENSE).
