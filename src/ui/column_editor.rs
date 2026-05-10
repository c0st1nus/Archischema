use crate::core::{Column, RelationshipOps, RelationshipType, SchemaGraph};
use crate::ui::liveshare_client::{
    ColumnData, ConnectionState, GraphOperation, RelationshipData, use_liveshare_context,
};
use crate::ui::{ErrorMessage, Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;
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

const TYPE_OPTIONS: &[(&str, &str, &str)] = &[
    ("uuid", "Identity", "128-bit RFC 4122"),
    ("bigserial", "Identity", "auto-incrementing 8B"),
    ("int", "Numeric", "4 bytes"),
    ("bigint", "Numeric", "8 bytes"),
    ("numeric(10,2)", "Numeric", "exact decimal"),
    ("real", "Numeric", "4-byte float"),
    ("boolean", "Flags", "true / false"),
    ("varchar(255)", "Text", "variable with limit"),
    ("text", "Text", "unbounded text"),
    ("jsonb", "Text", "binary JSON"),
    ("date", "Time", "calendar date"),
    ("timestamp", "Time", "without time zone"),
    ("timestamptz", "Time", "with time zone"),
    ("inet", "Network", "IPv4 / IPv6"),
    ("bytea", "Binary", "binary data"),
];

const DEFAULT_VALUE_OPTIONS: &[(&str, &str)] = &[
    ("NULL", "explicit null"),
    ("now()", "current timestamp"),
    ("current_timestamp", "SQL timestamp"),
    ("gen_random_uuid()", "pgcrypto UUID"),
    ("true", "boolean true"),
    ("false", "boolean false"),
    ("''", "empty string"),
    ("'{}'::jsonb", "empty jsonb object"),
];

const REFERENTIAL_ACTIONS: &[(&str, &str)] = &[
    ("NO ACTION", "No action"),
    ("RESTRICT", "Restrict"),
    ("CASCADE", "Cascade"),
    ("SET NULL", "Set null"),
    ("SET DEFAULT", "Set default"),
];

fn option_matches(query: &str, value: &str, group: &str, hint: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty()
        || value.to_ascii_lowercase().contains(&query)
        || group.to_ascii_lowercase().contains(&query)
        || hint.to_ascii_lowercase().contains(&query)
}

fn has_forbidden_sql_fragment(value: &str) -> bool {
    value.contains(';') || value.contains("--") || value.contains("/*") || value.contains("*/")
}

fn has_balanced_parentheses(value: &str) -> bool {
    let mut depth = 0_i32;
    for ch in value.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn validate_default_expression(expression: &str) -> Result<(), String> {
    let value = expression.trim();
    if value.is_empty() {
        return Ok(());
    }
    if has_forbidden_sql_fragment(value) {
        return Err(
            "Default expression cannot contain SQL statement separators or comments".to_string(),
        );
    }
    if !has_balanced_parentheses(value) {
        return Err("Default expression has unbalanced parentheses".to_string());
    }
    if !value.matches('\'').count().is_multiple_of(2) {
        return Err("Default expression has an unterminated string literal".to_string());
    }
    let lower = value.to_ascii_lowercase();
    if [
        "drop ",
        "alter ",
        "insert ",
        "update ",
        "delete ",
        "truncate ",
        "create ",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        return Err("Default expression cannot contain DDL or DML statements".to_string());
    }
    Ok(())
}

fn normalize_action(action: &str) -> String {
    REFERENTIAL_ACTIONS
        .iter()
        .find(|(value, _)| value.eq_ignore_ascii_case(action.trim()))
        .map(|(value, _)| (*value).to_string())
        .unwrap_or_else(|| "NO ACTION".to_string())
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
    /// Draft-mode save callback. When provided, the editor returns a Column and
    /// does not mutate SchemaGraph or send LiveShare operations.
    #[prop(optional, into)]
    on_column_save: Option<Callback<Column>>,
    /// Optional duplicate-name check scoped by the parent.
    #[prop(optional, into)]
    column_name_exists: Option<Callback<String, bool>>,
    /// Navigate to the previous existing column in the current table.
    #[prop(optional, into)]
    on_previous: Option<Callback<()>>,
    /// Navigate to the next existing column in the current table.
    #[prop(optional, into)]
    on_next: Option<Callback<()>>,
    #[prop(default = false)] can_previous: bool,
    #[prop(default = false)] can_next: bool,
) -> impl IntoView {
    // Состояние формы
    let (name, set_name) = signal(column.as_ref().map(|c| c.name.clone()).unwrap_or_default());
    let initial_data_type = column
        .as_ref()
        .map(|c| c.data_type.clone())
        .unwrap_or_else(|| "uuid".to_string());
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
    let (
        initial_fk,
        initial_fk_table,
        initial_fk_column,
        initial_fk_type,
        initial_fk_on_delete,
        initial_fk_on_update,
    ) = {
        if let (Some(g), Some(current_node), Some(col)) = (graph, current_table, column.as_ref()) {
            let graph_val = g.with_untracked(|v| v.clone());
            // Ищем связь, исходящую из текущей таблицы с этой колонкой
            let mut found_fk = false;
            let mut found_table: Option<NodeIndex> = None;
            let mut found_column: Option<String> = None;
            let mut found_type = RelationshipType::ManyToOne;
            let mut found_on_delete = "NO ACTION".to_string();
            let mut found_on_update = "NO ACTION".to_string();

            for edge_ref in graph_val.edges(current_node) {
                let rel = edge_ref.weight();
                if rel.from_column == col.name {
                    found_fk = true;
                    found_table = Some(edge_ref.target());
                    found_column = Some(rel.to_column.clone());
                    found_type = rel.relationship_type.clone();
                    found_on_delete = rel.on_delete.clone();
                    found_on_update = rel.on_update.clone();
                    break;
                }
            }
            (
                found_fk,
                found_table,
                found_column,
                found_type,
                found_on_delete,
                found_on_update,
            )
        } else {
            (
                false,
                None,
                None,
                RelationshipType::ManyToOne,
                "NO ACTION".to_string(),
                "NO ACTION".to_string(),
            )
        }
    };

    // FK состояние - инициализируем из существующих связей
    let (is_foreign_key, set_is_foreign_key) = signal(initial_fk);
    let (fk_target_table, set_fk_target_table) = signal::<Option<NodeIndex>>(initial_fk_table);
    let (fk_target_column, set_fk_target_column) = signal::<Option<String>>(initial_fk_column);
    let (fk_relationship_type, set_fk_relationship_type) = signal(initial_fk_type);
    let (fk_on_delete, set_fk_on_delete) = signal(normalize_action(&initial_fk_on_delete));
    let (fk_on_update, set_fk_on_update) = signal(normalize_action(&initial_fk_on_update));
    let (show_type_suggestions, set_show_type_suggestions) = signal(false);
    let (show_default_suggestions, set_show_default_suggestions) = signal(false);

    // Сохраняем начальное имя колонки для обновления связей при переименовании
    let original_column_name = column.as_ref().map(|c| c.name.clone());
    let is_editing_column = column.is_some();
    let show_navigation = is_editing_column && (on_previous.is_some() || on_next.is_some());
    let on_previous_nav = on_previous.clone();
    let on_next_nav = on_next.clone();
    let on_cancel_backdrop = on_cancel.clone();

    let handle_save = move |_| {
        let name_value = name.get().trim().to_string();
        let data_type_value = data_type.get().trim().to_string();

        // Валидация
        if let Err(e) = Column::validate_name(&name_value) {
            set_error.set(Some(e));
            return;
        }

        if let Err(e) = Column::validate_data_type(&data_type_value) {
            set_error.set(Some(e));
            return;
        }

        if let Some(exists_fn) = column_name_exists.as_ref()
            && exists_fn.run(name_value.clone())
        {
            set_error.set(Some(format!("Column '{}' already exists", name_value)));
            return;
        }

        let default = default_value.get().trim().to_string();
        if let Err(e) = validate_default_expression(&default) {
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

        if !default.is_empty() {
            new_column = new_column.with_default(default);
        }

        if let Some(save_column) = on_column_save.as_ref() {
            save_column.run(new_column);
            on_save.run(());
            return;
        }

        // Собираем все данные FK до update()
        let is_fk = is_foreign_key.get();
        let target_table = fk_target_table.get();
        let target_col = fk_target_column.get();
        let rel_type = fk_relationship_type.get();
        let on_delete_action = normalize_action(&fk_on_delete.get());
        let on_update_action = normalize_action(&fk_on_update.get());

        if is_fk {
            if graph.is_none() || current_table.is_none() {
                set_error.set(Some(
                    "Foreign keys can only be configured for an existing table".to_string(),
                ));
                return;
            }
            if target_table.is_none() {
                set_error.set(Some("Foreign key target table is required".to_string()));
                return;
            }
            if target_col
                .as_ref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                set_error.set(Some("Foreign key target column is required".to_string()));
                return;
            }
            if (on_delete_action == "SET NULL" || on_update_action == "SET NULL")
                && !new_column.is_nullable
            {
                set_error.set(Some(
                    "SET NULL action requires the column to be nullable".to_string(),
                ));
                return;
            }
            if (on_delete_action == "SET DEFAULT" || on_update_action == "SET DEFAULT")
                && new_column.default_value.is_none()
            {
                set_error.set(Some(
                    "SET DEFAULT action requires a default value".to_string(),
                ));
                return;
            }
        }

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
            let fk_on_delete = on_delete_action.clone();
            let fk_on_update = on_update_action.clone();
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
                    )
                    .with_actions(fk_on_delete.clone(), fk_on_update.clone());

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
                                    on_delete: fk_on_delete,
                                    on_update: fk_on_update,
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
        <div
            class=if inline { "column-editor-panel" } else { "column-editor-panel column-editor-floating" }
            on:mousedown=move |ev: web_sys::MouseEvent| ev.stop_propagation()
        >
            <div class="column-editor-head">
                <h3 class="column-editor-title">
                    {if is_editing_column { "Edit column" } else { "New column" }}
                </h3>
                {if show_navigation {
                    view! {
                        <div class="column-editor-nav" aria-label="Column navigation">
                            <button
                                type="button"
                                class="btn-icon"
                                disabled=!can_previous
                                title="Previous column"
                                aria-label="Previous column"
                                on:click=move |_| {
                                    if can_previous && let Some(callback) = on_previous_nav.as_ref() {
                                        callback.run(());
                                    }
                                }
                            >
                                <Icon name=icons::CHEVRON_LEFT class="h-4 w-4"/>
                            </button>
                            <button
                                type="button"
                                class="btn-icon"
                                disabled=!can_next
                                title="Next column"
                                aria-label="Next column"
                                on:click=move |_| {
                                    if can_next && let Some(callback) = on_next_nav.as_ref() {
                                        callback.run(());
                                    }
                                }
                            >
                                <Icon name=icons::CHEVRON_RIGHT class="h-4 w-4"/>
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

            </div>

            <div class="column-editor-body scroll">
                <ErrorMessage error=error/>

                <section class="surface p-3">
                    <div class="form-section-head form-section-head-compact">
                        <span class="eyebrow">"Definition"</span>
                        <span class="chip h-[18px] text-[10px]">"SQL column"</span>
                    </div>
                    <div class="grid gap-3 md:grid-cols-[1fr_0.82fr]">
                        <label class="block">
                            <span class="field-label">"Name" <span class="text-theme-error">"*"</span></span>
                            <input
                                type="text"
                                autocomplete="off"
                                spellcheck="false"
                                class="input-base mt-1"
                                placeholder="user_id"
                                prop:value=move || name.get()
                                on:input=move |ev| {
                                    set_name.set(event_target_value(&ev));
                                    set_error.set(None);
                                }
                            />
                            <span class="field-help">"Use snake_case for predictable SQL output."</span>
                        </label>

                        <label class="block">
                            <span class="field-label">"Data type" <span class="text-theme-error">"*"</span></span>
                            <div class="suggestion-field mt-1">
                                <input
                                    type="text"
                                    autocomplete="off"
                                    spellcheck="false"
                                    class="input-base font-mono"
                                    placeholder="varchar(255)"
                                    prop:value=move || data_type.get()
                                    on:focus=move |_| set_show_type_suggestions.set(true)
                                    on:blur=move |_| set_show_type_suggestions.set(false)
                                    on:input=move |ev| {
                                        set_data_type.set(event_target_value(&ev));
                                        set_show_type_suggestions.set(true);
                                        set_error.set(None);
                                    }
                                />
                                {move || {
                                    if show_type_suggestions.get() {
                                        let query = data_type.get();
                                        let matches = TYPE_OPTIONS
                                            .iter()
                                            .filter(|option| option_matches(&query, option.0, option.1, option.2))
                                            .take(14)
                                            .collect::<Vec<_>>();

                                        view! {
                                            <div class="suggestion-popover">
                                                {if matches.is_empty() {
                                                    view! {
                                                        <div class="suggestion-empty">
                                                            "No preset match. Enter a supported SQL type."
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    matches.into_iter().map(|option| {
                                                        let value = option.0.to_string();
                                                        let value_for_click = value.clone();
                                                        let active_value = value.clone();
                                                        view! {
                                                            <button
                                                                type="button"
                                                                class=move || if data_type.get().eq_ignore_ascii_case(&active_value) { "suggestion-option is-active" } else { "suggestion-option" }
                                                                on:mousedown=move |ev: web_sys::MouseEvent| {
                                                                    ev.prevent_default();
                                                                    set_data_type.set(value_for_click.clone());
                                                                    set_show_type_suggestions.set(false);
                                                                    set_error.set(None);
                                                                }
                                                            >
                                                                <span class="suggestion-main">
                                                                    <span class="font-mono">{value}</span>
                                                                    <span class="suggestion-group">{option.1}</span>
                                                                </span>
                                                                <span class="suggestion-hint">{option.2}</span>
                                                            </button>
                                                        }
                                                    }).collect_view().into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}
                            </div>
                            <span class="field-help">"Focus to browse presets, type to filter, or enter a supported SQL type."</span>
                        </label>
                    </div>
                </section>

                <section class="surface p-3">
                    <div class="form-section-head form-section-head-compact">
                        <span class="eyebrow">"Constraints"</span>
                        <span class="text-xs text-theme-muted">"Active pills are persisted"</span>
                    </div>
                    <div class="constraint-pill-grid">
                        <button
                            type="button"
                            class=move || if is_primary_key.get() { "constraint-pill is-active" } else { "constraint-pill" }
                            aria-pressed=move || is_primary_key.get()
                            on:click=move |_| {
                                let checked = !is_primary_key.get();
                                set_is_primary_key.set(checked);
                                if checked {
                                    set_is_nullable.set(false);
                                }
                            }
                        >
                            <Icon name=icons::KEY class="h-3.5 w-3.5"/>
                            <span>"Primary key"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if !is_nullable.get() { "constraint-pill is-active" } else { "constraint-pill" }
                            aria-pressed=move || !is_nullable.get()
                            disabled=move || is_primary_key.get()
                            title="Primary keys are always not nullable"
                            on:click=move |_| {
                                if !is_primary_key.get() {
                                    set_is_nullable.update(|value| *value = !*value);
                                }
                            }
                        >
                            <span>"Not null"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if is_unique.get() { "constraint-pill is-active" } else { "constraint-pill" }
                            aria-pressed=move || is_unique.get()
                            on:click=move |_| set_is_unique.update(|value| *value = !*value)
                        >
                            <span>"Unique"</span>
                        </button>
                        <button
                            type="button"
                            class=move || if is_foreign_key.get() { "constraint-pill is-active" } else { "constraint-pill" }
                            aria-pressed=move || is_foreign_key.get()
                            disabled=move || graph.is_none() || current_table.is_none()
                            title=if graph.is_some() && current_table.is_some() {
                                "Configure foreign key"
                            } else {
                                "Foreign keys are available after the table exists"
                            }
                            on:click=move |_| {
                                if graph.is_some() && current_table.is_some() {
                                    set_is_foreign_key.update(|value| *value = !*value);
                                }
                            }
                        >
                            <Icon name=icons::GIT_BRANCH class="h-3.5 w-3.5"/>
                            <span>"Foreign key"</span>
                        </button>
                        <button type="button" class="constraint-pill" disabled=true title="CHECK constraints are not part of the current model">"Check"</button>
                        <button type="button" class="constraint-pill" disabled=true title="Generated columns are not part of the current model">"Generated"</button>
                        <button type="button" class="constraint-pill" disabled=true title="Column comments are preview-only in this pass">"Comment"</button>
                    </div>
                    <p class="field-help">"Foreign key opens settings below. Disabled pills are not part of the current model."</p>
                </section>

                {move || {
                    if graph.is_some() && current_table.is_some() && is_foreign_key.get() {
                        let g = graph.unwrap();
                        view! {
                            <section class="surface p-3">
                                <div class="form-section-head form-section-head-compact">
                                    <div>
                                        <div class="eyebrow mb-1">"Foreign key"</div>
                                        <p class="text-xs text-theme-muted">"Creates or updates the current graph relationship."</p>
                                    </div>
                                </div>

                                <div class="fk-editor-card">
                                    <div class="grid gap-3 md:grid-cols-2">
                                        <label class="block">
                                            <span class="field-label">"References table"</span>
                                            <select
                                                class="select-base input-sm mt-1"
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
                                                <option value="">"Select table"</option>
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
                                        </label>

                                        <label class="block">
                                            <span class="field-label">"Relationship type"</span>
                                            <select
                                                class="select-base input-sm mt-1"
                                                prop:value=move || {
                                                    match fk_relationship_type.get() {
                                                        RelationshipType::OneToOne => "1:1",
                                                        RelationshipType::ManyToOne => "N:1",
                                                        RelationshipType::OneToMany => "1:N",
                                                        RelationshipType::ManyToMany => "N:M",
                                                    }
                                                }
                                                on:change=move |ev| {
                                                    let value = event_target_value(&ev);
                                                    let rel_type = match value.as_str() {
                                                        "1:1" => RelationshipType::OneToOne,
                                                        "1:N" => RelationshipType::OneToMany,
                                                        "N:M" => RelationshipType::ManyToMany,
                                                        _ => RelationshipType::ManyToOne,
                                                    };
                                                    set_fk_relationship_type.set(rel_type);
                                                }
                                            >
                                                <option value="N:1">"Many to one (N:1)"</option>
                                                <option value="1:N">"One to many (1:N)"</option>
                                                <option value="1:1">"One to one (1:1)"</option>
                                                <option value="N:M">"Many to many (N:M)"</option>
                                            </select>
                                        </label>
                                    </div>

                                    {move || {
                                        if let Some(target_idx) = fk_target_table.get() {
                                            let graph_val = g.get();
                                            if let Some(target_table) = graph_val.node_weight(target_idx) {
                                                let temp_col = Column::new(name.get(), data_type.get());
                                                let compatible_columns: Vec<Column> = target_table
                                                    .columns
                                                    .iter()
                                                    .filter(|c| temp_col.is_type_compatible_with(c))
                                                    .cloned()
                                                    .collect();
                                                let is_empty = compatible_columns.is_empty();

                                                view! {
                                                    <label class="mt-3 block">
                                                        <span class="field-label">"References column"</span>
                                                        <select
                                                            class="select-base input-sm mt-1"
                                                            prop:value=move || fk_target_column.get().unwrap_or_default()
                                                            on:change=move |ev| {
                                                                let value = event_target_value(&ev);
                                                                if !value.is_empty() {
                                                                    set_fk_target_column.set(Some(value));
                                                                } else {
                                                                    set_fk_target_column.set(None);
                                                                }
                                                            }
                                                        >
                                                            <option value="">"Select column"</option>
                                                            {compatible_columns
                                                                .into_iter()
                                                                .map(|col| {
                                                                    let col_name = col.name.clone();
                                                                    let col_name2 = col.name.clone();
                                                                    let col_type = col.data_type.clone();
                                                                    view! {
                                                                        <option value=col_name>
                                                                            {col_name2} " (" {col_type} ")"
                                                                        </option>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </select>
                                                        {move || {
                                                            if is_empty {
                                                                view! {
                                                                    <p class="mt-1 text-xs text-theme-warning">
                                                                        "No compatible columns found in target table."
                                                                    </p>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span></span> }.into_any()
                                                            }
                                                        }}
                                                    </label>
                                                }.into_any()
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }
                                    }}

                                    <div class="mt-3 grid gap-3 sm:grid-cols-2">
                                        <label class="block">
                                            <span class="field-label">"On delete"</span>
                                            <select
                                                class="select-base input-sm mt-1"
                                                prop:value=move || fk_on_delete.get()
                                                on:change=move |ev| set_fk_on_delete.set(normalize_action(&event_target_value(&ev)))
                                            >
                                                {REFERENTIAL_ACTIONS.iter().map(|(value, label)| {
                                                    view! { <option value=*value>{*label}</option> }
                                                }).collect_view()}
                                            </select>
                                        </label>
                                        <label class="block">
                                            <span class="field-label">"On update"</span>
                                            <select
                                                class="select-base input-sm mt-1"
                                                prop:value=move || fk_on_update.get()
                                                on:change=move |ev| set_fk_on_update.set(normalize_action(&event_target_value(&ev)))
                                            >
                                                {REFERENTIAL_ACTIONS.iter().map(|(value, label)| {
                                                    view! { <option value=*value>{*label}</option> }
                                                }).collect_view()}
                                            </select>
                                        </label>
                                    </div>
                                    <span class="field-help">"SET NULL requires a nullable column. SET DEFAULT requires a default value."</span>
                                </div>
                            </section>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}

                <section class="surface p-3">
                    <div class="form-section-head form-section-head-compact">
                        <span class="eyebrow">"Default value"</span>
                        <span class="text-xs text-theme-muted">"Raw SQL or literal"</span>
                    </div>
                    <label class="block">
                        <span class="field-label">"Expression"</span>
                        <div class="suggestion-field mt-1">
                            <input
                                type="text"
                                autocomplete="off"
                                spellcheck="false"
                                class="input-base font-mono"
                                placeholder="now(), 0, 'draft', NULL"
                                prop:value=move || default_value.get()
                                on:focus=move |_| set_show_default_suggestions.set(true)
                                on:blur=move |_| set_show_default_suggestions.set(false)
                                on:input=move |ev| {
                                    set_default_value.set(event_target_value(&ev));
                                    set_show_default_suggestions.set(true);
                                    set_error.set(None);
                                }
                            />
                            {move || {
                                if show_default_suggestions.get() {
                                    let query = default_value.get();
                                    let matches = DEFAULT_VALUE_OPTIONS
                                        .iter()
                                        .filter(|(value, hint)| option_matches(&query, value, "default", hint))
                                        .take(10)
                                        .collect::<Vec<_>>();

                                    view! {
                                        <div class="suggestion-popover">
                                            {if matches.is_empty() {
                                                view! {
                                                    <div class="suggestion-empty">
                                                        "No command match. Literal or custom SQL expression will be used."
                                                    </div>
                                                }.into_any()
                                            } else {
                                                matches.into_iter().map(|option| {
                                                    let value = option.0.to_string();
                                                    let value_for_click = value.clone();
                                                    let active_value = value.clone();
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class=move || if default_value.get().eq_ignore_ascii_case(&active_value) { "suggestion-option is-active" } else { "suggestion-option" }
                                                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                                                ev.prevent_default();
                                                                set_default_value.set(value_for_click.clone());
                                                                set_show_default_suggestions.set(false);
                                                                set_error.set(None);
                                                            }
                                                        >
                                                            <span class="suggestion-main">
                                                                <span class="font-mono">{value}</span>
                                                                <span class="suggestion-group">"command"</span>
                                                            </span>
                                                            <span class="suggestion-hint">{option.1}</span>
                                                        </button>
                                                    }
                                                }).collect_view().into_any()
                                            }}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}
                        </div>
                        <span class="field-help">"Use a suggested command or type any valid Postgres literal/expression."</span>
                    </label>
                </section>
            </div>

            <div class="column-editor-footer">
                <div>
                    {if is_editing_column {
                        view! {
                            <button
                                type="button"
                                class="btn-danger"
                                on:click=move |_| on_delete.run(())
                            >
                                <Icon name=icons::TRASH class="icon-btn"/>
                                "Delete"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! { <span class="text-xs text-theme-muted">"New column"</span> }.into_any()
                    }}
                </div>
                <div class="flex gap-2">
                    <button
                        type="button"
                        class="btn-secondary"
                        on:click=move |_| on_cancel.run(())
                    >
                        <Icon name=icons::X class="icon-btn"/>
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class="btn-primary"
                        on:click=handle_save
                    >
                        <Icon name=icons::CHECK class="icon-btn"/>
                        {if is_editing_column { "Save column" } else { "Create column" }}
                    </button>
                </div>
            </div>
        </div>
    };

    if inline {
        form_content.into_any()
    } else {
        view! {
            <div
                class="column-editor-backdrop fixed inset-0 flex items-center justify-center"
                on:mousedown=move |_| on_cancel_backdrop.run(())
            >
                {form_content}
            </div>
        }
        .into_any()
    }
}
