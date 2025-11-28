use crate::core::SchemaGraph;
use crate::ui::sidebar::Sidebar;
use crate::ui::table::TableNodeView;
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;
use petgraph::graph::NodeIndex;

#[component]
pub fn SchemaCanvas(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    // Состояние для drag & drop
    let (_dragging_node, set_dragging_node) = signal::<Option<(NodeIndex, f64, f64)>>(None);

    // Мемоизация индексов узлов для предотвращения лишних пересчетов
    let node_indices = Memo::new(move |_| graph.with(|g| g.node_indices().collect::<Vec<_>>()));

    // Мемоизация индексов рёбер
    let edge_indices = Memo::new(move |_| graph.with(|g| g.edge_indices().collect::<Vec<_>>()));

    // Глобальный обработчик перемещения мыши
    #[cfg(not(feature = "ssr"))]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        // Храним closures в Rc<RefCell> для возможности их удаления
        let closures: Rc<
            RefCell<
                Option<(
                    Closure<dyn Fn(web_sys::MouseEvent)>,
                    Closure<dyn Fn(web_sys::MouseEvent)>,
                )>,
            >,
        > = Rc::new(RefCell::new(None));

        let closures_for_effect = closures.clone();

        Effect::new(move || {
            let dragging = dragging_node.get();

            // Сначала очищаем старые обработчики
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if let Some((old_move, old_up)) = closures_for_effect.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "mousemove",
                        old_move.as_ref().unchecked_ref(),
                    );
                    let _ = document.remove_event_listener_with_callback(
                        "mouseup",
                        old_up.as_ref().unchecked_ref(),
                    );
                }
            }

            // Если не тащим, просто выходим
            if dragging.is_none() {
                return;
            }

            let (node_idx, offset_x, offset_y) = dragging.unwrap();

            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");

            // Обработчик перемещения - используем batch для группировки обновлений
            let move_closure = Closure::new(move |ev: web_sys::MouseEvent| {
                ev.prevent_default();
                let new_x = ev.client_x() as f64 - offset_x;
                let new_y = ev.client_y() as f64 - offset_y;

                // Используем batch для минимизации реактивных пересчётов
                batch(move || {
                    graph.update(|g| {
                        if let Some(node) = g.node_weight_mut(node_idx) {
                            node.position = (new_x, new_y);
                        }
                    });
                });
            });

            // Обработчик отпускания кнопки
            let up_closure = Closure::new(move |_: web_sys::MouseEvent| {
                set_dragging_node.set(None);
            });

            document
                .add_event_listener_with_callback(
                    "mousemove",
                    move_closure.as_ref().unchecked_ref(),
                )
                .unwrap();

            document
                .add_event_listener_with_callback("mouseup", up_closure.as_ref().unchecked_ref())
                .unwrap();

            // Сохраняем closures для последующего удаления
            *closures_for_effect.borrow_mut() = Some((move_closure, up_closure));
        });
    }

    // Обработчик фокуса на таблице из сайдбара
    let handle_table_focus = move |_node_idx: NodeIndex| {
        // TODO: Центрировать таблицу на канвасе
    };

    view! {
        <div class="relative w-full h-screen bg-gray-50 overflow-hidden flex">
            // Сайдбар
            <Sidebar graph=graph on_table_focus=handle_table_focus/>

            // Основной канвас (со смещением из-за сайдбара)
            <div class="flex-1 ml-96 relative">
                // Сетка на фоне
                <div class="absolute inset-0 bg-grid-pattern opacity-20"></div>

                // Рендерим все узлы (таблицы) - используем мемоизированные индексы
                {move || {
                    node_indices.get()
                        .into_iter()
                        .filter_map(|idx| {
                            // Используем with вместо get для избежания клонирования всего графа
                            graph.with(|g| {
                                g.node_weight(idx).map(|node| {
                                    let node_clone = node.clone();
                                    let node_idx = idx;

                                    view! {
                                        <TableNodeView
                                            node=node_clone
                                            on_mouse_down=move |ev: web_sys::MouseEvent| {
                                                if ev.button() != 0 {
                                                    return;
                                                }
                                                ev.prevent_default();
                                                ev.stop_propagation();

                                                graph.with_untracked(|g| {
                                                    if let Some(n) = g.node_weight(node_idx) {
                                                        let (x, y) = n.position;
                                                        let offset_x = ev.client_x() as f64 - x;
                                                        let offset_y = ev.client_y() as f64 - y;
                                                        set_dragging_node.set(Some((node_idx, offset_x, offset_y)));
                                                    }
                                                });
                                            }
                                        />
                                    }
                                })
                            })
                        })
                        .collect_view()
                }}

                // SVG слой для отрисовки связей
                <svg class="absolute top-0 left-0 w-full h-full pointer-events-none">
                    <defs>
                        <marker
                            id="arrowhead"
                            markerWidth="10"
                            markerHeight="10"
                            refX="9"
                            refY="3"
                            orient="auto"
                        >
                            <polygon points="0 0, 10 3, 0 6" fill="#4B5563" />
                        </marker>
                    </defs>

                    // Рендерим связи - используем мемоизированные индексы
                    {move || {
                        edge_indices.get()
                            .into_iter()
                            .filter_map(|edge_idx| {
                                graph.with(|g| {
                                    let (from_idx, to_idx) = g.edge_endpoints(edge_idx)?;
                                    let from_node = g.node_weight(from_idx)?;
                                    let to_node = g.node_weight(to_idx)?;
                                    let edge = g.edge_weight(edge_idx)?;

                                    let (from_x, from_y) = from_node.position;
                                    let (to_x, to_y) = to_node.position;

                                    // Константы для расчёта позиции стрелок
                                    const NODE_WIDTH: f64 = 280.0;
                                    const HEADER_HEIGHT: f64 = 48.0;
                                    const ROW_HEIGHT: f64 = 36.0;
                                    const PADDING_TOP: f64 = 8.0;
                                    const GAP: f64 = 30.0;

                                    // Находим индекс колонки в исходной таблице
                                    let from_col_idx = from_node.columns.iter()
                                        .position(|col| col.name == edge.from_column)
                                        .unwrap_or(0);

                                    // Находим индекс колонки в целевой таблице
                                    let to_col_idx = to_node.columns.iter()
                                        .position(|col| col.name == edge.to_column)
                                        .unwrap_or(0);

                                    // Вычисляем Y координаты для конкретных колонок
                                    let from_col_y = from_y + HEADER_HEIGHT + PADDING_TOP
                                        + (from_col_idx as f64 * ROW_HEIGHT) + (ROW_HEIGHT / 2.0);
                                    let to_col_y = to_y + HEADER_HEIGHT + PADDING_TOP
                                        + (to_col_idx as f64 * ROW_HEIGHT) + (ROW_HEIGHT / 2.0);

                                    // Определяем границы таблиц
                                    let from_right = from_x + NODE_WIDTH;
                                    let from_left = from_x;
                                    let to_right = to_x + NODE_WIDTH;
                                    let to_left = to_x;

                                    // Умная логика выбора пути стрелки
                                    let (start_x, start_y, end_x, end_y, path_data) =
                                        calculate_edge_path(
                                            from_x, from_y, to_x, to_y,
                                            from_col_y, to_col_y,
                                            from_left, from_right, to_left, to_right,
                                            NODE_WIDTH, GAP
                                        );

                                    // Позиция для текста
                                    let text_x = (start_x + end_x) / 2.0 + 5.0;
                                    let text_y = (start_y + end_y) / 2.0;
                                    let rel_type = edge.relationship_type.to_string();

                                    Some(view! {
                                        <g>
                                            <path
                                                d=path_data
                                                stroke="#4B5563"
                                                stroke-width="2"
                                                fill="none"
                                                marker-end="url(#arrowhead)"
                                            />
                                            <text
                                                x=text_x
                                                y=text_y
                                                fill="#4B5563"
                                                font-size="12"
                                                text-anchor="start"
                                                class="select-none"
                                            >
                                                {rel_type}
                                            </text>
                                        </g>
                                    })
                                })
                            })
                            .collect_view()
                    }}
                </svg>

                // Панель инструментов (правый верхний угол)
                <div class="absolute top-4 right-4 bg-white/90 backdrop-blur-sm p-4 rounded-xl shadow-lg border border-gray-200">
                    <h3 class="font-bold text-gray-800 mb-2 flex items-center">
                        <Icon name=icons::LIGHTNING class="w-5 h-5 mr-2 text-blue-600"/>
                        "Quick Help"
                    </h3>
                    <div class="text-sm text-gray-600 space-y-1">
                        <p>"• Drag tables to move"</p>
                        <p>"• Use sidebar to edit columns"</p>
                        <p>"• Click table name to focus"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Вычисляет путь SVG для рёбра графа с оптимизированной логикой
#[inline]
fn calculate_edge_path(
    from_x: f64,
    _from_y: f64,
    to_x: f64,
    _to_y: f64,
    from_col_y: f64,
    to_col_y: f64,
    from_left: f64,
    from_right: f64,
    to_left: f64,
    to_right: f64,
    node_width: f64,
    gap: f64,
) -> (f64, f64, f64, f64, String) {
    if to_left >= from_right + 10.0 {
        // Целевая таблица ЧЁТКО справа - стандартный путь
        let start_x = from_right;
        let end_x = to_left;
        let mid_x = (start_x + end_x) / 2.0;

        let path = format!(
            "M {} {} L {} {} L {} {} L {} {}",
            start_x, from_col_y, mid_x, from_col_y, mid_x, to_col_y, end_x, to_col_y
        );
        (start_x, from_col_y, end_x, to_col_y, path)
    } else if from_left >= to_right + 10.0 {
        // Целевая таблица ЧЁТКО слева - зеркальный путь
        let start_x = from_left;
        let end_x = to_right;
        let mid_x = (start_x + end_x) / 2.0;

        let path = format!(
            "M {} {} L {} {} L {} {} L {} {}",
            start_x, from_col_y, mid_x, from_col_y, mid_x, to_col_y, end_x, to_col_y
        );
        (start_x, from_col_y, end_x, to_col_y, path)
    } else {
        // Таблицы перекрываются по X или расположены по диагонали
        let from_center_x = from_x + node_width / 2.0;
        let to_center_x = to_x + node_width / 2.0;

        if to_center_x > from_center_x {
            // Целевая таблица правее
            let start_x = from_right;
            let end_x = to_left;
            let max_right = from_right.max(to_right);
            let out_x = max_right + gap;

            let path = format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
            );
            (start_x, from_col_y, end_x, to_col_y, path)
        } else {
            // Целевая таблица левее
            let start_x = from_left;
            let end_x = to_right;
            let min_left = from_left.min(to_left);
            let out_x = min_left - gap;

            let path = format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
            );
            (start_x, from_col_y, end_x, to_col_y, path)
        }
    }
}
