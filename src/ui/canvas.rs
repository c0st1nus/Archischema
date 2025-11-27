use crate::core::SchemaGraph;
use crate::ui::table::TableNodeView;
use leptos::prelude::*;
use leptos::web_sys;
use petgraph::graph::NodeIndex;

#[component]
pub fn SchemaCanvas(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    // Состояние для drag & drop
    #[allow(unused_variables)]
    let (dragging_node, set_dragging_node) = signal::<Option<(NodeIndex, f64, f64)>>(None);

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
                    // Closures будут удалены здесь
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

            // Обработчик перемещения
            let move_closure = Closure::new(move |ev: web_sys::MouseEvent| {
                ev.prevent_default();
                let new_x = ev.client_x() as f64 - offset_x;
                let new_y = ev.client_y() as f64 - offset_y;

                graph.update(|g| {
                    if let Some(node) = g.node_weight_mut(node_idx) {
                        node.position = (new_x, new_y);
                    }
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

    view! {
        <div class="relative w-full h-screen bg-gray-50 overflow-hidden">
            // Сетка на фоне
            <div class="absolute inset-0 bg-grid-pattern opacity-20"></div>

            // Рендерим все узлы (таблицы)
            {move || {
                let g = graph.get();
                g.node_indices()
                    .filter_map(|idx| {
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

                                        let g = graph.get_untracked();
                                        if let Some(n) = g.node_weight(node_idx) {
                                            let (x, y) = n.position;
                                            let offset_x = ev.client_x() as f64 - x;
                                            let offset_y = ev.client_y() as f64 - y;
                                            set_dragging_node.set(Some((node_idx, offset_x, offset_y)));
                                        }
                                    }
                                />
                            }
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

                // Рендерим связи
                {move || {
                    let g = graph.get();
                    g.edge_indices()
                        .filter_map(|edge_idx| {
                            g.edge_endpoints(edge_idx).and_then(|(from_idx, to_idx)| {
                                let from_node = g.node_weight(from_idx)?;
                                let to_node = g.node_weight(to_idx)?;
                                let edge = g.edge_weight(edge_idx)?;

                                let (from_x, from_y) = from_node.position;
                                let (to_x, to_y) = to_node.position;

                                let node_width = 250.0;
                                let node_height = 100.0;

                                let center_from_x = from_x + node_width / 2.0;
                                let center_from_y = from_y + node_height / 2.0;
                                let center_to_x = to_x + node_width / 2.0;
                                let center_to_y = to_y + node_height / 2.0;

                                Some(view! {
                                    <g>
                                        <line
                                            x1=center_from_x
                                            y1=center_from_y
                                            x2=center_to_x
                                            y2=center_to_y
                                            stroke="#4B5563"
                                            stroke-width="2"
                                            marker-end="url(#arrowhead)"
                                        />
                                        <text
                                            x=(center_from_x + center_to_x) / 2.0
                                            y=(center_from_y + center_to_y) / 2.0 - 5.0
                                            fill="#4B5563"
                                            font-size="12"
                                            text-anchor="middle"
                                            class="select-none"
                                        >
                                            {edge.relationship_type.to_string()}
                                        </text>
                                    </g>
                                })
                            })
                        })
                        .collect_view()
                }}
            </svg>

            // Панель инструментов
            <div class="absolute top-4 right-4 bg-white p-4 rounded-lg shadow-lg">
                <h3 class="font-bold text-gray-700 mb-2">"Schema Editor"</h3>
                <div class="text-sm text-gray-600">
                    <p>"Drag tables to reposition"</p>
                    <p class="text-xs mt-1">"Total tables: " {move || graph.get().node_count()}</p>
                    <p class="text-xs">"Total relations: " {move || graph.get().edge_count()}</p>
                </div>
            </div>
        </div>
    }
}
