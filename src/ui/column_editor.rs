use crate::core::{Column, MySqlDataType, RelationshipOps, RelationshipType, SchemaGraph};
use crate::ui::liveshare_client::{
    ConnectionState, GraphOperation, RelationshipData, use_liveshare_context,
};
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use petgraph::graph::NodeIndex;

#[component]
pub fn ColumnEditor(
    /// Текущая колонка для редактирования (None для создания новой)
    column: Option<Column>,
    /// Callback при сохранении колонки
    #[prop(into)]
    on_save: Callback<Column>,
    /// Callback при отмене
    #[prop(into)]
    on_cancel: Callback<()>,
    /// Callback при удалении колонки
    #[prop(into)]
    on_delete: Callback<()>,
    /// Inline режим (без модального окна)
    #[prop(default = false)]
    inline: bool,
    /// Граф схемы (для создания FK)
    #[prop(optional)]
    graph: Option<RwSignal<SchemaGraph>>,
    /// Текущая таблица (для создания FK)
    #[prop(optional)]
    current_table: Option<NodeIndex>,
) -> impl IntoView {
    // Состояние формы
    let (name, set_name) = signal(column.as_ref().map(|c| c.name.clone()).unwrap_or_default());
    let (data_type, set_data_type) = signal(
        column
            .as_ref()
            .map(|c| c.data_type.clone())
            .unwrap_or_else(|| "INT".to_string()),
    );
    let (is_primary_key, set_is_primary_key) =
        signal(column.as_ref().map(|c| c.is_primary_key).unwrap_or(false));
    let (is_nullable, set_is_nullable) =
        signal(column.as_ref().map(|c| c.is_nullable).unwrap_or(true));
    let (is_unique, set_is_unique) = signal(column.as_ref().map(|c| c.is_unique).unwrap_or(false));
    let (default_value, set_default_value) = signal(
        column
            .as_ref()
            .and_then(|c| c.default_value.clone())
            .unwrap_or_default(),
    );
    let (error, set_error) = signal::<Option<String>>(None);

    // FK состояние
    let (is_foreign_key, set_is_foreign_key) = signal(false);
    let (fk_target_table, set_fk_target_table) = signal::<Option<NodeIndex>>(None);
    let (fk_target_column, set_fk_target_column) = signal::<Option<String>>(None);
    let (fk_relationship_type, set_fk_relationship_type) = signal(RelationshipType::OneToMany);

    let available_types = MySqlDataType::all_types();

    let handle_save = move |_| {
        let name_value = name.get();
        let data_type_value = data_type.get();

        // Валидация
        if let Err(e) = Column::validate_name(&name_value) {
            set_error.set(Some(e));
            return;
        }

        if let Err(e) = Column::validate_data_type(&data_type_value) {
            set_error.set(Some(e));
            return;
        }

        let mut new_column = Column::new(name_value, data_type_value);

        if is_primary_key.get() {
            new_column = new_column.primary_key();
        } else if !is_nullable.get() {
            new_column = new_column.not_null();
        }

        if is_unique.get() {
            new_column = new_column.unique();
        }

        let default = default_value.get();
        if !default.is_empty() {
            new_column = new_column.with_default(default);
        }

        // Создание FK связи, если нужно
        if is_foreign_key.get()
            && let (Some(g), Some(current_node), Some(target_node), Some(target_col)) = (
                graph,
                current_table,
                fk_target_table.get(),
                fk_target_column.get(),
            )
        {
            let g_value = g.get();
            // Проверка совместимости типов
            if let Some(target_table) = g_value.node_weight(target_node)
                && let Some(target_column_obj) =
                    target_table.columns.iter().find(|c| c.name == target_col)
                && !new_column.is_type_compatible_with(target_column_obj)
            {
                set_error.set(Some(format!(
                    "Column type {} is not compatible with target column type {}",
                    new_column.data_type, target_column_obj.data_type
                )));
                return;
            }

            // Создание связи
            use crate::core::Relationship;
            let rel_name = format!("fk_{}_{}", new_column.name, target_col);
            let rel_type = fk_relationship_type.get();
            let from_col = new_column.name.clone();
            let to_col = target_col.clone();

            g.update(|graph| {
                let relationship = Relationship::new(
                    rel_name.clone(),
                    rel_type.clone(),
                    from_col.clone(),
                    to_col.clone(),
                );

                if let Ok(edge_idx) =
                    graph.create_relationship(current_node, target_node, relationship)
                {
                    // Send LiveShare sync
                    let liveshare_ctx = use_liveshare_context();
                    if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected
                    {
                        liveshare_ctx.send_graph_op(GraphOperation::CreateRelationship {
                            edge_id: edge_idx.index() as u32,
                            from_node: current_node.index() as u32,
                            to_node: target_node.index() as u32,
                            relationship: RelationshipData {
                                name: rel_name,
                                relationship_type: rel_type.to_string(),
                                from_column: from_col,
                                to_column: to_col,
                            },
                        });
                    }
                }
            });
        }

        on_save.run(new_column);
    };

    let form_content = view! {
        <div class=if inline {
            ""
        } else {
            "bg-theme-surface border border-theme-primary rounded-lg shadow-theme-xl p-6 w-full max-w-md theme-transition"
        }>
            {if !inline {
                view! {
                    <h2 class="text-2xl font-bold text-theme-primary mb-4">
                        {if column.is_some() { "Edit Column" } else { "New Column" }}
                    </h2>
                }
                    .into_any()
            } else {
                view! {
                    <h3 class="text-lg font-semibold text-theme-primary mb-4">
                        {if column.is_some() { "Edit Column" } else { "New Column" }}
                    </h3>
                }
                    .into_any()
            }}

            {move || {
                error
                    .get()
                    .map(|err| {
                        view! {
                            <div class="bg-theme-error border border-theme-error text-theme-error px-4 py-3 rounded mb-4 theme-transition">
                                {err}
                            </div>
                        }
                    })
            }}

            <div class="space-y-4">
                // Имя колонки
                <div>
                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                        "Column Name"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 input-theme rounded-md"
                        placeholder="e.g., user_id"
                        prop:value=move || name.get()
                        on:input=move |ev| {
                            set_name.set(event_target_value(&ev));
                            set_error.set(None);
                        }
                    />
                </div>

                // Тип данных
                <div>
                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                        "Data Type"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <select
                        class="w-full px-3 py-2 input-theme rounded-md"
                        on:change=move |ev| {
                            set_data_type.set(event_target_value(&ev));
                        }
                    >
                        {available_types
                            .iter()
                            .map(|&dt| {
                                let selected = dt == data_type.get_untracked().as_str();
                                view! {
                                    <option value=dt selected=selected>
                                        {dt}
                                    </option>
                                }
                            })
                            .collect_view()}

                    </select>
                </div>

                // Чекбоксы
                <div class="space-y-2">
                    <label class="flex items-center">
                        <input
                            type="checkbox"
                            class="mr-2 h-4 w-4 text-blue-600 rounded focus:ring-blue-500 bg-theme-surface border-theme-secondary"
                            prop:checked=move || is_primary_key.get()
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                set_is_primary_key.set(checked);
                                if checked {
                                    set_is_nullable.set(false);
                                }
                            }
                        />
                        <span class="text-sm text-theme-secondary">"Primary Key"</span>
                    </label>

                    <label class="flex items-center">
                        <input
                            type="checkbox"
                            class="mr-2 h-4 w-4 text-blue-600 rounded focus:ring-blue-500 bg-theme-surface border-theme-secondary"
                            prop:checked=move || is_nullable.get()
                            prop:disabled=move || is_primary_key.get()
                            on:change=move |ev| {
                                set_is_nullable.set(event_target_checked(&ev));
                            }
                        />
                        <span class="text-sm text-theme-secondary">"Nullable"</span>
                    </label>

                    <label class="flex items-center">
                        <input
                            type="checkbox"
                            class="mr-2 h-4 w-4 text-blue-600 rounded focus:ring-blue-500 bg-theme-surface border-theme-secondary"
                            prop:checked=move || is_unique.get()
                            on:change=move |ev| {
                                set_is_unique.set(event_target_checked(&ev));
                            }
                        />
                        <span class="text-sm text-theme-secondary">"Unique"</span>
                    </label>
                </div>

                // Значение по умолчанию
                <div>
                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                        "Default Value"
                    </label>
                    <input
                        type="text"
                        class="w-full px-3 py-2 input-theme rounded-md"
                        placeholder="Leave empty for no default"
                        prop:value=move || default_value.get()
                        on:input=move |ev| {
                            set_default_value.set(event_target_value(&ev));
                        }
                    />
                </div>

                // Foreign Key секция
                {move || {
                    if graph.is_some() && current_table.is_some() {
                        view! {
                            <div class="border-t border-theme-primary pt-4">
                                <label class="flex items-center mb-3">
                                    <input
                                        type="checkbox"
                                        class="mr-2 h-4 w-4 text-blue-600 rounded focus:ring-blue-500 bg-theme-surface border-theme-secondary"
                                        prop:checked=move || is_foreign_key.get()
                                        on:change=move |ev| {
                                            set_is_foreign_key.set(event_target_checked(&ev));
                                        }
                                    />
                                    <span class="text-sm font-medium text-theme-secondary">"Foreign Key"</span>
                                </label>

                                {move || {
                                    if is_foreign_key.get() {
                                        let g = graph.unwrap();
                                        view! {
                                            <div class="ml-6 space-y-3 bg-theme-secondary p-3 rounded-md theme-transition">
                                                // Выбор целевой таблицы
                                                <div>
                                                    <label class="block text-xs font-medium text-theme-secondary mb-1">
                                                        "References Table"
                                                    </label>
                                                    <select
                                                        class="w-full px-2 py-1.5 text-sm input-theme rounded-md"
                                                        on:change=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            if !value.is_empty() {
                                                                if let Ok(idx) = value.parse::<usize>() {
                                                                    set_fk_target_table.set(Some(NodeIndex::new(idx)));
                                                                    set_fk_target_column.set(None);
                                                                }
                                                            } else {
                                                                set_fk_target_table.set(None);
                                                                set_fk_target_column.set(None);
                                                            }
                                                        }
                                                    >
                                                        <option value="">"-- Select Table --"</option>
                                                        {move || {
                                                            let graph_val = g.get();
                                                            graph_val
                                                                .node_indices()
                                                                .filter(|&idx| Some(idx) != current_table)
                                                                .map(|idx| {
                                                                    let table = graph_val.node_weight(idx).unwrap();
                                                                    let selected = fk_target_table.get() == Some(idx);
                                                                    view! {
                                                                        <option value=idx.index().to_string() selected=selected>
                                                                            {table.name.clone()}
                                                                        </option>
                                                                    }
                                                                })
                                                                .collect_view()
                                                        }}
                                                    </select>
                                                </div>

                                                // Выбор целевой колонки
                                                {move || {
                                                    if let Some(target_idx) = fk_target_table.get() {
                                                        let graph_val = g.get();
                                                        if let Some(target_table) = graph_val.node_weight(target_idx) {
                                                            // Создаём временную колонку для проверки совместимости
                                                            let temp_col = Column::new(
                                                                name.get(),
                                                                data_type.get()
                                                            );

                                                            // Клонируем колонки для использования в closure
                                                            let compatible_columns: Vec<Column> = target_table
                                                                .columns
                                                                .iter()
                                                                .filter(|c| temp_col.is_type_compatible_with(c))
                                                                .cloned()
                                                                .collect();

                                                            let is_empty = compatible_columns.is_empty();

                                                            view! {
                                                                <div>
                                                                    <label class="block text-xs font-medium text-theme-secondary mb-1">
                                                                        "References Column"
                                                                    </label>
                                                                    <select
                                                                        class="w-full px-2 py-1.5 text-sm input-theme rounded-md"
                                                                        on:change=move |ev| {
                                                                            let value = event_target_value(&ev);
                                                                            if !value.is_empty() {
                                                                                set_fk_target_column.set(Some(value));
                                                                            } else {
                                                                                set_fk_target_column.set(None);
                                                                            }
                                                                        }
                                                                    >
                                                                        <option value="">"-- Select Column --"</option>
                                                                        {compatible_columns
                                                                            .into_iter()
                                                                            .map(|col| {
                                                                                let col_name = col.name.clone();
                                                                                let col_name2 = col.name.clone();
                                                                                let col_type = col.data_type.clone();
                                                                                let selected = fk_target_column.get()
                                                                                    .as_ref()
                                                                                    .map(|s| s == &col_name)
                                                                                    .unwrap_or(false);
                                                                                view! {
                                                                                    <option value=col_name selected=selected>
                                                                                        {col_name2}
                                                                                        " ("
                                                                                        {col_type}
                                                                                        ")"
                                                                                    </option>
                                                                                }
                                                                            })
                                                                            .collect_view()}
                                                                    </select>
                                                                    {move || {
                                                                        if is_empty {
                                                                            view! {
                                                                                <p class="text-xs text-theme-warning mt-1">
                                                                                    "⚠ No compatible columns found in target table"
                                                                                </p>
                                                                            }
                                                                                .into_any()
                                                                        } else {
                                                                            view! { <div></div> }.into_any()
                                                                        }
                                                                    }}
                                                                </div>
                                                            }
                                                                .into_any()
                                                        } else {
                                                            view! { <div></div> }.into_any()
                                                        }
                                                    } else {
                                                        view! { <div></div> }.into_any()
                                                    }
                                                }}

                                                // Выбор типа связи
                                                <div>
                                                    <label class="block text-xs font-medium text-theme-secondary mb-1">
                                                        "Relationship Type"
                                                    </label>
                                                    <select
                                                        class="w-full px-2 py-1.5 text-sm input-theme rounded-md"
                                                        on:change=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            let rel_type = match value.as_str() {
                                                                "1:1" => RelationshipType::OneToOne,
                                                                "N:M" => RelationshipType::ManyToMany,
                                                                _ => RelationshipType::OneToMany,
                                                            };
                                                            set_fk_relationship_type.set(rel_type);
                                                        }
                                                    >
                                                        <option value="1:N" selected=true>"One to Many (1:N)"</option>
                                                        <option value="1:1">"One to One (1:1)"</option>
                                                        <option value="N:M">"Many to Many (N:M)"</option>
                                                    </select>
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            // Кнопки
            <div class="flex justify-between mt-6">
                <div>
                    {if column.is_some() {
                        view! {
                            <button
                                class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 flex items-center dark:bg-red-700 dark:hover:bg-red-600"
                                on:click=move |_| on_delete.run(())
                            >
                                <Icon name=icons::TRASH class="w-4 h-4 mr-2"/>
                                "Delete"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>
                <div class="flex space-x-3">
                    <button
                        class="px-4 py-2 border border-theme-primary rounded-md text-theme-secondary hover:bg-theme-tertiary focus:outline-none focus:ring-2 focus:ring-theme-accent flex items-center theme-transition"
                        on:click=move |_| on_cancel.run(())
                    >
                        <Icon name=icons::X class="w-4 h-4 mr-2"/>
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 btn-theme-primary rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 flex items-center"
                        on:click=handle_save
                    >
                        <Icon name=icons::CHECK class="w-4 h-4 mr-2"/>
                        "Save"
                    </button>
                </div>
            </div>
        </div>
    };

    if inline {
        form_content.into_any()
    } else {
        view! {
            <div class="fixed inset-0 modal-backdrop-theme flex items-center justify-center z-50">
                {form_content}
            </div>
        }
        .into_any()
    }
}
