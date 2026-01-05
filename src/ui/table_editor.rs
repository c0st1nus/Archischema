use crate::core::{SchemaGraph, TableOps};
use crate::ui::liveshare_client::{ConnectionState, GraphOperation, use_liveshare_context};
use crate::ui::{Dialog, ErrorMessage, Icon, SaveCancelHints, icons};
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

    let handle_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            handle_save();
        }
        _ => {}
    };

    view! {
        <Dialog
            is_open=is_open
            on_close=Callback::new(move |_| handle_cancel())
            max_width="max-w-lg"
            close_on_backdrop=true
        >
            <div class="p-6">
                // Заголовок
                <div class="mb-6">
                    <h3 class="text-2xl font-bold text-theme-primary mb-2">"Edit Table"</h3>
                    <p class="text-sm text-theme-muted">
                        "Rename your table or manage its properties"
                    </p>
                </div>

                // Форма
                <div class="space-y-5">
                    // Поле имени таблицы
                    <div>
                        <label class="block text-sm font-medium text-theme-primary mb-2">
                            "Table Name"
                            <span class="text-red-500 ml-1">"*"</span>
                        </label>
                        <input
                            node_ref=input_ref
                            type="text"
                            class="w-full px-4 py-2.5 bg-theme-surface border border-theme-primary rounded-lg text-theme-primary placeholder-theme-muted focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent transition-all"
                            placeholder="Enter table name"
                            prop:value=move || table_name.get()
                            on:input=move |ev| {
                                set_table_name.set(event_target_value(&ev));
                                set_error.set(None);
                            }
                            on:keydown=handle_keydown
                            disabled=move || is_saving.get()
                        />

                        <ErrorMessage error=error/>
                    </div>

                    // Информация о таблице
                    <div class="bg-theme-tertiary border border-theme-primary rounded-lg p-4 space-y-3">
                        <div class="flex items-center text-sm font-semibold text-theme-primary mb-2">
                            <Icon name=icons::INFORMATION_CIRCLE class="w-4 h-4 mr-2 text-blue-500"/>
                            "Table Information"
                        </div>
                        <div class="flex items-center justify-between text-sm">
                            <span class="text-theme-muted">"Columns:"</span>
                            <span class="font-medium text-theme-primary">
                                {move || {
                                    graph
                                        .with(|g| {
                                            g.node_weight(node_idx).map(|n| n.columns.len()).unwrap_or(0)
                                        })
                                }}

                            </span>
                        </div>
                        <div class="flex items-center justify-between text-sm">
                            <span class="text-theme-muted">"Relationships:"</span>
                            <span class="font-medium text-theme-primary">
                                {move || {
                                    graph
                                        .with(|g| {
                                            g.edges(node_idx).count() + g.edges_directed(node_idx, petgraph::Direction::Incoming).count()
                                        })
                                }}

                            </span>
                        </div>
                    </div>
                </div>

                // Кнопки действий
                <div class="flex items-center justify-between mt-6 pt-5 border-t border-theme-primary">
                    <button
                        class="px-4 py-2.5 text-sm font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-all focus:outline-none focus:ring-2 focus:ring-red-500 flex items-center"
                        on:click=move |_| handle_delete()
                        disabled=move || is_saving.get()
                    >
                        <Icon name=icons::TRASH class="w-4 h-4 mr-2"/>
                        "Delete Table"
                    </button>

                    <div class="flex items-center space-x-3">
                        <button
                            class="px-5 py-2.5 text-sm font-medium text-theme-secondary bg-theme-secondary hover:bg-theme-tertiary border border-theme-primary rounded-lg transition-all focus:outline-none focus:ring-2 focus:ring-theme-accent"
                            on:click=move |_| handle_cancel()
                            disabled=move || is_saving.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-6 py-2.5 text-sm font-semibold text-white bg-gradient-to-r from-accent-primary to-accent-secondary hover:shadow-lg disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-all focus:outline-none focus:ring-2 focus:ring-accent-primary flex items-center"
                            on:click=move |_| handle_save()
                            disabled=move || is_saving.get() || table_name.get().trim().is_empty()
                        >
                            {move || {
                                if is_saving.get() {
                                    view! {
                                        <>
                                            <Icon name=icons::LOADER class="w-4 h-4 mr-2 animate-spin"/>
                                            "Saving..."
                                        </>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <>
                                            <Icon name=icons::CHECK class="w-4 h-4 mr-2"/>
                                            "Save Changes"
                                        </>
                                    }
                                        .into_any()
                                }
                            }}

                        </button>
                    </div>
                </div>

                // Подсказка по горячим клавишам
                <div class="mt-4 pt-4 border-t border-theme-primary/50">
                    <SaveCancelHints/>
                </div>
            </div>
        </Dialog>
    }
}
