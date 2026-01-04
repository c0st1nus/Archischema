use crate::core::{Column, MySqlDataType, RelationshipOps, RelationshipType, SchemaGraph};
use crate::ui::liveshare_client::{
    ColumnData, ConnectionState, GraphOperation, RelationshipData, use_liveshare_context,
};
use crate::ui::{ErrorMessage, Icon, icons};
use leptos::prelude::*;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

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
pub fn ColumnEditor(
    /// Текущая колонка для редактирования (None для создания новой)
    column: Option<Column>,
    /// Индекс колонки (None для создания новой)
    #[prop(default = None)]
    column_index: Option<usize>,
    /// Callback при сохранении (вызывается после сохранения)
    #[prop(into)]
    on_save: Callback<()>,
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
    let initial_data_type = column
        .as_ref()
        .map(|c| c.data_type.clone())
        .unwrap_or_else(|| "INT".to_string());
    let (data_type, set_data_type) = signal(initial_data_type.clone());
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

    // Определяем начальное состояние FK из существующих связей
    let (initial_fk, initial_fk_table, initial_fk_column, initial_fk_type) = {
        if let (Some(g), Some(current_node), Some(col)) = (graph, current_table, column.as_ref()) {
            let graph_val = g.with_untracked(|v| v.clone());
            // Ищем связь, исходящую из текущей таблицы с этой колонкой
            let mut found_fk = false;
            let mut found_table: Option<NodeIndex> = None;
            let mut found_column: Option<String> = None;
            let mut found_type = RelationshipType::ManyToOne;

            for edge_ref in graph_val.edges(current_node) {
                let rel = edge_ref.weight();
                if rel.from_column == col.name {
                    found_fk = true;
                    found_table = Some(edge_ref.target());
                    found_column = Some(rel.to_column.clone());
                    found_type = rel.relationship_type.clone();
                    break;
                }
            }
            (found_fk, found_table, found_column, found_type)
        } else {
            (false, None, None, RelationshipType::ManyToOne)
        }
    };

    // FK состояние - инициализируем из существующих связей
    let (is_foreign_key, set_is_foreign_key) = signal(initial_fk);
    let (fk_target_table, set_fk_target_table) = signal::<Option<NodeIndex>>(initial_fk_table);
    let (fk_target_column, set_fk_target_column) = signal::<Option<String>>(initial_fk_column);
    let (fk_relationship_type, set_fk_relationship_type) = signal(initial_fk_type);

    // Сохраняем начальное имя колонки для обновления связей при переименовании
    let original_column_name = column.as_ref().map(|c| c.name.clone());

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

        let mut new_column = Column::new(name_value.clone(), data_type_value);

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

        // Собираем все данные FK до update()
        let is_fk = is_foreign_key.get();
        let target_table = fk_target_table.get();
        let target_col = fk_target_column.get();
        let rel_type = fk_relationship_type.get();

        // Обработка FK связей - все в одном update() для правильной реактивности
        if let (Some(g), Some(current_node)) = (graph, current_table) {
            // Проверка совместимости типов ДО update (только чтение)
            if is_fk
                && let (Some(target_node), Some(target_col_name)) =
                    (target_table, target_col.as_ref())
            {
                let g_value = g.with_untracked(|v| v.clone());
                if let Some(target_table_node) = g_value.node_weight(target_node)
                    && let Some(target_column_obj) = target_table_node
                        .columns
                        .iter()
                        .find(|c| c.name == *target_col_name)
                    && !new_column.is_type_compatible_with(target_column_obj)
                {
                    set_error.set(Some(format!(
                        "Column type {} is not compatible with target column type {}",
                        new_column.data_type, target_column_obj.data_type
                    )));
                    return;
                }
            }

            // Подготавливаем данные для LiveShare sync
            let liveshare_ctx = use_liveshare_context();
            let is_connected =
                liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected;

            // Клонируем данные для использования в closure
            let name_value_clone = name_value.clone();
            let original_name = original_column_name.clone();
            let fk_target = target_table;
            let fk_col = target_col.clone();
            let fk_rel_type = rel_type.clone();
            let fk_enabled = is_fk;
            let col_idx = column_index;
            let column_to_save = new_column.clone();

            // Все изменения графа в одном update() - и колонка, и FK связь
            g.update(move |graph_mut| {
                use crate::core::Relationship;

                // 1. Сохраняем колонку в таблицу
                if let Some(node) = graph_mut.node_weight_mut(current_node) {
                    if let Some(idx) = col_idx {
                        // Обновление существующей колонки
                        if idx < node.columns.len() {
                            node.columns[idx] = column_to_save.clone();
                        }
                    } else {
                        // Добавление новой колонки
                        node.columns.push(column_to_save.clone());
                    }
                }

                // 2. Удаляем старую FK связь (если редактируем существующую колонку)
                let col_name_for_old_fk = original_name.as_ref().unwrap_or(&name_value_clone);
                let edges_to_remove: Vec<_> = graph_mut
                    .edges(current_node)
                    .filter(|e| e.weight().from_column == *col_name_for_old_fk)
                    .map(|e| e.id())
                    .collect();

                for edge_id in edges_to_remove {
                    graph_mut.remove_edge(edge_id);
                }

                // 3. Создаём новую FK связь, если нужно
                if fk_enabled
                    && let (Some(target_node), Some(target_col_name)) = (fk_target, fk_col)
                {
                    let rel_name = format!("fk_{}_{}", column_to_save.name, target_col_name);
                    let from_col = column_to_save.name.clone();
                    let to_col = target_col_name.clone();

                    let relationship = Relationship::new(
                        rel_name.clone(),
                        fk_rel_type.clone(),
                        from_col.clone(),
                        to_col.clone(),
                    );

                    if let Ok(edge_idx) =
                        graph_mut.create_relationship(current_node, target_node, relationship)
                    {
                        // Send LiveShare sync for relationship
                        if is_connected {
                            liveshare_ctx.send_graph_op(GraphOperation::CreateRelationship {
                                edge_id: edge_idx.index() as u32,
                                from_node: current_node.index() as u32,
                                to_node: target_node.index() as u32,
                                relationship: RelationshipData {
                                    name: rel_name,
                                    relationship_type: fk_rel_type.to_string(),
                                    from_column: from_col,
                                    to_column: to_col,
                                },
                            });
                        }
                    }
                }
            });

            // Send LiveShare sync for column
            if is_connected {
                let col_data = ColumnData {
                    name: new_column.name.clone(),
                    data_type: new_column.data_type.clone(),
                    is_primary_key: new_column.is_primary_key,
                    is_nullable: new_column.is_nullable,
                    is_unique: new_column.is_unique,
                    default_value: new_column.default_value.clone(),
                    foreign_key: None,
                };

                if let (Some(g), Some(current_node)) = (graph, current_table) {
                    let table_uuid = g.with(|g| {
                        g.node_weight(current_node)
                            .map(|n| n.uuid)
                            .unwrap_or_else(uuid::Uuid::new_v4)
                    });

                    if let Some(idx) = column_index {
                        liveshare_ctx.send_graph_op(GraphOperation::UpdateColumn {
                            node_id: current_node.index() as u32,
                            table_uuid,
                            column_index: idx,
                            column: col_data,
                        });
                    } else {
                        liveshare_ctx.send_graph_op(GraphOperation::AddColumn {
                            node_id: current_node.index() as u32,
                            table_uuid,
                            column: col_data,
                        });
                    }
                }
            }
        }

        // Trigger save after column changes
        if column_index.is_some() {
            dispatch_save_event("column_updated");
        } else {
            dispatch_save_event("column_added");
        }

        // If FK was added/changed, trigger additional save event
        if is_foreign_key.get() {
            dispatch_save_event("foreign_key_changed");
        }

        on_save.run(());
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
                    <h3 class="title-lg mb-4">
                        {if column.is_some() { "Edit Column" } else { "New Column" }}
                    </h3>
                }
                    .into_any()
            }}

            <ErrorMessage error=error/>

            <div class="space-y-4">
                // Имя колонки
                <div>
                    <label class="label-sm">
                        "Column Name"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="input-base input-sm"
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
                    <label class="label-sm">
                        "Data Type"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <select
                        class="select-base"
                        prop:value=move || data_type.get()
                        on:change=move |ev| {
                            set_data_type.set(event_target_value(&ev));
                        }
                    >
                        {available_types
                            .iter()
                            .map(|&dt| {
                                view! {
                                    <option value=dt>
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
                    <label class="label-sm">
                        "Default Value"
                    </label>
                    <input
                        type="text"
                        class="input-base input-sm"
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
                            <div class="divider-top pt-4">
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
                                            <div class="ml-6 space-y-3 card-info">
                                                // Выбор целевой таблицы
                                                <div>
                                                    <label class="label-sm">
                                                        "References Table"
                                                    </label>
                                                    <select
                                                        class="select-base"
                                                        prop:value=move || {
                                                            fk_target_table.get()
                                                                .map(|idx| idx.index().to_string())
                                                                .unwrap_or_default()
                                                        }
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
                                                                    view! {
                                                                        <option value=idx.index().to_string()>
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
                                                                    <label class="label-sm">
                                                                        "References Column"
                                                                    </label>
                                                                    <select
                                                                        class="select-base"
                                                                        prop:value=move || {
                                                                            fk_target_column.get().unwrap_or_default()
                                                                        }
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
                                                                                view! {
                                                                                    <option value=col_name>
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

                                                // Выбор типа связи (без ManyToMany - делается через дополнительную таблицу)
                                                <div>
                                                    <label class="label-sm">
                                                        "Relationship Type"
                                                    </label>
                                                    <select
                                                        class="select-base"
                                                        prop:value=move || {
                                                            match fk_relationship_type.get() {
                                                                RelationshipType::OneToOne => "1:1",
                                                                RelationshipType::ManyToOne => "N:1",
                                                                RelationshipType::OneToMany => "1:N",
                                                                RelationshipType::ManyToMany => "N:1", // Fallback, shouldn't happen in UI
                                                            }
                                                        }
                                                        on:change=move |ev| {
                                                            let value = event_target_value(&ev);
                                                            let rel_type = match value.as_str() {
                                                                "1:1" => RelationshipType::OneToOne,
                                                                "1:N" => RelationshipType::OneToMany,
                                                                _ => RelationshipType::ManyToOne,
                                                            };
                                                            set_fk_relationship_type.set(rel_type);
                                                        }
                                                    >
                                                        <option value="N:1">"Many to One (N:1)"</option>
                                                        <option value="1:N">"One to Many (1:N)"</option>
                                                        <option value="1:1">"One to One (1:1)"</option>
                                                    </select>
                                                    <p class="text-xs text-theme-muted mt-1">
                                                        "For Many-to-Many relationships, create a junction table"
                                                    </p>
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
                                class="btn-danger"
                                on:click=move |_| on_delete.run(())
                            >
                                <Icon name=icons::TRASH class="icon-btn"/>
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
                        class="btn-secondary"
                        on:click=move |_| on_cancel.run(())
                    >
                        <Icon name=icons::X class="icon-btn"/>
                        "Cancel"
                    </button>
                    <button
                        class="btn-primary"
                        on:click=handle_save
                    >
                        <Icon name=icons::CHECK class="icon-btn"/>
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
