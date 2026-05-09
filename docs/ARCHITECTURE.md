# ArchiSchema — Architecture Overview

This document describes the high-level architecture of ArchiSchema and the
data flow between major subsystems. For the collaborative editing layer
(LiveShare), see [liveshare/LIVESHARE_COMPLETE_GUIDE.md](./liveshare/LIVESHARE_COMPLETE_GUIDE.md).

---

## 1. Deployment shape

One Rust binary serves:

- SSR for the Leptos SPA
- Static asset delivery from `target/site/pkg` (with `precompressed_br` + `precompressed_gzip`)
- REST APIs for auth, diagrams, folders, sharing, AI chat
- WebSocket rooms for live collaboration

```
                 ┌───────────────────────────────────────────┐
    Browser ◀──▶ │         axum on tokio runtime             │ ◀──▶  PostgreSQL 18
    (WASM SPA)   │                                           │          (sqlx)
                 │  ├── leptos-axum: SSR + /pkg/*            │
                 │  ├── /api/auth/*      ← AuthService       │
                 │  ├── /api/diagrams/*  ← DiagramRepository │
                 │  ├── /api/folders/*   ← FolderRepository  │
                 │  ├── /api/diagrams/{}/shares/* ← ShareRepo│
                 │  ├── /api/ai/chat     ← reqwest → OpenRouter
                 │  └── /room/{id} + WS  ← LiveshareState    │
                 └───────────────────────────────────────────┘
```

The same process runs in dev (`task dev` — `cargo leptos watch`) and in
release (`task release` — optimised binary + compressed assets).

---

## 2. Runtime layers

| Layer              | Where                                    | Notes                                       |
| ------------------ | ---------------------------------------- | ------------------------------------------- |
| Presentation       | `src/ui/*`                               | Leptos components; CSR + hydrate            |
| Page routing       | `src/app.rs`                             | `leptos_router` declarations                |
| Server entrypoint  | `src/main.rs`                            | Wires repositories → services → routers     |
| Domain             | `src/core/schema.rs`, `validation.rs`    | Graph of tables/relationships (petgraph)    |
| Persistence        | `src/core/db/*`                          | `PgPool` + per-aggregate repositories       |
| Realtime           | `src/core/liveshare/*` + `src/ui/liveshare_client.rs` | Yjs-compatible CRDT + throttling |
| AI proxy           | `src/core/ai_api.rs`, `core/ai_tools.rs` | Streams OpenRouter responses                |

---

## 3. Schema graph

The canonical domain model is `SchemaGraph` (in `src/core/schema.rs`):

- Backed by `petgraph::Graph<TableNode, Relationship>`.
- Every `TableNode` carries a **stable `Uuid`** plus a human `name`,
  `columns`, and optional canvas `position`. The UUID is the identity —
  the graph `NodeIndex` is used internally but never crossed process
  boundaries.
- `Relationship` carries `from_table_id` / `to_table_id` (UUIDs),
  `from_column` / `to_column`, plus `RelationshipType` (OneToOne /
  OneToMany / ManyToMany) and a UUID of its own.
- Helpers live in `TableOps` / `RelationshipOps` traits.

The graph serialises to JSON for REST `/api/diagrams/*` payloads and to
CRDT snapshots for `/room/{id}`.

---

## 4. SQL parsing and generation

`src/core/sql_parser.rs` uses [`sqlparser`](https://crates.io/crates/sqlparser)
0.61 to parse user-edited SQL back into a `SchemaGraph`.

- `validate_sql(…)` → `SqlValidationResult` with span-aware errors
- `check_schema_sql(…)` → fast pre-save sanity check (used from
  `source_editor.rs`)
- `apply_sql_to_graph(…)` → merges parsed SQL into an existing graph,
  preserving UUIDs when table/column names match
- `validate_sql_with_graph(…)` → graph-aware validation (duplicate names,
  dangling FKs…)

Round-trip guarantee: editing SQL in the Source tab, then switching back
to Visual mode, must preserve IDs so that LiveShare peers don't see
"table renamed" for an unrelated edit.

SQL *output* is in `src/core/export.rs` (`SchemaExporter`) and supports
the dialects in `SqlDialect` (PostgreSQL / MySQL / SQLite).

---

## 5. Persistence

Repositories follow a consistent shape: `pub struct XRepository { pool: PgPool }`
with methods that return `Result<T, XRepositoryError>`.

| Repository           | Aggregate               | Interesting bits                            |
| -------------------- | ----------------------- | ------------------------------------------- |
| `UserRepository`     | `users`                 | bcrypt cost 12; `find_by_email` is unique   |
| `SessionRepository`  | `sessions`              | refresh-token rotation; hashed storage      |
| `DiagramRepository`  | `diagrams`              | `find_by_id`, `upsert`, `list_for_user`     |
| `FolderRepository`   | `folders`               | tree via `parent_id` self-FK                |
| `ShareRepository`    | `diagram_shares`        | permission levels: `viewer`, `editor`, `owner` |
| `LiveShareRepository`| `liveshare_sessions` + participants + snapshots | persists yrs state + participant history |

Migrations live in `migrations/`, timestamped. They are applied on
startup by `create_pool_with_migrations`. Do not retroactively mutate a
migration that has been applied to any environment; add a new one.

---

## 6. Authentication

Stateless JWT with a two-token scheme (see `src/core/auth/jwt.rs`):

- **Access token** — HS256, `exp` 15 min, carries `sub`, `email`, `username`.
  Used as `Authorization: Bearer …` on every API request.
- **Refresh token** — HS256, `exp` 7 days, `token_type: "refresh"`.
  Server also stores a hash in `sessions` for forced revocation.

Token TTL is configurable via `JWT_ACCESS_EXPIRATION_MINUTES` /
`JWT_REFRESH_EXPIRATION_DAYS`.

The LiveShare WS handshake reuses the access token (see
`src/core/liveshare/auth.rs`). When the access token expires mid-session
the socket is closed with code `4001`.

---

## 7. LiveShare (at a glance)

The full narrative is in `docs/liveshare/LIVESHARE_COMPLETE_GUIDE.md`.
Key points:

- One **Room** per active LiveShare session, keyed by UUID.
- Each connected client runs a per-socket actor (`websocket.rs`) that:
  1. Authenticates via JWT.
  2. Sends a snapshot to catch the new client up.
  3. Subscribes to the room's broadcast channel.
  4. Streams incremental `GraphOperation` events + volatile cursor/awareness.
- State is reconciled with a version-based last-write-wins strategy
  (`reconciliation.rs`); tombstones handle deletes.
- Periodic snapshots are written to `liveshare_snapshots` (last 10 kept,
  `snapshots.rs`). Crash recovery uses the newest snapshot.
- Throttling: cursor events capped at ~30 fps; schema updates at
  100–300 ms; awareness batched in 100 ms windows.
- Rate limiting at the socket level (`rate_limiter.rs`) protects against
  runaway clients.

---

## 8. AI assistant

- **Client UI**: `src/ui/ai_chat.rs` with a configurable mode (Write vs Ask).
- **Server proxy**: `src/core/ai_api.rs` exposes `/api/ai/chat`. The server
  holds the shared OpenRouter key (`OPENAPI_TOKEN` env var) and streams
  the response body back to the browser using an `mpsc::channel` → axum
  `Body::from_stream`.
- **Tool calls**: `src/core/ai_tools.rs` defines structured tools the LLM
  can invoke (add_table, add_column, add_relationship, …). These map 1:1
  to `GraphOperation`, so any AI-driven edit uses the same LiveShare
  machinery as a human click.

---

## 9. Frontend composition

Mount point: `app.rs::App`. Global contexts (provided once at the root):

1. **ThemeContext** — dark/light/auto, persisted in `localStorage`.
2. **AuthContext** — JWT tokens + user profile, refreshed on page load.
3. **LiveShareContext** — connection state, remote users, room info,
   graph-ops sender.
4. **ActivityTracker** — idle/away detection, feeds LiveShare awareness.

`src/ui/pages/editor.rs` is the main workspace and wires
`SchemaCanvas` + `Sidebar` + `SourceEditor` + `AiChatPanel` +
`LiveSharePanel` + `SettingsModal`. The canvas itself is in
`src/ui/canvas.rs`.

The editor UI is intentionally split by ownership:

- `src/ui/canvas.rs` owns the topbar, visual canvas shell, overlay wiring and
  Visual/Code mode composition.
- `src/ui/sidebar.rs` owns table search, stats, table list, column edit entry
  points and the new-table flow.
- `src/ui/table.rs` owns the draggable table card and compact one-line column
  rows.
- `src/ui/table_editor.rs` owns the fixed right-rail table inspector.
- `src/ui/column_editor.rs` owns the focused column form and FK controls.
- `src/ui/source_editor.rs` owns the SQL editor, diagnostics rail and source
  outline.

Frontend design rules, model boundaries for preview-only controls and CSS token
ownership are documented in [UI_DESIGN.md](./UI_DESIGN.md). Brand tokens are
documented in [`BRAND.md`](../BRAND.md) and applied through `style/input.css`.

---

## 10. Build pipeline

`cargo-leptos` drives the dual build:

- **Server bin** (`bin-features = ["ssr"]`) compiles native and links the
  SSR renderer.
- **Client lib** (`lib-features = ["hydrate"]`) compiles to
  `wasm32-unknown-unknown`, goes through `wasm-bindgen`, and is emitted
  into `target/site/pkg/archischema{.js,.wasm,.css}`.
- The `wasm-release` profile in `Cargo.toml` strips, LTOs, and ships a
  size-optimised WASM bundle.
- `task release` runs the release build and then the `compress` subtask,
  which brotli/gzip-compresses every `.wasm`/`.js`/`.css` so axum can
  serve pre-compressed assets via `ServeDir::precompressed_br` /
  `precompressed_gzip`.
