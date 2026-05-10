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
use petgraph::graph::NodeIndex;

#[derive(Clone, Debug, PartialEq)]
enum EditingMode {
    None,
    CreatingTable,
    EditingTable(NodeIndex),
}

#[component]
pub fn Sidebar(
    graph: RwSignal<SchemaGraph>,
    #[prop(into)] on_table_focus: Callback<NodeIndex>,
    /// Shared target for the floating column editor and canvas/sidebar highlight.
    column_editor_target: RwSignal<Option<(NodeIndex, Option<usize>)>>,
    /// Editor mode signal (Visual/Source)
    editor_mode: RwSignal<EditorMode>,
    /// Sidebar collapsed state (shared with parent for layout coordination)
    is_collapsed: RwSignal<bool>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    let set_is_collapsed = is_collapsed;
    let (search_query, set_search_query) = signal(String::new());
    let (expanded_tables, set_expanded_tables) = signal::<Vec<NodeIndex>>(Vec::new());

    // Состояние для редактора (колонка или таблица)
    let (editing_mode, set_editing_mode) = signal(EditingMode::None);
    let set_column_editor_target = column_editor_target;

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
                "editor-sidebar collapsed"
            } else {
                "editor-sidebar"
            }
        }>
        {move || {
            if is_collapsed.get() {
                // Свернутый вид
                view! {
                    <div class="flex h-full flex-col items-center gap-4 py-4">
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
                    <div class="flex h-full flex-col">
                        // Editor Mode Switcher
                        <div class="border-b border-theme-primary bg-theme-surface px-3 py-3 theme-transition">
                            <div class="flex items-center gap-2">
                                <EditorModeSwitcher mode=editor_mode />
                                <button
                                    class="btn-icon flex-shrink-0"
                                    on:click=move |_| set_is_collapsed.set(true)
                                    title="Collapse sidebar"
                                >
                                    <Icon name=icons::PANEL_LEFT_CLOSE class="icon-standalone"/>
                                </button>
                            </div>
                        </div>

                        {move || {
                            match editing_mode.get() {
                                EditingMode::EditingTable(_node_idx) => {
                                    // Режим редактирования таблицы - показываем обычный список таблиц
                                    // Сам диалог отрисовывается модально в конце компонента
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
                                                        class="input-base input-sm pl-10"
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
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-accent">
                                                            {move || total_tables.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Tables"</div>
                                                    </div>
                                                    <div class="text-center">
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-primary">
                                                            {move || total_columns.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Columns"</div>
                                                    </div>
                                                    <div class="text-center">
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-warning">
                                                            {move || total_relations.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Relations"</div>
                                                    </div>
                                                </div>
                                            </div>

                                            // Список таблиц (просмотр в фоне, диалог модальный)
                                            <div class="flex-1 overflow-y-auto px-3 py-3 opacity-50">
                                                <div class="text-center py-8 text-theme-muted">
                                                    "Editing table..."
                                                </div>
                                            </div>
                                        </div>
                                    }
                                        .into_any()
                                }
                                EditingMode::CreatingTable => {
                                    // Режим создания новой таблицы - показываем обычный список таблиц
                                    // Сам диалог отрисовывается модально в конце компонента
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
                                                        class="input-base input-sm pl-10"
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
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-accent">
                                                            {move || total_tables.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Tables"</div>
                                                    </div>
                                                    <div class="text-center">
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-primary">
                                                            {move || total_columns.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Columns"</div>
                                                    </div>
                                                    <div class="text-center">
                                                        <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-warning">
                                                            {move || total_relations.get()}
                                                        </div>
                                                        <div class="text-xs text-theme-muted mt-0.5">"Relations"</div>
                                                    </div>
                                                </div>
                                            </div>

                                            // Список таблиц (просмотр в фоне, диалог модальный)
                                            <div class="flex-1 overflow-y-auto px-3 py-3 opacity-50">
                                                <div class="text-center py-8 text-theme-muted">
                                                    "Creating new table..."
                                                </div>
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
                                        <div class="px-3 py-2">
                                            <div class="relative flex items-center">
                                                <div class="absolute left-3 pointer-events-none flex items-center justify-center">
                                                    <Icon name=icons::SEARCH class="icon-text text-theme-muted"/>
                                                </div>
                                                <input
                                                    type="text"
                                                    class="input-base input-sm pl-8 pr-12"
                                                    placeholder="Search tables and columns..."
                                                    prop:value=move || search_query.get()
                                                    on:input=move |ev| {
                                                        set_search_query.set(event_target_value(&ev));
                                                    }
                                                />
                                                <span class="kbd absolute right-2 top-1/2 -translate-y-1/2">"/"</span>
                                            </div>
                                        </div>

                                        // Статистика
                                        <div class="grid grid-cols-3 gap-px px-3 pb-2 theme-transition">
                                            <div class="schema-stat-tile">
                                                <div class="schema-stat-label">
                                                    <Icon name=icons::DATABASE class="h-3 w-3"/>"Tables"
                                                </div>
                                                <div class="schema-stat-value">
                                                    {move || total_tables.get()}
                                                </div>
                                            </div>
                                            <div class="schema-stat-tile">
                                                <div class="schema-stat-label">
                                                    <Icon name=icons::CODE class="h-3 w-3"/>"Columns"
                                                </div>
                                                <div class="schema-stat-value">
                                                    {move || total_columns.get()}
                                                </div>
                                            </div>
                                            <div class="schema-stat-tile">
                                                <div class="schema-stat-label">
                                                    <Icon name=icons::LIGHTNING class="h-3 w-3"/>"Relations"
                                                </div>
                                                <div class="schema-stat-value">
                                                    {move || total_relations.get()}
                                                </div>
                                            </div>
                                        </div>

                                        <div class="flex items-center justify-between px-3 py-1.5">
                                            <span class="eyebrow">"Tables · " {move || total_tables.get()}</span>
                                            <button
                                                type="button"
                                                class="btn-icon btn-sm"
                                                title="Expand or collapse all tables"
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
                                                <Icon name=icons::EXPAND class="h-3.5 w-3.5"/>
                                            </button>
                                        </div>

                                        // Список таблиц
                                        <div class="scroll flex-1 overflow-y-auto pb-2">
                                            {move || {
                                                let query = search_query.get().to_lowercase();
                                                let expanded = expanded_tables.get();
                                                let active_column = column_editor_target.get();

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
                                                            <div class="schema-list-card">
                                                                // Заголовок таблицы
                                                                <div class="schema-list-header cursor-pointer group">
                                                                    <div
                                                                        class="flex min-w-0 flex-1 items-center"
                                                                        on:click=move |_| {
                                                                            on_table_focus.run(node_idx);
                                                                            if !is_expanded {
                                                                                toggle_table(node_idx);
                                                                            }
                                                                        }
                                                                    >

                                                                        <button
                                                                            class="btn-icon btn-sm mr-2"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                toggle_table(node_idx);
                                                                            }
                                                                        >
                                                                            {if is_expanded {
                                                                                view! {
                                                                                    <Icon name=icons::CHEVRON_DOWN class="w-4 h-4 transition-transform" />
                                                                                }
                                                                            } else {
                                                                                view! {
                                                                                    <Icon name=icons::CHEVRON_RIGHT class="w-4 h-4 transition-transform" />
                                                                                }
                                                                            }}
                                                                        </button>

                                                                        <div class="mr-2 flex h-6 w-6 items-center justify-center rounded-md border border-theme bg-theme-accent-light text-theme-accent shadow-theme-sm">
                                                                            <Icon name=icons::TABLE class="w-3.5 h-3.5"/>
                                                                        </div>

                                                                        <div class="flex min-w-0 flex-1 items-center gap-2">
                                                                            <div class="min-w-0 truncate font-semibold text-theme-primary">
                                                                                {node.name.clone()}
                                                                            </div>
                                                                            <span class="ml-auto rounded border border-theme bg-theme-tertiary px-1.5 py-0.5 font-mono text-[10px] text-theme-muted">
                                                                                {node.columns.len()}" cols"
                                                                            </span>
                                                                        </div>
                                                                    </div>

                                                                    <div class="flex items-center gap-1">
                                                                        <button
                                                                            class="btn-icon btn-sm"
                                                                            title="Edit table"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_editing_mode.set(EditingMode::EditingTable(node_idx));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::EDIT class="w-3.5 h-3.5"/>
                                                                        </button>
                                                                        <button
                                                                            class="btn-icon btn-sm"
                                                                            title="Add column"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_column_editor_target.set(Some((node_idx, None)));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::PLUS class="w-3.5 h-3.5"/>
                                                                        </button>
                                                                    </div>
                                                                </div>

                                                                // Список колонок
                                                                {if is_expanded {
                                                                    view! {
                                                                            <div class="border-t border-theme bg-transparent theme-transition">
                                                                            {if node.columns.is_empty() {
                                                                                view! {
                                                                                    <div class="px-4 py-6 text-center text-sm text-theme-muted">
                                                                                        "No columns yet"
                                                                                        <button
                                                                                            class="btn-link mx-auto mt-2"
                                                                                            on:click=move |_| {
                                                                                                set_column_editor_target.set(Some((node_idx, None)));
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
                                                                                        let is_active = active_column == Some((node_idx, Some(col_idx)));
                                                                                        view! {
                                                                                            <ColumnItem
                                                                                                column=column.clone()
                                                                                                is_active=is_active
                                                                                                on_click=move || {
                                                                                                    set_column_editor_target.set(Some((node_idx, Some(col_idx))));
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
                                        <div class="border-t border-theme-primary bg-theme-secondary p-2.5 theme-transition">
                                            <button
                                                class="btn-primary btn-lg w-full"
                                                on:click=move |_| {
                                                    set_editing_mode.set(EditingMode::CreatingTable);
                                                }
                                            >
                                                <Icon name=icons::PLUS class="icon-btn"/>
                                                "New table"
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

            // NewTableDialog как модальное окно поверх всего
            <NewTableDialog
                is_open=Signal::derive(move || editing_mode.get() == EditingMode::CreatingTable)
                table_exists=Callback::new(move |name: String| {
                    graph.with(|g| g.table_exists(&name))
                })
                on_create=Callback::new(move |data: NewTableData| {
                    // Создаём таблицу с указанным именем
                    let position = (300.0, 300.0);
                    let table_name = data.table_name.clone();
                    let columns = data.columns.clone();

                    // Создаём таблицу
                    let result = graph.write().create_table(&table_name, position);

                    match result {
                        Ok(new_node_idx) => {
                            // Add the preset columns selected in the create-table dialog.
                            graph.update(|g| {
                                if let Some(node) = g.node_weight_mut(new_node_idx) {
                                    node.columns.extend(columns.clone());
                                }
                            });

                            // Отправляем операцию создания таблицы
                            let table_uuid = graph.with(|g| {
                                g.node_weight(new_node_idx).map(|n| n.uuid).unwrap_or_else(uuid::Uuid::new_v4)
                            });
                            send_graph_op(GraphOperation::CreateTable {
                                node_id: new_node_idx.index() as u32,
                                table_uuid,
                                name: table_name,
                                position,
                            });

                            // Отправляем операции добавления колонок
                            for column in columns {
                                send_graph_op(GraphOperation::AddColumn {
                                    node_id: new_node_idx.index() as u32,
                                    table_uuid,
                                    column: ColumnData {
                                        name: column.name,
                                        data_type: column.data_type,
                                        is_primary_key: column.is_primary_key,
                                        is_nullable: column.is_nullable,
                                        is_unique: column.is_unique,
                                        default_value: column.default_value,
                                        foreign_key: None,
                                    },
                                });
                            }

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

            // TableEditor как модальное окно поверх всего
            {move || {
                if let EditingMode::EditingTable(node_idx) = editing_mode.get() {
                    view! {
                        <TableEditor
                            is_open=Signal::derive(move || matches!(editing_mode.get(), EditingMode::EditingTable(_)))
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
                                let table_uuid = graph.with(|g| {
                                    g.node_weight(node_idx).map(|n| n.uuid).unwrap_or_else(uuid::Uuid::new_v4)
                                });
                                send_graph_op(GraphOperation::DeleteTable {
                                    node_id: node_idx.index() as u32,
                                    table_uuid,
                                });
                                set_editing_mode.set(EditingMode::None);
                            }
                            on_add_column=move |target_node_idx: NodeIndex| {
                                set_column_editor_target.set(Some((target_node_idx, None)));
                            }
                            on_edit_column=move |column_idx: usize| {
                                set_column_editor_target.set(Some((node_idx, Some(column_idx))));
                            }
                        />
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            {move || {
                if let Some((node_idx, col_idx)) = column_editor_target.get() {
                    let g = graph.get();
                    let node = g.node_weight(node_idx).cloned();
                    let column = col_idx.and_then(|idx| {
                        node.as_ref().and_then(|n| n.columns.get(idx).cloned())
                    });
                    let column_count = node.as_ref().map(|n| n.columns.len()).unwrap_or(0);
                    let can_previous = col_idx.is_some_and(|idx| idx > 0);
                    let can_next = col_idx.is_some_and(|idx| idx + 1 < column_count);

                    view! {
                        <ColumnEditor
                            column=column
                            column_index=col_idx
                            inline=false
                            graph=graph
                            current_table=node_idx
                            can_previous=can_previous
                            can_next=can_next
                            on_previous=Callback::new(move |_| {
                                if let Some(idx) = col_idx && idx > 0 {
                                    set_column_editor_target.set(Some((node_idx, Some(idx - 1))));
                                }
                            })
                            on_next=Callback::new(move |_| {
                                if let Some(idx) = col_idx && idx + 1 < column_count {
                                    set_column_editor_target.set(Some((node_idx, Some(idx + 1))));
                                }
                            })
                            column_name_exists=Callback::new(move |candidate: String| {
                                graph.with(|g| {
                                    g.node_weight(node_idx)
                                        .map(|node| {
                                            node.columns.iter().enumerate().any(|(idx, column)| {
                                                Some(idx) != col_idx && column.name == candidate
                                            })
                                        })
                                        .unwrap_or(false)
                                })
                            })
                            on_save=move || {
                                set_column_editor_target.set(None);
                            }

                            on_cancel=move || {
                                set_column_editor_target.set(None);
                            }

                            on_delete=move || {
                                if let Some(idx) = col_idx {
                                    graph
                                        .update(|g| {
                                            if let Some(node) = g.node_weight_mut(node_idx) && idx < node.columns.len() {
                                                node.columns.remove(idx);
                                            }
                                        });
                                    let table_uuid = graph.with(|g| {
                                        g.node_weight(node_idx).map(|n| n.uuid).unwrap_or_else(uuid::Uuid::new_v4)
                                    });
                                    send_graph_op(GraphOperation::DeleteColumn {
                                        node_id: node_idx.index() as u32,
                                        table_uuid,
                                        column_index: idx,
                                    });
                                }
                                set_column_editor_target.set(None);
                            }
                        />
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}
#[component]
fn ColumnItem(
    column: Column,
    #[prop(default = false)] is_active: bool,
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView {
    let column_name = column.name.clone();
    let column_type = column.data_type.clone();

    view! {
        <button
            type="button"
            class=if is_active { "schema-column-item group is-active" } else { "schema-column-item group" }
            aria-current=if is_active { "true" } else { "false" }
            title=format!("{} {}", column_name.clone(), column_type.clone())
            on:click=move |_| on_click.run(())
        >
            <div class="column-inline-row">
                <div class="column-inline-main">
                    <span class="column-inline-name group-hover:text-theme-accent">
                        {column_name.clone()}
                    </span>
                    <span class="column-inline-badges">
                        {if column.is_primary_key {
                            view! {
                                <span class="schema-table-badge schema-table-badge-pk" title="Primary key">
                                    "PK"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if !column.is_nullable {
                            view! {
                                <span class="schema-table-badge schema-table-badge-nn" title="Not null">
                                    "NN"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if column.is_unique {
                            view! {
                                <span class="schema-table-badge schema-table-badge-uq" title="Unique">
                                    "UQ"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </span>
                </div>
                <span class="column-inline-type" title=column_type.clone()>{column_type.clone()}</span>
            </div>
        </button>
    }
}
