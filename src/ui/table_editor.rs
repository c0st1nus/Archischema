use crate::core::{SchemaGraph, TableOps};
use crate::ui::liveshare_client::{ConnectionState, GraphOperation, use_liveshare_context};
use crate::ui::{ErrorMessage, Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;
use petgraph::graph::NodeIndex;

#[cfg(not(feature = "ssr"))]
fn dispatch_save_event(reason: &str) {
    use wasm_bindgen::JsValue;

    if let Some(window) = web_sys::window() {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(&JsValue::from_str(reason));
        if let Ok(event) =
            web_sys::CustomEvent::new_with_event_init_dict("diagram-save-requested", &init)
        {
            let _ = window.dispatch_event(&event);
        }
    }
}

#[cfg(feature = "ssr")]
fn dispatch_save_event(_reason: &str) {
    // No-op on server
}

#[component]
pub fn TableEditor(
    /// Whether dialog is open
    is_open: Signal<bool>,
    graph: RwSignal<SchemaGraph>,
    node_idx: NodeIndex,
    #[prop(into)] on_save: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(into)] on_delete: Callback<()>,
    #[prop(into)] on_add_column: Callback<NodeIndex>,
    #[prop(into)] on_edit_column: Callback<usize>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    // Получаем текущее имя таблицы (untracked - это начальное значение)
    let initial_name = graph.with_untracked(|g| {
        g.node_weight(node_idx)
            .map(|n| n.name.clone())
            .unwrap_or_default()
    });

    let (table_name, set_table_name) = signal(initial_name.clone());
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_saving, set_is_saving) = signal(false);

    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Auto-focus на input при открытии диалога
    Effect::new(move || {
        if is_open.get() {
            // Reset form when dialog opens
            set_table_name.set(graph.with(|g| {
                g.node_weight(node_idx)
                    .map(|n| n.name.clone())
                    .unwrap_or_default()
            }));
            set_error.set(None);
            set_is_saving.set(false);

            if let Some(input) = input_ref.get() {
                let _ = input.focus();
                input.select();
            }
        }
    });

    let handle_save = move || {
        let name = table_name.get().trim().to_string();

        // Валидация
        if name.is_empty() {
            set_error.set(Some("Table name cannot be empty".to_string()));
            return;
        }

        set_is_saving.set(true);
        set_error.set(None);

        // Попытка переименовать таблицу
        match graph.write().rename_table(node_idx, name.clone()) {
            Ok(()) => {
                // Send sync op
                if liveshare_ctx.connection_state.with_untracked(|v| *v)
                    == ConnectionState::Connected
                {
                    let table_uuid = graph.with(|g| {
                        g.node_weight(node_idx)
                            .map(|n| n.uuid)
                            .unwrap_or_else(uuid::Uuid::new_v4)
                    });
                    liveshare_ctx.send_graph_op(GraphOperation::RenameTable {
                        node_id: node_idx.index() as u32,
                        table_uuid,
                        new_name: name,
                    });
                }
                set_is_saving.set(false);

                // Trigger save after table rename
                dispatch_save_event("table_renamed");

                on_save.run(());
            }
            Err(err) => {
                set_is_saving.set(false);
                set_error.set(Some(err));
            }
        }
    };

    let handle_cancel = move || {
        set_error.set(None);
        set_is_saving.set(false);
        on_cancel.run(());
    };

    let handle_delete = move || {
        on_delete.run(());
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            ev.prevent_default();
            handle_save();
        }
    };

    let column_count = move || {
        graph.with(|g| {
            g.node_weight(node_idx)
                .map(|n| n.columns.len())
                .unwrap_or(0)
        })
    };
    let relation_count = move || {
        graph.with(|g| {
            g.edges(node_idx).count()
                + g.edges_directed(node_idx, petgraph::Direction::Incoming)
                    .count()
        })
    };
    let table_columns = move || {
        graph.with(|g| {
            g.node_weight(node_idx)
                .map(|n| n.columns.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <aside class=move || if is_open.get() { "table-inspector-rail" } else { "table-inspector-rail hidden" }>
            <div class="table-inspector-head">
                <div class="flex min-w-0 items-center gap-3">
                    <Icon name=icons::DATABASE class="h-4 w-4 text-theme-accent" />
                    <div class="min-w-0">
                        <div class="flex min-w-0 items-center gap-2">
                            <h3 class="title-lg truncate">{move || table_name.get()}</h3>
                            <span class="chip h-[22px] text-[11px]">{move || format!("{} cols · {} rels", column_count(), relation_count())}</span>
                        </div>
                        <p class="subtitle">"Right-rail inspector for the selected table."</p>
                    </div>
                </div>
                <button type="button" class="btn-icon" title="Close inspector" on:click=move |_| handle_cancel()>
                    <Icon name=icons::X class="icon-standalone" />
                </button>
            </div>

            <div class="border-b border-theme bg-theme-secondary px-4 py-3">
                <div class="tabs-list tabs-full-width">
                    <button type="button" class="tab-item tab-active"><Icon name=icons::DATABASE class="h-3.5 w-3.5" />"Schema"</button>
                    <button type="button" class="tab-item tab-disabled" disabled=true title="Relationship editing stays on the canvas for now">"Relations"</button>
                    <button type="button" class="tab-item tab-disabled" disabled=true title="Indexes are a preview-only concept in this pass">"Indexes"</button>
                    <button type="button" class="tab-item tab-disabled" disabled=true title="History is not persisted yet">"History"</button>
                </div>
            </div>

            <div class="table-inspector-body scroll">
                <div class="grid gap-4 sm:grid-cols-2">
                    <label class="block sm:col-span-1">
                        <span class="field-label">"Name" <span class="text-theme-error">"*"</span></span>
                        <input
                            node_ref=input_ref
                            type="text"
                            autocomplete="off"
                            spellcheck="false"
                            class="input-base mt-1"
                            placeholder="orders"
                            prop:value=move || table_name.get()
                            on:input=move |ev| {
                                set_table_name.set(event_target_value(&ev));
                                set_error.set(None);
                            }
                            on:keydown=handle_keydown
                            disabled=move || is_saving.get()
                        />
                    </label>

                    <label class="block sm:col-span-1">
                        <span class="field-label">"Schema"</span>
                        <input type="text" class="input-base mt-1" value="public" disabled=true />
                    </label>

                    <label class="block sm:col-span-2">
                        <span class="field-label">"Description"</span>
                        <textarea
                            class="input-base mt-1 min-h-[76px] resize-none"
                            disabled=true
                            placeholder="Description metadata is preview-only in this pass."
                        ></textarea>
                        <span class="field-help">"Only table name is persisted by the current graph model."</span>
                        <ErrorMessage error=error/>
                    </label>
                </div>

                <section class="surface overflow-hidden">
                    <div class="form-section-head">
                        <span class="eyebrow">"Columns · " {move || column_count()}</span>
                        <button
                            type="button"
                            class="btn-ghost btn-sm"
                            title="Add column"
                            on:click=move |_| on_add_column.run(node_idx)
                        >
                            <Icon name=icons::PLUS class="h-3 w-3" />
                            "Add"
                        </button>
                    </div>
                    <div class="max-h-[280px] overflow-y-auto scroll">
                        {move || {
                            let columns = table_columns();
                            if columns.is_empty() {
                                view! {
                                    <div class="rounded-md border border-dashed border-theme p-5 text-center text-xs text-theme-muted">
                                        "No columns yet. Add the first column from the table list."
                                    </div>
                                }.into_any()
                            } else {
                                columns.into_iter().enumerate().map(|(column_idx, column)| {
                                    let name = column.name.clone();
                                    let data_type = column.data_type.clone();
                                    view! {
                                        <div class="inspector-column-row">
                                            <div class="column-inline-main">
                                                <span class="column-inline-badges">
                                                    {if column.is_primary_key {
                                                        view! { <span class="schema-table-badge schema-table-badge-pk">"PK"</span> }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }}
                                                </span>
                                                <span class="column-inline-name" title=name.clone()>{name.clone()}</span>
                                            </div>
                                            <span class="column-inline-type" title=data_type.clone()>{data_type.clone()}</span>
                                            <span class="column-inline-badges justify-end">
                                                {if !column.is_nullable {
                                                    view! { <span class="schema-table-badge schema-table-badge-nn">"NN"</span> }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                                {if column.is_unique {
                                                    view! { <span class="schema-table-badge schema-table-badge-uq">"UQ"</span> }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </span>
                                            <button
                                                type="button"
                                                class="btn-icon btn-sm"
                                                title="Edit column"
                                                on:click=move |_| on_edit_column.run(column_idx)
                                            >
                                                <Icon name=icons::EDIT class="h-3.5 w-3.5" />
                                            </button>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </div>
                </section>

                <section class="surface overflow-hidden">
                    <div class="form-section-head">
                        <span class="eyebrow">"Indexes · 2"</span>
                        <button type="button" class="btn-ghost btn-sm" disabled=true><Icon name=icons::PLUS class="h-3 w-3" />"Add"</button>
                    </div>
                    <div class="space-y-2 p-3">
                        <div class="index-preview-row"><Icon name=icons::CODE class="h-3.5 w-3.5 text-theme-accent" /><span class="truncate mono">{move || format!("{}_pkey", table_name.get())}</span><span class="chip h-[18px] text-[10px]">"unique"</span></div>
                        <div class="index-preview-row"><Icon name=icons::CODE class="h-3.5 w-3.5 text-theme-accent" /><span class="truncate mono">"idx_relationships"</span><span class="chip h-[18px] text-[10px]">"preview"</span></div>
                    </div>
                </section>

                <section class="danger-zone-card">
                    <div class="mb-2 flex items-center gap-2 font-semibold text-theme-error">
                        <Icon name=icons::WARNING class="h-4 w-4" />
                        "Danger zone"
                    </div>
                    <p class="mb-3 text-xs text-theme-muted">"Drops this table from the diagram. Existing relationships will be detached."</p>
                    <button class="btn-danger" on:click=move |_| handle_delete() disabled=move || is_saving.get()>
                        <Icon name=icons::TRASH class="h-3.5 w-3.5"/>
                        "Delete table"
                    </button>
                </section>
            </div>

            <div class="table-inspector-footer">
                <span class="text-xs text-theme-muted"><span class="text-theme-accent">"✓"</span> " Saved locally"</span>
                <div class="flex items-center gap-2">
                    <button type="button" class="btn-secondary btn-sm" disabled=true title="SQL diff preview is planned for a later model pass">
                        <Icon name=icons::CODE class="h-3 w-3" />
                        "View SQL diff"
                    </button>
                    <button
                        class="btn-primary btn-sm"
                        on:click=move |_| handle_save()
                        disabled=move || is_saving.get() || table_name.get().trim().is_empty()
                    >
                        {move || if is_saving.get() {
                            view! { <Icon name=icons::LOADER class="h-3 w-3 animate-spin"/> }.into_any()
                        } else {
                            view! { <Icon name=icons::CHECK class="h-3 w-3"/> }.into_any()
                        }}
                        "Save"
                    </button>
                </div>
            </div>
        </aside>
    }
}
