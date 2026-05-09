# AGENTS.md — Guide for AI coding agents working on ArchiSchema

This document tells AI agents (Claude Code, Codex, Cursor, etc.) what they
need to know to make correct, well-scoped changes to this repository.

The file is written in English so tools that don't speak Russian can still
follow it. **Human-facing answers to the repo owner should stay in Russian.**

---

## 1. What this project is

ArchiSchema is a browser-based **collaborative database schema diagram
editor**. A user draws tables and relationships on a canvas; the editor
generates and parses SQL, persists diagrams to Postgres, and — when the user
opens a "LiveShare" session — other clients edit the same diagram through a
WebSocket-backed CRDT layer.

**Stack (single Cargo workspace):**

| Layer      | Crate / tool                                             |
| ---------- | -------------------------------------------------------- |
| Frontend   | Leptos 0.8 (CSR + hydrate, WASM)                         |
| Backend    | Leptos-axum SSR, `axum` 0.8 on `tokio` 1                 |
| Persistence| `sqlx` 0.8 (Postgres 18, `tls-rustls-ring`)              |
| CRDT       | `yrs` 0.25 + `y-sync` 0.4 (Yjs-compatible)               |
| Auth       | Custom JWT (`jsonwebtoken` 10), bcrypt password hashing  |
| Styling    | Tailwind CSS 3.4 + ArchiSchema tokens in `style/input.css`|
| AI chat    | OpenRouter proxy (`reqwest` 0.13) in `src/core/ai_api.rs`|

Rust edition: **2024**. Rustc ≥ 1.94 is required.

---

## 2. Repository layout

```
src/
├── main.rs                 # axum server entry (ssr only), wires all routers
├── lib.rs                  # exports `app`, `core`, `ui`; hydrate entry point
├── app.rs                  # Leptos <App/> shell + Router routes
├── core/                   # domain & server code (most of it ssr-only)
│   ├── schema.rs           # TableNode / Relationship / SchemaGraph (petgraph)
│   ├── sql_parser.rs       # sqlparser-based SQL <-> graph conversion
│   ├── validation.rs       # identifier/shape validation
│   ├── export.rs           # export to SQL / JSON
│   ├── auto_layout.rs      # hierarchical layout algorithm
│   ├── ai_api.rs           # /api/ai/chat proxy (server, streams responses)
│   ├── ai_config.rs        # client-facing AI config (modes, prompts)
│   ├── ai_tools.rs         # structured tool calls for the AI agent
│   ├── auth/               # JWT service, AuthService, /api/auth/* router
│   ├── db/                 # sqlx pool + repositories (user, session, diagram,
│   │                        folder, share, liveshare)
│   ├── diagrams/           # /api/diagrams/* router
│   ├── folders/            # /api/folders/* router
│   ├── sharing/            # /api/diagrams/{id}/shares/* router
│   └── liveshare/          # see §4
├── ui/                     # Leptos components (csr + hydrate)
│   ├── pages/              # LandingPage, LoginPage, RegisterPage, DashboardPage,
│   │                        ProfilePage, EditorPage, NotFoundPage
│   ├── common/             # Button, Dialog, modal, form, dropdown, badge…
│   ├── auth/               # AuthContext + LoginForm / RegisterForm / UserMenu
│   ├── canvas.rs           # main diagram canvas
│   ├── sidebar.rs          # table list + search
│   ├── table.rs, table_editor.rs, new_table_dialog.rs, column_editor.rs
│   ├── source_editor.rs    # SQL text mode
│   ├── liveshare_client.rs # client-side WebSocket + LiveShareContext
│   ├── liveshare_panel.rs, settings_modal.rs, remote_cursors.rs
│   ├── activity_tracker.rs # mouse/keyboard idle detection
│   ├── notifications.rs, sync_status_indicator.rs, icon.rs, theme.rs, markdown.rs
│   └── graph_ops.rs        # GraphOperation apply helpers
migrations/                 # sqlx migrations (timestamped .sql)
public/                     # static assets served from /
style/                      # Tailwind input; compiled to style/output.css
docs/                       # long-form docs (see §7)
Taskfile.yml                # task runner entrypoints (dev/build/test/check…)
docker-compose.yaml         # postgres:18.1 on :5432
```

---

## 3. Feature flags

**Only two features matter:**

- `ssr` — enables server crates (axum, sqlx, tokio, reqwest, jsonwebtoken…).
  All server-only modules are gated with `#[cfg(feature = "ssr")]`.
- `hydrate` — enables WASM/DOM crates (web-sys, wasm-bindgen, gloo-timers…).
  Client-only modules use `#[cfg(not(feature = "ssr"))]` (i.e. "not server").

`cargo-leptos` builds the bin with `ssr` and the cdylib with `hydrate`.
You should *never* need `--all-features` when iterating — use one or the
other.

**Canonical commands (run from repo root):**

```bash
cargo check --features ssr                                    # server
cargo check --no-default-features --features hydrate --target wasm32-unknown-unknown
cargo clippy --features ssr
cargo test --features ssr                                     # see §8
task dev                                                      # full dev loop
```

---

## 4. LiveShare subsystem

LiveShare is the collaborative editing layer. It is the largest, most
delicate part of the codebase — ≈14k LOC under `src/core/liveshare/` and
`src/ui/liveshare_*`. Full design is in `docs/liveshare/LIVESHARE_COMPLETE_GUIDE.md`.

**Key modules:**

| File                          | Role                                                     |
| ----------------------------- | -------------------------------------------------------- |
| `core/liveshare/protocol.rs`  | WS message enums, DTOs, `GraphOperation`, `AwarenessState` |
| `core/liveshare/websocket.rs` | Per-connection actor: auth, broadcast, snapshot restore  |
| `core/liveshare/room.rs`      | `Room` state, `RoomManager`, password verification       |
| `core/liveshare/api.rs`       | REST: `POST /room`, `GET /room/{id}`, `DELETE /room/{id}`|
| `core/liveshare/auth.rs`      | JWT-based WS auth                                        |
| `core/liveshare/broadcast_manager.rs` | Per-user version tracking for incremental sync  |
| `core/liveshare/cursor_broadcaster.rs`| Volatile cursor channel (packet loss OK)        |
| `core/liveshare/throttling.rs`| Rate limiting for cursor (33ms) & schema (100-300ms)     |
| `core/liveshare/reconciliation.rs`| last-write-wins merge with tombstones                |
| `core/liveshare/snapshots.rs` | Periodic yrs state snapshots to Postgres                 |
| `core/liveshare/rate_limiter.rs`| Per-connection message-rate guard                      |
| `core/liveshare/idle_detection.rs`| Server-side idle/away tracking                       |
| `core/liveshare/load_test.rs` / `load_test_integration.rs` | bench harness |
| `ui/liveshare_client.rs`      | Client `LiveShareContext` + `RemoteUser` state           |

**Message types:** see `ClientMessage` / `ServerMessage` enums in
`protocol.rs`. Cursor / idle updates go on a **volatile broadcast channel**
— losing packets is acceptable. Schema updates (`GraphOperation`) go on a
reliable channel and carry a monotonic `version` per element (UUID-keyed).

**Stable IDs:** tables and relationships are identified by `Uuid`. Do not
invent a new "node index" identifier; always thread the UUID through.

---

## 5. Routing, auth, and API surface

HTTP routers are defined per subsystem and merged in `main.rs`:

| Path prefix                       | Owner module          | Auth          |
| --------------------------------- | --------------------- | ------------- |
| `/` (everything else)             | Leptos SSR            | varies        |
| `/room/{room_id}` (GET, WS)       | `core/liveshare/*`    | JWT via query |
| `/api/ai/chat`                    | `core/ai_api.rs`      | none (proxy)  |
| `/api/auth/*`                     | `core/auth/api.rs`    | public        |
| `/api/diagrams/*`                 | `core/diagrams/api.rs`| JWT Bearer    |
| `/api/folders/*`                  | `core/folders/api.rs` | JWT Bearer    |
| `/api/diagrams/{id}/shares/*`     | `core/sharing/api.rs` | JWT Bearer    |

Access tokens live 15 minutes, refresh tokens 7 days (see
`src/core/auth/jwt.rs`).

---

## 6. Database and migrations

Postgres 18, `sqlx` with `runtime-tokio` + `tls-rustls-ring`. All
repositories live in `src/core/db/repositories/` and expose thin methods
over `PgPool`.

**Migration rules:**

- Migrations are timestamped SQL files in `migrations/`. They are applied
  automatically from `create_pool_with_migrations` at server startup.
- Never retroactively edit a migration that has been applied to any
  environment. Add a new timestamped file instead.
- The `pgcrypto` extension must be available (enabled in the initial
  migration via `CREATE EXTENSION IF NOT EXISTS "pgcrypto"`).

`docker-compose up -d` launches `postgres:18.1` on `localhost:5432` with
`postgres/postgres` credentials and database `archischema`, which matches
the `DATABASE_URL` in `.env.example`.

---

## 7. Documentation map

| File                                                | Contents                               |
| --------------------------------------------------- | -------------------------------------- |
| `README.md`                                         | Quick start, dev workflow              |
| `AGENTS.md` (this file)                             | What agents need to know               |
| `BRAND.md`                                          | Brand palette, typography, voice       |
| `docs/ARCHITECTURE.md`                              | High-level architecture + diagrams     |
| `docs/UI_DESIGN.md`                                 | Editor UI rules and redesign boundaries|
| `docs/liveshare/LIVESHARE_COMPLETE_GUIDE.md`        | Deep dive on LiveShare (phases 1–13)   |
| `TODO.md`                                           | Living roadmap, in Russian             |

When you add long-form documentation, prefer Markdown inside `docs/` and
link it from this section and from `README.md`.

---

## 8. Tests

There are two kinds of tests:

1. **Unit tests** — under `#[cfg(test)]` next to the code. Run with
   `cargo test --features ssr`. Safe to run anywhere.
2. **Integration tests** — marked `#[ignore]`. They require a live
   PostgreSQL. There are ~56 of them, gated because CI doesn't yet have a
   dedicated DB (see TODO item #9). To run locally:

   ```bash
   docker-compose up -d
   DATABASE_URL=postgres://postgres:postgres@localhost:5432/archischema \
     cargo test --features ssr -- --ignored
   ```

Load tests under `src/core/liveshare/load_test*.rs` emit `println!`
metrics on purpose — don't "clean them up" to `tracing`.

---

## 9. House rules for agents

**Do:**

- Preserve feature-gating. If you use a server crate, gate the import and
  usage with `#[cfg(feature = "ssr")]`.
- Keep stable IDs (UUIDs) for schema entities — never reintroduce index-based
  identity for tables/relationships.
- Prefer `cargo check --features ssr` for fast feedback; run the wasm check
  only when you've touched client-only code.
- Use `try_init` for global state like tracing-subscriber — tests may have
  already initialised it.
- For HTML dialogs, use the `Dialog` / `DialogWithHeader` components in
  `ui/common/modal.rs`. The older `BaseModal` path is being phased out.

**Don't:**

- Don't invent new env-var names. The existing `OPENAPI_BASE` /
  `OPENAPI_TOKEN` are confusingly named (they talk to OpenRouter, not
  OpenAPI) — but renaming them is a breaking change for deployed `.env`
  files, so don't touch without explicit ask.
- Don't drop `#[cfg(test)]`-gated `#[ignore]` attributes on the DB tests
  unless you've also provisioned a CI database.
- Don't rewrite `protocol.rs` enum variants. The WebSocket protocol is
  versioned implicitly by struct shape; old clients will break.
- Don't run `cargo update` without asking — dep bumps are tracked deliberately.
- Don't commit a real API key. `.env` is gitignored, `.env.example` must
  contain placeholders only.

---

## 10. Known open issues (as of 2026-04-24)

Cross-reference `TODO.md` for the authoritative list. Short version:

- **2.3** — diagram name sync on join via password-protected session needs
  verification. Debug logs were removed in this audit; reproduce before
  adding more instrumentation.
- **6** — migrate remaining dialogs (delete confirm, settings, LiveShare
  connect, error) to HTML `<dialog>`.
- **7** — move Visual ↔ Source toggle from sidebar onto the canvas.
- **8** — no auto-refresh of access tokens; session silently expires.
- **9** — no CI job for the `#[ignore]`-gated integration tests.
- Several `TODO` markers in `ui/liveshare_client.rs` (lines 776, 867, 871,
  906) relate to unused Yjs-native update paths — current sync relies on
  `GraphOperation`, not raw yrs diffs. Decide before refactoring.
