use crate::core::{SchemaGraph, TableOps};
use crate::ui::liveshare_client::{ConnectionState, GraphOperation, use_liveshare_context};
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;
use petgraph::graph::NodeIndex;

#[component]
pub fn TableEditor(
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

    let (table_name, set_table_name) = signal(initial_name);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_saving, set_is_saving) = signal(false);

    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Auto-focus на input при монтировании
    Effect::new(move || {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
            input.select();
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
                if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
                    liveshare_ctx.send_graph_op(GraphOperation::RenameTable {
                        node_id: node_idx.index() as u32,
                        new_name: name,
                    });
                }
                set_is_saving.set(false);
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
        "Escape" => {
            ev.prevent_default();
            handle_cancel();
        }
        _ => {}
    };

    view! {
        <div class="space-y-4">
            // Заголовок
            <div>
                <h3 class="text-lg font-semibold text-theme-primary">"Edit Table"</h3>
                <p class="text-sm text-theme-muted">
                    "Rename your table or manage its properties"
                </p>
            </div>

            // Форма
            <div class="space-y-4">
                // Поле имени таблицы
                <div>
                    <label class="block text-sm font-medium text-theme-secondary mb-2">
                        "Table Name"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <input
                        node_ref=input_ref
                        type="text"
                        class="w-full px-4 py-2.5 input-theme rounded-lg transition-all"
                        placeholder="Enter table name"
                        prop:value=move || table_name.get()
                        on:input=move |ev| {
                            set_table_name.set(event_target_value(&ev));
                            set_error.set(None);
                        }
                        on:keydown=handle_keydown
                        disabled=move || is_saving.get()
                    />

                    {move || {
                        error
                            .get()
                            .map(|err| {
                                view! {
                                    <div class="mt-2 flex items-center text-sm text-theme-error">
                                        <Icon name=icons::ALERT_CIRCLE class="w-4 h-4 mr-1.5"/>
                                        <span>{err}</span>
                                    </div>
                                }
                            })
                    }}
                </div>

                // Информация о таблице
                <div class="p-4 bg-theme-secondary rounded-lg space-y-2 theme-transition">
                    <div class="flex items-center justify-between text-sm">
                        <span class="text-theme-tertiary">"Columns:"</span>
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
                        <span class="text-theme-tertiary">"Relationships:"</span>
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
            <div class="flex items-center justify-between pt-4 border-t border-theme-primary">
                <button
                    class="px-4 py-2 text-sm font-medium text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors flex items-center"
                    on:click=move |_| handle_delete()
                    disabled=move || is_saving.get()
                >
                    <Icon name=icons::TRASH class="w-4 h-4 mr-1.5"/>
                    "Delete Table"
                </button>

                <div class="flex items-center space-x-2">
                    <button
                        class="px-4 py-2 text-sm font-medium text-theme-secondary hover:bg-theme-tertiary rounded-lg transition-colors"
                        on:click=move |_| handle_cancel()
                        disabled=move || is_saving.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-6 py-2 text-sm font-medium btn-theme-primary rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
                        on:click=move |_| handle_save()
                        disabled=move || is_saving.get() || table_name.get().trim().is_empty()
                    >
                        {move || {
                            if is_saving.get() {
                                view! {
                                    <>
                                        <Icon name=icons::LOADER class="w-4 h-4 mr-1.5 animate-spin"/>
                                        "Saving..."
                                    </>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <>
                                        <Icon name=icons::CHECK class="w-4 h-4 mr-1.5"/>
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
            <div class="pt-2 border-t border-theme-primary">
                <div class="flex items-center justify-center space-x-4 text-xs text-theme-muted">
                    <div class="flex items-center">
                        <kbd class="px-2 py-1 bg-theme-tertiary rounded border border-theme-secondary font-mono text-theme-secondary">
                            "Enter"
                        </kbd>
                        <span class="ml-1">"to save"</span>
                    </div>
                    <div class="flex items-center">
                        <kbd class="px-2 py-1 bg-theme-tertiary rounded border border-theme-secondary font-mono text-theme-secondary">
                            "Esc"
                        </kbd>
                        <span class="ml-1">"to cancel"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}
