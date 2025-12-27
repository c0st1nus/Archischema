use crate::core::{Column, SchemaGraph, TableOps};
use crate::ui::column_editor::ColumnEditor;
use crate::ui::icon::{Icon, icons};
use crate::ui::liveshare_client::{
    ColumnData, ConnectionState, GraphOperation, use_liveshare_context,
};
use crate::ui::new_table_dialog::{CreateTableResult, NewTableData, NewTableDialog};
use crate::ui::source_editor::{EditorMode, EditorModeSwitcher};
use crate::ui::table_editor::TableEditor;
use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::components::A;
use petgraph::graph::NodeIndex;

/// Diagram name editor component - extracted to reduce nesting depth
#[component]
fn DiagramNameEditor(
    name_signal: RwSignal<String>,
    is_demo: bool,
    #[prop(default = None)] on_name_change: Option<Callback<String>>,
) -> impl IntoView {
    let is_editing = RwSignal::new(false);
    let edit_value = RwSignal::new(name_signal.with_untracked(|v| v.clone()));

    view! {
        {move || {
            if is_editing.get() {
                view! {
                    <input
                        type="text"
                        class="flex-1 min-w-0 px-2 py-1 text-sm font-medium bg-theme-surface border border-theme-primary rounded text-theme-primary focus:outline-none focus:ring-1 focus:ring-accent-primary"
                        prop:value=move || edit_value.get()
                        on:input=move |ev| edit_value.set(event_target_value(&ev))
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                let new_name = edit_value.get();
                                if !new_name.trim().is_empty() {
                                    name_signal.set(new_name.clone());
                                    if let Some(cb) = on_name_change.as_ref() {
                                        cb.run(new_name);
                                    }
                                }
                                is_editing.set(false);
                            } else if ev.key() == "Escape" {
                                edit_value.set(name_signal.get());
                                is_editing.set(false);
                            }
                        }
                        on:blur=move |_| {
                            let new_name = edit_value.get();
                            if !new_name.trim().is_empty() && new_name != name_signal.get() {
                                name_signal.set(new_name.clone());
                                if let Some(cb) = on_name_change.as_ref() {
                                    cb.run(new_name);
                                }
                            }
                            is_editing.set(false);
                        }
                        autofocus
                    />
                }.into_any()
            } else {
                let current_name = name_signal.get();
                view! {
                    <div class="flex items-center gap-2 flex-1 min-w-0 group">
                        <h1
                            class="text-sm font-semibold text-theme-primary truncate cursor-pointer hover:text-accent-primary transition-colors"
                            title=current_name.clone()
                            on:click=move |_| {
                                if !is_demo {
                                    edit_value.set(name_signal.get());
                                    is_editing.set(true);
                                }
                            }
                        >
                            {move || name_signal.get()}
                        </h1>
                        {if !is_demo {
                            view! {
                                <button
                                    class="flex-shrink-0 p-1 text-theme-muted hover:text-theme-primary opacity-0 group-hover:opacity-100 transition-all"
                                    on:click=move |_| {
                                        edit_value.set(name_signal.get());
                                        is_editing.set(true);
                                    }
                                    title="Rename diagram"
                                >
                                    <Icon name=icons::EDIT class="w-3.5 h-3.5" />
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <span class="flex-shrink-0 text-xs text-yellow-500 px-1.5 py-0.5 bg-yellow-500/10 rounded">"Demo"</span>
                            }.into_any()
                        }}
                    </div>
                }.into_any()
            }
        }}
    }
}

#[derive(Clone, Debug, PartialEq)]
enum EditingMode {
    None,
    CreatingTable,
    EditingColumn(NodeIndex, Option<usize>),
    EditingTable(NodeIndex),
}

#[component]
pub fn Sidebar(
    graph: RwSignal<SchemaGraph>,
    #[prop(into)] on_table_focus: Callback<NodeIndex>,
    /// Editor mode signal (Visual/Source)
    editor_mode: RwSignal<EditorMode>,
    /// Sidebar collapsed state (shared with parent for layout coordination)
    is_collapsed: RwSignal<bool>,
    /// Diagram name (editable)
    #[prop(default = None)]
    diagram_name: Option<RwSignal<String>>,
    /// Whether this is demo mode
    #[prop(default = false)]
    is_demo: bool,
    /// Callback when diagram name changes
    #[prop(default = None)]
    on_name_change: Option<Callback<String>>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    let set_is_collapsed = is_collapsed;
    let (search_query, set_search_query) = signal(String::new());
    let (expanded_tables, set_expanded_tables) = signal::<Vec<NodeIndex>>(Vec::new());

    // Состояние для редактора (колонка или таблица)
    let (editing_mode, set_editing_mode) = signal(EditingMode::None);

    // Helper to send graph operation when connected
    let send_graph_op = move |op: GraphOperation| {
        if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
            liveshare_ctx.send_graph_op(op);
        }
    };

    // Мемоизация статистики для предотвращения повторных подсчетов
    let total_tables = Memo::new(move |_| graph.with(|g| g.node_count()));
    let total_columns = Memo::new(move |_| {
        graph.with(|g| g.node_weights().map(|n| n.columns.len()).sum::<usize>())
    });
    let total_relations = Memo::new(move |_| graph.with(|g| g.edge_count()));

    // Мемоизация списка индексов узлов
    let node_indices = Memo::new(move |_| graph.with(|g| g.node_indices().collect::<Vec<_>>()));

    // Функция для переключения раскрытия таблицы
    let toggle_table = move |node_idx: NodeIndex| {
        set_expanded_tables.update(|expanded| {
            if expanded.contains(&node_idx) {
                expanded.retain(|&idx| idx != node_idx);
            } else {
                expanded.push(node_idx);
            }
        });
    };

    view! {
        <div class=move || {
            if is_collapsed.get() {
                "fixed left-0 top-0 h-screen w-14 bg-theme-surface border-r border-theme-primary shadow-theme-lg z-20 transition-all duration-300 theme-transition"
            } else {
                "fixed left-0 top-0 h-screen w-96 bg-theme-surface border-r border-theme-primary shadow-theme-xl z-20 transition-all duration-300 theme-transition"
            }
        }>
        {move || {
            if is_collapsed.get() {
                // Свернутый вид
                view! {
                    <div class="h-full flex flex-col items-center py-4 bg-theme-surface theme-transition">
                        <button
                            class="btn-icon"
                            on:click=move |_| set_is_collapsed.set(false)
                            title="Expand sidebar"
                        >
                            <Icon name=icons::PANEL_LEFT_OPEN class="icon-lg"/>
                        </button>
                    </div>
                }
                    .into_any()
            } else {
                // Развернутый вид
                view! {
                    <div class="h-full flex flex-col bg-theme-surface theme-transition">
                        // Navigation header with diagram name
                        <div class="px-4 py-3 border-b border-theme-primary bg-theme-tertiary theme-transition">
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2 flex-1 min-w-0">
                                    <A
                                        href="/dashboard"
                                        attr:class="btn-icon"
                                        attr:title="Back to Dashboard"
                                    >
                                        <Icon name=icons::ARROW_LEFT class="icon-standalone"/>
                                    </A>

                                    {if let Some(name_signal) = diagram_name {
                                        view! {
                                            <DiagramNameEditor
                                                name_signal=name_signal
                                                is_demo=is_demo
                                                on_name_change=on_name_change
                                            />
                                        }.into_any()
                                    } else {
                                        view! {
                                            <span class="text-sm font-semibold text-theme-primary">"Schema Editor"</span>
                                        }.into_any()
                                    }}
                                </div>
                                <button
                                    class="btn-icon"
                                    on:click=move |_| set_is_collapsed.set(true)
                                    title="Collapse sidebar"
                                >
                                    <Icon name=icons::PANEL_LEFT_CLOSE class="icon-standalone"/>
                                </button>
                            </div>
                        </div>

                        // Editor Mode Switcher
                        <div class="px-6 py-3 border-b border-theme-primary bg-theme-surface theme-transition">
                            <EditorModeSwitcher mode=editor_mode />
                        </div>

                        {move || {
                            match editing_mode.get() {
                                EditingMode::EditingColumn(node_idx, col_idx) => {
                                    // Режим редактирования колонки
                                    let g = graph.get();
                                    let node = g.node_weight(node_idx).cloned();
                                    let column = col_idx.and_then(|idx| {
                                        node.as_ref().and_then(|n| n.columns.get(idx).cloned())
                                    });
                                    let table_name = node.map(|n| n.name.clone()).unwrap_or_default();
                                    view! {
                                        <div class="flex-1 flex flex-col overflow-hidden">
                                            // Хлебные крошки
                                            <div class="px-6 py-3 divider-bottom bg-theme-secondary theme-transition">
                                                <button
                                                    class="nav-back"
                                                    on:click=move |_| set_editing_mode.set(EditingMode::None)
                                                >
                                                    <Icon name=icons::CHEVRON_LEFT class="icon-text"/>
                                                    "Back to tables"
                                                </button>
                                                <div class="mt-1 breadcrumb">
                                                    <span class="font-medium text-theme-secondary">{table_name}</span>
                                                    {if col_idx.is_some() {
                                                        " → Edit Column"
                                                    } else {
                                                        " → New Column"
                                                    }}
                                                </div>
                                            </div>

                                            // Редактор колонки в сайдбаре
                                            <div class="flex-1 overflow-y-auto px-6 py-4 bg-theme-surface theme-transition">
                                                <ColumnEditor
                                                    column=column
                                                    column_index=col_idx
                                                    inline=true
                                                    graph=graph
                                                    current_table=node_idx
                                                    on_save=move || {
                                                        // ColumnEditor теперь сам сохраняет колонку и FK в одном update()
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_cancel=move || {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_delete=move || {
                                                        if let Some(idx) = col_idx {
                                                            graph
                                                                .update(|g| {
                                                                    if let Some(node) = g.node_weight_mut(node_idx) && idx < node.columns.len() {
                                                                        node.columns.remove(idx);
                                                                    }
                                                                });
                                                            // Send sync op
                                                            send_graph_op(GraphOperation::DeleteColumn {
                                                                node_id: node_idx.index() as u32,
                                                                column_index: idx,
                                                            });
                                                        }
                                                        set_editing_mode.set(EditingMode::None);
                                                    }
                                                />
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                                EditingMode::EditingTable(node_idx) => {
                                    // Режим редактирования таблицы
                                    view! {
                                        <div class="flex-1 flex flex-col overflow-hidden">
                                            // Хлебные крошки
                                            <div class="px-6 py-3 divider-bottom bg-theme-secondary theme-transition">
                                                <button
                                                    class="nav-back"
                                                    on:click=move |_| set_editing_mode.set(EditingMode::None)
                                                >
                                                    <Icon name=icons::CHEVRON_LEFT class="icon-text"/>
                                                    "Back to tables"
                                                </button>
                                                <div class="mt-1 breadcrumb">
                                                    "Edit Table"
                                                </div>
                                            </div>

                                            // Редактор таблицы в сайдбаре
                                            <div class="flex-1 overflow-y-auto px-6 py-4 bg-theme-surface theme-transition">
                                                <TableEditor
                                                    graph=graph
                                                    node_idx=node_idx
                                                    on_save=move || {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_cancel=move || {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_delete=move || {
                                                        graph.update(|g| {
                                                            let _ = g.delete_table(node_idx);
                                                        });
                                                        // Send sync op
                                                        send_graph_op(GraphOperation::DeleteTable {
                                                            node_id: node_idx.index() as u32,
                                                        });
                                                        set_editing_mode.set(EditingMode::None);
                                                    }
                                                />
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                                EditingMode::CreatingTable => {
                                    // Режим создания новой таблицы
                                    view! {
                                        <div class="flex-1 flex flex-col overflow-hidden">
                                            // Хлебные крошки - вся панель кликабельна
                                            <button
                                                class="w-full px-6 py-3 border-b border-theme-primary bg-theme-secondary hover:bg-theme-tertiary theme-transition text-left cursor-pointer"
                                                on:click=move |_| set_editing_mode.set(EditingMode::None)
                                            >
                                                <div class="flex items-center text-sm text-theme-accent font-medium">
                                                    <Icon name=icons::CHEVRON_LEFT class="w-4 h-4 mr-1"/>
                                                    "Back to tables"
                                                </div>
                                                <div class="mt-1 text-xs text-theme-muted">
                                                    "Create New Table"
                                                </div>
                                            </button>

                                            // Диалог создания таблицы
                                            <div class="flex-1 overflow-y-auto px-6 py-4 bg-theme-surface theme-transition">
                                                <NewTableDialog
                                                    table_exists=Callback::new(move |name: String| {
                                                        graph.with(|g| g.table_exists(&name))
                                                    })
                                                    on_create=Callback::new(move |data: NewTableData| {
                                                        // Создаём таблицу с указанным именем
                                                        let position = (300.0, 300.0);
                                                        let table_name = data.table_name.clone();
                                                        let pk_name = data.pk_name.clone();
                                                        let pk_type = data.pk_type.clone();

                                                        // Создаём таблицу
                                                        let result = graph.write().create_table(&table_name, position);

                                                        match result {
                                                            Ok(new_node_idx) => {
                                                                // Добавляем первичный ключ
                                                                graph.update(|g| {
                                                                    if let Some(node) = g.node_weight_mut(new_node_idx) {
                                                                        node.columns.push(
                                                                            Column::new(&pk_name, &pk_type).primary_key()
                                                                        );
                                                                    }
                                                                });

                                                                // Отправляем операцию создания таблицы
                                                                send_graph_op(GraphOperation::CreateTable {
                                                                    node_id: new_node_idx.index() as u32,
                                                                    name: table_name,
                                                                    position,
                                                                });

                                                                // Отправляем операцию добавления первичного ключа
                                                                send_graph_op(GraphOperation::AddColumn {
                                                                    node_id: new_node_idx.index() as u32,
                                                                    column: ColumnData {
                                                                        name: pk_name,
                                                                        data_type: pk_type,
                                                                        is_primary_key: true,
                                                                        is_nullable: false,
                                                                        is_unique: false,
                                                                        default_value: None,
                                                                        foreign_key: None,
                                                                    },
                                                                });

                                                                // Раскрываем таблицу в списке
                                                                set_expanded_tables.update(|expanded| {
                                                                    if !expanded.contains(&new_node_idx) {
                                                                        expanded.push(new_node_idx);
                                                                    }
                                                                });

                                                                // Возвращаемся к списку таблиц
                                                                set_editing_mode.set(EditingMode::None);

                                                                CreateTableResult::Success
                                                            }
                                                            Err(err) => {
                                                                CreateTableResult::Error(err)
                                                            }
                                                        }
                                                    })
                                                    on_cancel=move || {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }
                                                />
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                                EditingMode::None => {
                                // Режим просмотра списка таблиц
                                view! {
                                    <div class="flex-1 flex flex-col overflow-hidden bg-theme-surface theme-transition">
                                        // Поиск
                                        <div class="px-6 py-4 border-b border-theme-primary">
                                            <div class="relative flex items-center">
                                                <div class="absolute left-3 pointer-events-none flex items-center justify-center">
                                                    <Icon name=icons::SEARCH class="icon-text text-theme-muted"/>
                                                </div>
                                                <input
                                                    type="text"
                                                    class="input-base input-sm"
                                                    placeholder="Search tables and columns..."
                                                    prop:value=move || search_query.get()
                                                    on:input=move |ev| {
                                                        set_search_query.set(event_target_value(&ev));
                                                    }
                                                />
                                            </div>
                                        </div>

                                        // Статистика
                                        <div class="px-6 py-3 bg-theme-secondary border-b border-theme-primary theme-transition">
                                            <div class="grid grid-cols-3 gap-3">
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-blue-500">
                                                        {move || total_tables.get()}
                                                    </div>
                                                    <div class="text-xs text-theme-muted mt-0.5">"Tables"</div>
                                                </div>
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-purple-500">
                                                        {move || total_columns.get()}
                                                    </div>
                                                    <div class="text-xs text-theme-muted mt-0.5">"Columns"</div>
                                                </div>
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-green-500">
                                                        {move || total_relations.get()}
                                                    </div>
                                                    <div class="text-xs text-theme-muted mt-0.5">"Relations"</div>
                                                </div>
                                            </div>
                                        </div>

                                        // Кнопка создания таблицы
                                        <div class="px-6 py-3 divider-bottom bg-theme-tertiary theme-transition">
                                            <button
                                                class="w-full px-4 py-3 btn-theme-primary rounded-lg text-sm font-semibold flex items-center justify-center shadow-sm transition-all"
                                                on:click=move |_| {
                                                    // Открываем диалог создания новой таблицы
                                                    set_editing_mode.set(EditingMode::CreatingTable);
                                                }
                                            >
                                                <Icon name=icons::PLUS class="icon-btn"/>
                                                "New Table"
                                            </button>
                                        </div>

                                        // Список таблиц
                                        <div class="flex-1 overflow-y-auto px-3 py-3">
                                            {move || {
                                                let query = search_query.get().to_lowercase();
                                                let expanded = expanded_tables.get();

                                                // Используем мемоизированные индексы и with вместо get
                                                node_indices.get()
                                                    .into_iter()
                                                    .filter_map(|node_idx| {
                                                        graph.with(|g| {
                                                            let node = g.node_weight(node_idx)?;
                                                            let table_matches = query.is_empty()
                                                                || node.name.to_lowercase().contains(&query);
                                                            let column_matches = !query.is_empty()
                                                                && node
                                                                    .columns
                                                                    .iter()
                                                                    .any(|col| {
                                                                        col.name.to_lowercase().contains(&query)
                                                                            || col.data_type.to_string().to_lowercase().contains(&query)
                                                                    });
                                                            if !table_matches && !column_matches {
                                                                return None;
                                                            }
                                                            Some((node_idx, node.clone()))
                                                        })
                                                    })
                                                    .map(|(node_idx, node)| {
                                                        let is_expanded = expanded.contains(&node_idx);
                                                        let query_clone = query.clone();
                                                        view! {
                                                            <div class="mb-2 rounded-xl border border-theme-primary overflow-hidden hover:border-theme-accent theme-transition bg-theme-surface">
                                                                // Заголовок таблицы
                                                                <div class="flex items-center justify-between px-4 py-3 bg-theme-secondary hover:bg-theme-tertiary theme-transition cursor-pointer group">
                                                                    <div
                                                                        class="flex items-center flex-1"
                                                                        on:click=move |_| {
                                                                            on_table_focus.run(node_idx);
                                                                            if !is_expanded {
                                                                                toggle_table(node_idx);
                                                                            }
                                                                        }
                                                                    >

                                                                        <button
                                                                            class="mr-2 text-theme-muted hover:text-theme-accent focus:outline-none transition-colors"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                toggle_table(node_idx);
                                                                            }
                                                                        >
                                                                            {if is_expanded {
                                                                                view! {
                                                                                    <Icon
                                                                                        name=icons::CHEVRON_DOWN
                                                                                        class="w-5 h-5 transition-transform"
                                                                                    />
                                                                                }
                                                                            } else {
                                                                                view! {
                                                                                    <Icon
                                                                                        name=icons::CHEVRON_RIGHT
                                                                                        class="w-5 h-5 transition-transform"
                                                                                    />
                                                                                }
                                                                            }}
                                                                        </button>

                                                                        <div class="w-8 h-8 rounded-lg flex items-center justify-center mr-3 shadow-sm" style="background: linear-gradient(to bottom right, var(--accent-primary), var(--accent-secondary));">
                                                                            <Icon name=icons::TABLE class="w-5 h-5 text-white"/>
                                                                        </div>

                                                                        <div class="flex-1 min-w-0">
                                                                            <div class="font-semibold text-theme-primary truncate">
                                                                                {node.name.clone()}
                                                                            </div>
                                                                            <div class="text-xs text-theme-muted">
                                                                                {node.columns.len()}
                                                                                " columns"
                                                                            </div>
                                                                        </div>
                                                                    </div>

                                                                    <div class="flex items-center space-x-1">
                                                                        <button
                                                                            class="p-1.5 text-theme-muted hover:text-purple-500 hover:bg-theme-tertiary rounded-lg transition-colors"
                                                                            title="Edit table"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_editing_mode.set(EditingMode::EditingTable(node_idx));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::SEARCH class="icon-text"/>
                                                                        </button>
                                                                        <button
                                                                            class="p-1.5 text-theme-muted hover:text-theme-accent hover:bg-theme-tertiary rounded-lg transition-colors"
                                                                            title="Add column"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_editing_mode
                                                                                    .set(EditingMode::EditingColumn(node_idx, None));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::X class="icon-text"/>
                                                                        </button>
                                                                    </div>
                                                                </div>

                                                                // Список колонок
                                                                {if is_expanded {
                                                                    view! {
                                                                        <div class="bg-theme-tertiary theme-transition">
                                                                            {if node.columns.is_empty() {
                                                                                view! {
                                                                                    <div class="px-4 py-6 text-center text-theme-muted text-sm">
                                                                                        "No columns yet"
                                                                                        <button
                                                                                            class="block mx-auto mt-2 text-theme-accent hover:opacity-80 font-medium"
                                                                                            on:click=move |_| {
                                                                                                set_editing_mode
                                                                                                    .set(EditingMode::EditingColumn(node_idx, None));
                                                                                            }
                                                                                        >

                                                                                            "+ Add first column"
                                                                                        </button>
                                                                                    </div>
                                                                                }
                                                                                    .into_any()
                                                                            } else {
                                                                                node
                                                                                    .columns
                                                                                    .iter()
                                                                                    .enumerate()
                                                                                    .filter(|(_, col)| {
                                                                                        query_clone.is_empty()
                                                                                            || col.name.to_lowercase().contains(&query_clone)
                                                                                            || col.data_type.to_lowercase().contains(&query_clone)
                                                                                    })
                                                                                    .map(|(col_idx, column)| {
                                                                                        view! {
                                                                                            <ColumnItem
                                                                                                column=column.clone()
                                                                                                on_click=move || {
                                                                                                    set_editing_mode
                                                                                                        .set(EditingMode::EditingColumn(node_idx, Some(col_idx)));
                                                                                                }
                                                                                            />
                                                                                        }
                                                                                    })
                                                                                    .collect_view()
                                                                                    .into_any()
                                                                            }}
                                                                        </div>
                                                                    }
                                                                        .into_any()
                                                                } else {
                                                                    view! { <div></div> }.into_any()
                                                                }}
                                                            </div>
                                                        }
                                                    })
                                                    .collect_view()
                                            }}
                                        </div>

                                        // Футер
                                        <div class="px-6 py-4 border-t border-theme-primary bg-theme-secondary theme-transition">
                                            <button
                                                class="w-full px-4 py-2.5 btn-theme-primary rounded-xl text-sm font-medium flex items-center justify-center shadow-sm transition-all"
                                                on:click=move |_| {
                                                    set_expanded_tables
                                                        .update(|expanded| {
                                                            let g = graph.with_untracked(|v| v.clone());
                                                            if expanded.len() == g.node_count() {
                                                                expanded.clear();
                                                            } else {
                                                                *expanded = g.node_indices().collect();
                                                            }
                                                        });
                                                }
                                            >
                                                {move || {
                                                    let g = graph.get();
                                                    let expanded = expanded_tables.get();
                                                    if expanded.len() == g.node_count() {
                                                        view! {
                                                            <>
                                                                <Icon name=icons::PLUS class="icon-text"/>
                                                                "Collapse All"
                                                            </>
                                                        }
                                                    } else {
                                                        view! {
                                                            <>
                                                                <Icon name=icons::EXPAND class="w-4 h-4 mr-2"/>
                                                                "Expand All"
                                                            </>
                                                        }
                                                    }
                                                }}
                                            </button>
                                        </div>
                                    </div>
                                }
                                    .into_any()
                                }
                            }
                        }}
                    </div>
                }.into_any()
            }
        }}


        </div>
    }
}
#[component]
fn ColumnItem(column: Column, #[prop(into)] on_click: Callback<()>) -> impl IntoView {
    view! {
        <div
            class="px-4 py-3 hover:bg-theme-secondary cursor-pointer border-b border-theme-primary last:border-b-0 theme-transition group"
            on:click=move |_| on_click.run(())
        >
            <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center">
                        {if column.is_primary_key {
                            view! {
                                <span class="inline-flex items-center px-2 py-0.5 rounded-md text-xs font-semibold bg-yellow-100 text-yellow-800 mr-2 border border-yellow-200 dark:bg-yellow-900 dark:text-yellow-200 dark:border-yellow-700">
                                    <Icon name=icons::KEY class="w-3 h-3 mr-1"/>
                                    "PK"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        <span class="font-medium text-theme-primary text-sm truncate group-hover:text-theme-accent transition-colors">
                            {column.name.clone()}
                        </span>
                    </div>
                    <div class="flex flex-wrap items-center gap-2 mt-1.5">
                        <code class="text-xs bg-theme-tertiary text-theme-secondary px-2 py-0.5 rounded-md font-mono border border-theme-primary">
                            {column.data_type.clone()}
                        </code>
                        {if !column.is_nullable {
                            view! {
                                <span class="inline-flex items-center text-xs font-semibold text-red-600 bg-red-50 px-2 py-0.5 rounded-md border border-red-200 dark:bg-red-900/30 dark:text-red-400 dark:border-red-800">
                                    "NOT NULL"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if column.is_unique {
                            view! {
                                <span class="inline-flex items-center text-xs font-semibold text-blue-600 bg-blue-50 px-2 py-0.5 rounded-md border border-blue-200 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-800">
                                    "UNIQUE"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>
                    {column
                        .default_value
                        .map(|def| {
                            view! {
                                <div class="mt-1.5 text-xs text-theme-tertiary">
                                    <span class="text-theme-muted">"DEFAULT: "</span>
                                    <code class="bg-purple-50 text-purple-700 px-1.5 py-0.5 rounded border border-purple-200 font-mono dark:bg-purple-900/30 dark:text-purple-400 dark:border-purple-800">
                                        {def}
                                    </code>
                                </div>
                            }
                        })}

                </div>
                <Icon
                    name=icons::CHEVRON_RIGHT
                    class="w-5 h-5 text-theme-muted group-hover:text-theme-accent ml-3 flex-shrink-0 transition-colors"
                />
            </div>
        </div>
    }
}
