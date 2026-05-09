# ArchiSchema UI Design Notes

This document captures the current editor UI structure, design boundaries and
implementation rules. It complements [ARCHITECTURE.md](./ARCHITECTURE.md), which
describes system-level data flow.

## Design Sources

- Brand source: [`BRAND.md`](../BRAND.md).
- Applied token source: [`style/input.css`](../style/input.css).
- Generated CSS: `style/output.css`, built by `npm run build:css` only.
- Reference artboards: `Archischema(1)/DESIGNS/` and `Archischema(1)/styles/tokens.css`.

Do not introduce a second palette or typography system. Shared surfaces, cards,
buttons, inputs and canvas colors should use semantic CSS variables from
`style/input.css`.

## Editor Shell

The main editor workspace is composed in `src/ui/canvas.rs` and mounted from
`src/ui/pages/editor.rs`.

- Topbar owns global navigation: brand, breadcrumb back to `/dashboard`, diagram
  title, sync state, sharing, AI and user/settings actions.
- Sidebar owns mode switching, search, stats, table list and the `New table`
  entry point. It should not contain a separate `Back to Dashboard` panel.
- Visual mode renders the canvas and table cards.
- Code mode renders `SourceEditor` as an IDE-like layout with outline, wide code
  pane, diagnostics rail and footer status.
- AI chat, LiveShare panel and settings are overlays/docked panels, not schema
  model owners.

## Component Ownership

| UI area | File | Notes |
| ------- | ---- | ----- |
| Canvas table card | `src/ui/table.rs` | Uses one-line column rows with ellipsis and right-aligned type. |
| Sidebar table list | `src/ui/sidebar.rs` | Opens table/column editors and creates tables. |
| New table form | `src/ui/new_table_dialog.rs` | Wide dialog with schema/folder/description preview fields and preset columns. |
| Table inspector | `src/ui/table_editor.rs` | Fixed right rail; persists rename/delete only. |
| Column editor | `src/ui/column_editor.rs` | Focused form for name, type, constraints, default and FK relationship. |
| Source/Code editor | `src/ui/source_editor.rs` | SQL editor, validation diagnostics, Visual/Code switcher. |
| Shared primitives | `src/ui/common/*`, `style/input.css` | Buttons, dialogs, tokens and reusable surface classes. |

## Model Boundaries

The redesign intentionally does not expand the persisted schema model.

- Persisted today: table names, table positions, columns, basic constraints,
  defaults and relationships supported by `SchemaGraph`.
- Preview-only or disabled today: table schema, folder metadata, descriptions,
  indexes, history, SQL diff confirmation, column comments, `CHECK`, generated
  columns, FK `ON DELETE` / `ON UPDATE` actions.
- Do not change `TableNode`, `Column`, `Relationship`, `GraphOperation`, database
  migrations or saved diagram JSON just to satisfy visual-only controls.
- If a design-only field becomes a product requirement, add a separate model and
  migration plan first.

## Interaction Rules

- Interactive elements must be real `button` or `a` elements.
- Keep visible `focus-visible` states for buttons, links, inputs, table rows and
  segmented controls.
- Keep unsupported controls visible but disabled with a short `title` or helper
  text explaining the limitation.
- Use `Dialog` / `DialogWithHeader` from `src/ui/common/modal.rs` for modal
  surfaces. Use right rail patterns for table inspection.
- Use compact labels such as `PK`, `FK`, `NN`, `UQ` for constraint badges.
- Do not use `transition: all`; transition named properties only.

## Responsive Behavior

The desktop editor is the primary workflow. Current responsive behavior is a
safe fallback rather than a full mobile redesign.

- On narrow viewports, `editor-body` allows horizontal scrolling so the sidebar
  and canvas do not collapse into unusable widths.
- Source mode gives the code pane width priority; diagnostics are hidden on
  smaller breakpoints.
- Table inspector becomes full-width on small screens.
- A proper mobile-first editor layout should be planned separately if mobile
  editing becomes a product goal.

## Verification

Run these checks after UI/CSS changes:

```bash
npm run build:css
cargo check --features ssr
cargo check --no-default-features --features hydrate --target wasm32-unknown-unknown
```

For browser QA, load `http://127.0.0.1:3000/editor/demo` and verify:

- Visual/Code switch active state is obvious.
- Sidebar fields are not permanently highlighted when a table is expanded.
- Canvas and sidebar column rows stay on one line with ellipsis.
- New table dialog, table inspector and column editor fit their containers.
- Source/Code mode leaves the code pane wide enough to edit comfortably.
- Console has no runtime JavaScript errors.
