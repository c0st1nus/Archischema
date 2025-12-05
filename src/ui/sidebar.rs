use crate::core::{Column, SchemaGraph, TableOps};
use crate::ui::column_editor::ColumnEditor;
use crate::ui::liveshare_client::{
    ColumnData, ConnectionState, GraphOperation, use_liveshare_context,
};
use crate::ui::table_editor::TableEditor;
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;
use petgraph::graph::NodeIndex;

#[derive(Clone, Debug, PartialEq)]
enum EditingMode {
    None,
    EditingColumn(NodeIndex, Option<usize>),
    EditingTable(NodeIndex),
}

#[component]
pub fn Sidebar(
    graph: RwSignal<SchemaGraph>,
    #[prop(into)] on_table_focus: Callback<NodeIndex>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    let (is_collapsed, set_is_collapsed) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (expanded_tables, set_expanded_tables) = signal::<Vec<NodeIndex>>(Vec::new());

    // Состояние для редактора (колонка или таблица)
    let (editing_mode, set_editing_mode) = signal(EditingMode::None);

    // Helper to send graph operation when connected
    let send_graph_op = move |op: GraphOperation| {
        if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
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
                "fixed left-0 top-0 h-screen w-14 bg-white border-r border-gray-200 shadow-lg z-20 transition-all duration-300"
            } else {
                "fixed left-0 top-0 h-screen w-96 bg-white border-r border-gray-200 shadow-xl z-20 transition-all duration-300"
            }
        }>
        {move || {
            if is_collapsed.get() {
                // Свернутый вид
                view! {
                    <div class="h-full flex flex-col items-center py-4">
                        <button
                            class="text-gray-600 hover:text-blue-600 hover:bg-blue-50 p-3 rounded-lg transition-colors"
                            on:click=move |_| set_is_collapsed.set(false)
                            title="Expand sidebar"
                        >
                            <Icon name=icons::MENU class="w-6 h-6"/>
                        </button>
                    </div>
                }
                    .into_any()
            } else {
                // Развернутый вид
                view! {
                    <div class="h-full flex flex-col">
                        // Заголовок
                        <div class="px-6 py-4 border-b border-gray-200 bg-gradient-to-r from-slate-50 to-white">
                            <div class="flex items-center justify-between">
                                <div class="flex items-center space-x-3">
                                    <div class="w-10 h-10 bg-blue-600 rounded-lg flex items-center justify-center">
                                        <Icon name=icons::TABLE class="w-6 h-6 text-white"/>
                                    </div>
                                    <div>
                                        <h2 class="text-lg font-bold text-gray-900">"Schema"</h2>
                                        <p class="text-xs text-gray-500">"Database Explorer"</p>
                                    </div>
                                </div>
                                <button
                                    class="text-gray-400 hover:text-gray-600 hover:bg-gray-100 p-2 rounded-lg transition-colors"
                                    on:click=move |_| set_is_collapsed.set(true)
                                    title="Collapse sidebar"
                                >
                                    <Icon name=icons::CHEVRON_LEFT class="w-5 h-5"/>
                                </button>
                            </div>
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
                                            <div class="px-6 py-3 border-b border-gray-200 bg-gray-50">
                                                <button
                                                    class="flex items-center text-sm text-blue-600 hover:text-blue-700 font-medium"
                                                    on:click=move |_| set_editing_mode.set(EditingMode::None)
                                                >
                                                    <Icon name=icons::CHEVRON_LEFT class="w-4 h-4 mr-1"/>
                                                    "Back to tables"
                                                </button>
                                                <div class="mt-1 text-xs text-gray-500">
                                                    <span class="font-medium text-gray-700">{table_name}</span>
                                                    {if col_idx.is_some() {
                                                        " → Edit Column"
                                                    } else {
                                                        " → New Column"
                                                    }}
                                                </div>
                                            </div>

                                            // Редактор колонки в сайдбаре
                                            <div class="flex-1 overflow-y-auto px-6 py-4">
                                                <ColumnEditor
                                                    column=column
                                                    inline=true
                                                    graph=graph
                                                    current_table=node_idx
                                                    on_save=move |new_column: Column| {
                                                        let col_data = ColumnData {
                                                            name: new_column.name.clone(),
                                                            data_type: new_column.data_type.to_string(),
                                                            is_primary_key: new_column.is_primary_key,
                                                            is_nullable: new_column.is_nullable,
                                                            is_unique: new_column.is_unique,
                                                            default_value: new_column.default_value.clone(),
                                                            foreign_key: None,
                                                        };
                                                        let is_update = col_idx.is_some();
                                                        let col_index = col_idx.unwrap_or(0);
                                                        graph
                                                            .update(|g| {
                                                                if let Some(node) = g.node_weight_mut(node_idx) {
                                                                    if let Some(idx) = col_idx {
                                                                        if idx < node.columns.len() {
                                                                            node.columns[idx] = new_column;
                                                                        }
                                                                    } else {
                                                                        node.columns.push(new_column);
                                                                    }
                                                                }
                                                            });
                                                        // Send sync op
                                                        if is_update {
                                                            send_graph_op(GraphOperation::UpdateColumn {
                                                                node_id: node_idx.index() as u32,
                                                                column_index: col_index,
                                                                column: col_data,
                                                            });
                                                        } else {
                                                            send_graph_op(GraphOperation::AddColumn {
                                                                node_id: node_idx.index() as u32,
                                                                column: col_data,
                                                            });
                                                        }
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_cancel=move |_| {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_delete=move |_| {
                                                        if let Some(idx) = col_idx {
                                                            graph
                                                                .update(|g| {
                                                                    if let Some(node) = g.node_weight_mut(node_idx) {
                                                                        if idx < node.columns.len() {
                                                                            node.columns.remove(idx);
                                                                        }
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
                                            <div class="px-6 py-3 border-b border-gray-200 bg-gray-50">
                                                <button
                                                    class="flex items-center text-sm text-blue-600 hover:text-blue-700 font-medium"
                                                    on:click=move |_| set_editing_mode.set(EditingMode::None)
                                                >
                                                    <Icon name=icons::CHEVRON_LEFT class="w-4 h-4 mr-1"/>
                                                    "Back to tables"
                                                </button>
                                                <div class="mt-1 text-xs text-gray-500">
                                                    "Edit Table"
                                                </div>
                                            </div>

                                            // Редактор таблицы в сайдбаре
                                            <div class="flex-1 overflow-y-auto px-6 py-4">
                                                <TableEditor
                                                    graph=graph
                                                    node_idx=node_idx
                                                    on_save=move |_| {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_cancel=move |_| {
                                                        set_editing_mode.set(EditingMode::None);
                                                    }

                                                    on_delete=move |_| {
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
                                EditingMode::None => {
                                // Режим просмотра списка таблиц
                                view! {
                                    <div class="flex-1 flex flex-col overflow-hidden">
                                        // Поиск
                                        <div class="px-6 py-4 border-b border-gray-200">
                                            <div class="relative">
                                                <input
                                                    type="text"
                                                    class="w-full pl-10 pr-4 py-2.5 bg-gray-50 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500 focus:bg-white transition-all text-sm"
                                                    placeholder="Search tables and columns..."
                                                    prop:value=move || search_query.get()
                                                    on:input=move |ev| {
                                                        set_search_query.set(event_target_value(&ev));
                                                    }
                                                />

                                                <div class="absolute left-3 top-3 pointer-events-none">
                                                    <Icon name=icons::SEARCH class="w-5 h-5 text-gray-400"/>
                                                </div>
                                            </div>
                                        </div>

                                        // Статистика
                                        <div class="px-6 py-3 bg-slate-50 border-b border-gray-200">
                                            <div class="grid grid-cols-3 gap-3">
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-blue-600">
                                                        {move || total_tables.get()}
                                                    </div>
                                                    <div class="text-xs text-gray-500 mt-0.5">"Tables"</div>
                                                </div>
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-purple-600">
                                                        {move || total_columns.get()}
                                                    </div>
                                                    <div class="text-xs text-gray-500 mt-0.5">"Columns"</div>
                                                </div>
                                                <div class="text-center">
                                                    <div class="text-2xl font-bold text-green-600">
                                                        {move || total_relations.get()}
                                                    </div>
                                                    <div class="text-xs text-gray-500 mt-0.5">"Relations"</div>
                                                </div>
                                            </div>
                                        </div>

                                        // Кнопка создания таблицы
                                        <div class="px-6 py-4 border-b border-gray-200 bg-white">
                                            <button
                                                class="w-full px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 text-sm font-semibold flex items-center justify-center shadow-sm transition-all"
                                                on:click=move |_| {
                                                    // Создаем новую таблицу в центре видимой области
                                                    let new_node_idx = graph.write().create_table_auto((300.0, 300.0));
                                                    // Send sync op
                                                    let name = graph.with(|g| {
                                                        g.node_weight(new_node_idx).map(|n| n.name.clone()).unwrap_or_default()
                                                    });
                                                    send_graph_op(GraphOperation::CreateTable {
                                                        node_id: new_node_idx.index() as u32,
                                                        name,
                                                        position: (300.0, 300.0),
                                                    });
                                                    // Открываем редактор для новой таблицы
                                                    set_editing_mode.set(EditingMode::EditingTable(new_node_idx));
                                                    // Раскрываем таблицу в списке
                                                    set_expanded_tables
                                                        .update(|expanded| {
                                                            if !expanded.contains(&new_node_idx) {
                                                                expanded.push(new_node_idx);
                                                            }
                                                        });
                                                }
                                            >
                                                <Icon name=icons::PLUS class="w-5 h-5 mr-2"/>
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
                                                            <div class="mb-2 rounded-xl border border-gray-200 overflow-hidden hover:border-blue-300 transition-colors bg-white">
                                                                // Заголовок таблицы
                                                                <div class="flex items-center justify-between px-4 py-3 bg-gradient-to-r from-gray-50 to-white hover:from-blue-50 hover:to-white transition-all cursor-pointer group">
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
                                                                            class="mr-2 text-gray-400 hover:text-blue-600 focus:outline-none transition-colors"
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

                                                                        <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-blue-600 rounded-lg flex items-center justify-center mr-3 shadow-sm">
                                                                            <Icon name=icons::TABLE class="w-5 h-5 text-white"/>
                                                                        </div>

                                                                        <div class="flex-1 min-w-0">
                                                                            <div class="font-semibold text-gray-900 truncate">
                                                                                {node.name.clone()}
                                                                            </div>
                                                                            <div class="text-xs text-gray-500">
                                                                                {node.columns.len()}
                                                                                " columns"
                                                                            </div>
                                                                        </div>
                                                                    </div>

                                                                    <div class="flex items-center space-x-1">
                                                                        <button
                                                                            class="p-1.5 text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded-lg transition-colors"
                                                                            title="Edit table"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_editing_mode.set(EditingMode::EditingTable(node_idx));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::EDIT class="w-4 h-4"/>
                                                                        </button>
                                                                        <button
                                                                            class="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                                                                            title="Add column"
                                                                            on:click=move |ev: web_sys::MouseEvent| {
                                                                                ev.stop_propagation();
                                                                                set_editing_mode
                                                                                    .set(EditingMode::EditingColumn(node_idx, None));
                                                                            }
                                                                        >
                                                                            <Icon name=icons::PLUS class="w-5 h-5"/>
                                                                        </button>
                                                                    </div>
                                                                </div>

                                                                // Список колонок
                                                                {if is_expanded {
                                                                    view! {
                                                                        <div class="bg-gray-50">
                                                                            {if node.columns.is_empty() {
                                                                                view! {
                                                                                    <div class="px-4 py-6 text-center text-gray-400 text-sm">
                                                                                        "No columns yet"
                                                                                        <button
                                                                                            class="block mx-auto mt-2 text-blue-600 hover:text-blue-700 font-medium"
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
                                                                                                on_click=move |_| {
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
                                        <div class="px-6 py-4 border-t border-gray-200 bg-gradient-to-r from-slate-50 to-white">
                                            <button
                                                class="w-full px-4 py-2.5 bg-gradient-to-r from-blue-600 to-blue-700 text-white rounded-xl hover:from-blue-700 hover:to-blue-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 text-sm font-medium flex items-center justify-center shadow-sm transition-all"
                                                on:click=move |_| {
                                                    set_expanded_tables
                                                        .update(|expanded| {
                                                            let g = graph.get_untracked();
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
                                                                <Icon name=icons::COLLAPSE class="w-4 h-4 mr-2"/>
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
                }
                    .into_any()
            }
        }}

        </div>
    }
}

#[component]
fn ColumnItem(column: Column, #[prop(into)] on_click: Callback<()>) -> impl IntoView {
    view! {
        <div
            class="px-4 py-3 hover:bg-blue-50 cursor-pointer border-b border-gray-200 last:border-b-0 transition-colors group"
            on:click=move |_| on_click.run(())
        >
            <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center">
                        {if column.is_primary_key {
                            view! {
                                <span class="inline-flex items-center px-2 py-0.5 rounded-md text-xs font-semibold bg-yellow-100 text-yellow-800 mr-2 border border-yellow-200">
                                    <Icon name=icons::KEY class="w-3 h-3 mr-1"/>
                                    "PK"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        <span class="font-medium text-gray-900 text-sm truncate group-hover:text-blue-700 transition-colors">
                            {column.name.clone()}
                        </span>
                    </div>
                    <div class="flex flex-wrap items-center gap-2 mt-1.5">
                        <code class="text-xs bg-gray-100 text-gray-700 px-2 py-0.5 rounded-md font-mono border border-gray-200">
                            {column.data_type.clone()}
                        </code>
                        {if !column.is_nullable {
                            view! {
                                <span class="inline-flex items-center text-xs font-semibold text-red-600 bg-red-50 px-2 py-0.5 rounded-md border border-red-200">
                                    "NOT NULL"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if column.is_unique {
                            view! {
                                <span class="inline-flex items-center text-xs font-semibold text-blue-600 bg-blue-50 px-2 py-0.5 rounded-md border border-blue-200">
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
                                <div class="mt-1.5 text-xs text-gray-600">
                                    <span class="text-gray-400">"DEFAULT: "</span>
                                    <code class="bg-purple-50 text-purple-700 px-1.5 py-0.5 rounded border border-purple-200 font-mono">
                                        {def}
                                    </code>
                                </div>
                            }
                        })}

                </div>
                <Icon
                    name=icons::CHEVRON_RIGHT
                    class="w-5 h-5 text-gray-300 group-hover:text-blue-600 ml-3 flex-shrink-0 transition-colors"
                />
            </div>
        </div>
    }
}
