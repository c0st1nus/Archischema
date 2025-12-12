//! SQL Source Editor component
//!
//! Provides a text editor for viewing and editing the database schema as SQL DDL statements.
//! This is an alternative view to the visual canvas editor.

use crate::core::{ExportOptions, SchemaExporter, SchemaGraph, SqlDialect};
use leptos::prelude::*;
use leptos::web_sys;

/// View mode for the schema editor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Visual canvas editor (default)
    #[default]
    Visual,
    /// SQL source code editor
    Source,
}

/// SQL Source Editor component
#[component]
pub fn SourceEditor(
    /// The schema graph to display/edit
    graph: RwSignal<SchemaGraph>,
    /// Whether the editor is in read-only mode
    #[prop(default = false)]
    readonly: bool,
) -> impl IntoView {
    // SQL content derived from graph
    let sql_content = Memo::new(move |_| {
        graph.with(|g| {
            let options = ExportOptions {
                sql_dialect: SqlDialect::MySQL,
                include_positions: true,
                include_drop_statements: false,
                pretty_print: true,
                ..Default::default()
            };
            SchemaExporter::export_sql(g, &options).unwrap_or_else(|e| format!("-- Error: {}", e))
        })
    });

    // Line numbers derived from content
    let line_count = Memo::new(move |_| sql_content.with(|s| s.lines().count().max(1)));

    // Local editable content (for non-readonly mode)
    let (local_content, set_local_content) = signal(String::new());
    let (is_modified, set_is_modified) = signal(false);
    let (parse_error, set_parse_error) = signal::<Option<String>>(None);

    // Sync local content when graph changes (only if not modified)
    Effect::new(move |_| {
        if !is_modified.get() {
            set_local_content.set(sql_content.get());
        }
    });

    // Display content - either local (if modified) or from graph
    let display_content = Memo::new(move |_| {
        if is_modified.get() {
            local_content.get()
        } else {
            sql_content.get()
        }
    });

    // Handle text input changes
    let on_input = move |ev: leptos::ev::Event| {
        use leptos::wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let textarea = target.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        let value = textarea.value();
        set_local_content.set(value);
        set_is_modified.set(true);
        set_parse_error.set(None);
    };

    // Reset to graph state
    let reset_changes = move |_: leptos::ev::MouseEvent| {
        set_local_content.set(sql_content.get());
        set_is_modified.set(false);
        set_parse_error.set(None);
    };

    view! {
        <div class="h-full flex flex-col bg-theme-primary theme-transition">
            // Toolbar
            <div class="flex items-center justify-between px-4 py-2 bg-theme-secondary border-b border-theme-primary theme-transition">
                <div class="flex items-center gap-2">
                    <span class="text-theme-secondary text-sm font-medium">"SQL Source"</span>
                    <span class="text-theme-muted text-xs">
                        {move || format!("{} lines", line_count.get())}
                    </span>
                    {move || {
                        if is_modified.get() {
                            view! {
                                <span class="px-2 py-0.5 text-xs font-medium rounded-full bg-yellow-500/20 text-yellow-400">
                                    "Modified"
                                </span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                // Action buttons (only show if not readonly and modified)
                {move || {
                    if !readonly && is_modified.get() {
                        view! {
                            <div class="flex items-center gap-2">
                                <button
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-theme-tertiary text-theme-secondary hover:bg-theme-primary transition-colors"
                                    on:click=reset_changes
                                >
                                    "Reset"
                                </button>
                                // Apply button disabled for now
                                // <button
                                //     class="px-3 py-1.5 text-xs font-medium rounded-lg bg-accent-primary text-white hover:bg-accent-secondary transition-colors"
                                //     on:click=apply_changes
                                // >
                                //     "Apply"
                                // </button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            // Error message
            {move || {
                parse_error.get().map(|err| view! {
                    <div class="px-4 py-2 bg-red-500/20 border-b border-red-500/30 text-red-400 text-sm">
                        {err}
                    </div>
                })
            }}

            // Editor area
            <div class="flex-1 flex overflow-hidden">
                // Line numbers
                <div class="flex-shrink-0 w-12 bg-theme-secondary border-r border-theme-primary overflow-hidden theme-transition">
                    <div class="py-3 px-2 text-right font-mono text-xs text-theme-muted select-none" style="line-height: 1.5rem;">
                        {move || {
                            (1..=line_count.get())
                                .map(|n| view! { <div>{n}</div> })
                                .collect_view()
                        }}
                    </div>
                </div>

                // Text area
                <div class="flex-1 overflow-auto bg-theme-primary">
                    {move || {
                        if readonly {
                            // Read-only: use pre element
                            view! {
                                <pre class="p-3 font-mono text-sm text-theme-primary whitespace-pre overflow-x-auto" style="line-height: 1.5rem; tab-size: 4;">
                                    {display_content.get()}
                                </pre>
                            }.into_any()
                        } else {
                            // Editable: use textarea
                            view! {
                                <textarea
                                    class="w-full h-full p-3 font-mono text-sm text-theme-primary bg-transparent resize-none outline-none"
                                    style="line-height: 1.5rem; tab-size: 4;"
                                    spellcheck="false"
                                    prop:value=move || display_content.get()
                                    on:input=on_input
                                />
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Footer with help text
            <div class="px-4 py-2 bg-theme-secondary border-t border-theme-primary text-theme-muted text-xs theme-transition">
                {move || {
                    if readonly {
                        "Read-only view. Switch to Visual mode to edit."
                    } else if is_modified.get() {
                        "Changes are local only. SQL parsing is not yet implemented."
                    } else {
                        "SQL DDL representation of your schema. Editing will be available in a future update."
                    }
                }}
            </div>
        </div>
    }
}

/// Mode switcher component for sidebar
#[component]
pub fn EditorModeSwitcher(
    /// Current editor mode
    mode: RwSignal<EditorMode>,
) -> impl IntoView {
    view! {
        <div class="flex items-center bg-theme-tertiary rounded-lg p-1 theme-transition">
            // Visual mode button
            <button
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all"
                style=move || {
                    if mode.get() == EditorMode::Visual {
                        "background-color: var(--bg-surface); color: var(--text-primary); box-shadow: var(--shadow-sm);"
                    } else {
                        "background-color: transparent; color: var(--text-tertiary);"
                    }
                }
                on:click=move |_| mode.set(EditorMode::Visual)
            >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
                </svg>
                "Visual"
            </button>

            // Source mode button
            <button
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all"
                style=move || {
                    if mode.get() == EditorMode::Source {
                        "background-color: var(--bg-surface); color: var(--text-primary); box-shadow: var(--shadow-sm);"
                    } else {
                        "background-color: transparent; color: var(--text-tertiary);"
                    }
                }
                on:click=move |_| mode.set(EditorMode::Source)
            >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                </svg>
                "Source"
            </button>
        </div>
    }
}
