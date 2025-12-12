use crate::core::{SchemaGraph, TableOps, auto_layout, create_demo_graph};
use crate::ui::ai_chat::{AiChatButton, AiChatPanel};
#[cfg(not(feature = "ssr"))]
use crate::ui::liveshare_client::{
    ColumnData, GraphStateSnapshot, RelationshipData, RelationshipSnapshot, TableSnapshot,
};
use crate::ui::liveshare_client::{ConnectionState, GraphOperation, use_liveshare_context};
use crate::ui::remote_cursors::{CursorTracker, RemoteCursors};
use crate::ui::settings_modal::{SettingsButton, SettingsModal};
use crate::ui::sidebar::Sidebar;
use crate::ui::source_editor::{EditorMode, SourceEditor};
use crate::ui::table::TableNodeView;
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::{html, web_sys};
use petgraph::graph::{EdgeIndex, NodeIndex};
#[cfg(not(feature = "ssr"))]
use std::collections::HashMap;
use std::collections::HashSet;

#[component]
pub fn SchemaCanvas(graph: RwSignal<SchemaGraph>) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    // Состояние для drag & drop
    let (_dragging_node, set_dragging_node) = signal::<Option<(NodeIndex, f64, f64)>>(None);

    // Track which nodes are currently being remotely dragged (for UI purposes)
    // This is a RwSignal so it can be used in reactive contexts
    let remote_dragging_nodes = RwSignal::new(std::collections::HashSet::<u32>::new());

    // Состояние для трансформации канваса (zoom и pan)
    #[allow(unused_variables)]
    let (zoom, set_zoom) = signal(1.0_f64);
    #[allow(unused_variables)]
    let (pan_x, set_pan_x) = signal(0.0_f64);
    #[allow(unused_variables)]
    let (pan_y, set_pan_y) = signal(0.0_f64);

    // Состояние для панорамирования средней кнопкой мыши
    #[allow(unused_variables)]
    let (panning, set_panning) = signal::<Option<(f64, f64)>>(None);

    // Ссылка на элемент канваса для добавления обработчиков событий
    let canvas_ref = NodeRef::<html::Div>::new();

    // Editor mode (Visual or Source)
    let editor_mode = RwSignal::new(EditorMode::Visual);

    // State for highlighted edges (when clicking on edge or table)
    let highlighted_edges: RwSignal<HashSet<EdgeIndex>> = RwSignal::new(HashSet::new());
    // State for selected table (to highlight all its edges)
    let selected_table: RwSignal<Option<NodeIndex>> = RwSignal::new(None);
    // Track if mouse moved during drag (to prevent selection on drag)
    let was_dragged: RwSignal<bool> = RwSignal::new(false);

    // Мемоизация индексов узлов для предотвращения лишних пересчетов
    let node_indices = Memo::new(move |_| graph.with(|g| g.node_indices().collect::<Vec<_>>()));

    // Мемоизация индексов рёбер
    let edge_indices = Memo::new(move |_| graph.with(|g| g.edge_indices().collect::<Vec<_>>()));

    // Listen for remote graph operations from LiveShare
    // NOTE: We use handler.forget() here which technically leaks memory, but:
    // 1. These are global event listeners that live for the entire app lifetime
    // 2. Leptos's on_cleanup requires Send+Sync which JS closures don't implement
    // 3. In WASM single-threaded context, this is acceptable for long-lived handlers
    #[cfg(not(feature = "ssr"))]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        // Setup smooth interpolation for remote table movements
        // Using Rc<RefCell> for internal state that doesn't need to cross reactive boundaries
        let remote_drag_targets: std::rc::Rc<std::cell::RefCell<HashMap<u32, (f64, f64, f64)>>> =
            std::rc::Rc::new(std::cell::RefCell::new(HashMap::new()));
        let animation_running: std::rc::Rc<std::cell::RefCell<bool>> =
            std::rc::Rc::new(std::cell::RefCell::new(false));

        // Animation function that will be called recursively
        fn start_animation_loop(
            targets: std::rc::Rc<std::cell::RefCell<HashMap<u32, (f64, f64, f64)>>>,
            animation_running: std::rc::Rc<std::cell::RefCell<bool>>,
            remote_dragging_signal: RwSignal<std::collections::HashSet<u32>>,
            graph: RwSignal<SchemaGraph>,
        ) {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::closure::Closure;

            // Check if already running
            if *animation_running.borrow() {
                return;
            }
            *animation_running.borrow_mut() = true;

            fn animate(
                targets: std::rc::Rc<std::cell::RefCell<HashMap<u32, (f64, f64, f64)>>>,
                animation_running: std::rc::Rc<std::cell::RefCell<bool>>,
                remote_dragging_signal: RwSignal<std::collections::HashSet<u32>>,
                graph: RwSignal<SchemaGraph>,
            ) {
                use wasm_bindgen::JsCast;
                use wasm_bindgen::closure::Closure;

                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        *animation_running.borrow_mut() = false;
                        return;
                    }
                };

                let now = js_sys::Date::now();
                let mut has_active_animations = false;
                let lerp_factor = 0.25; // Smooth interpolation factor (higher = faster catch-up)

                // Get current targets and update positions
                let updates: Vec<(u32, f64, f64, bool)> = {
                    let targets_ref = targets.borrow();
                    targets_ref
                        .iter()
                        .map(|(&node_id, &(target_x, target_y, last_update))| {
                            // Check if stale (no update for 200ms = drag ended)
                            let is_stale = now - last_update > 200.0;
                            (node_id, target_x, target_y, is_stale)
                        })
                        .collect()
                };

                // Apply smooth interpolation and track which nodes are done
                let mut nodes_to_remove: Vec<u32> = Vec::new();
                for (node_id, target_x, target_y, is_stale) in updates {
                    let is_done = graph
                        .try_update(|g| {
                            let idx = NodeIndex::new(node_id as usize);
                            if let Some(node) = g.node_weight_mut(idx) {
                                let (current_x, current_y) = node.position;
                                let dx = target_x - current_x;
                                let dy = target_y - current_y;

                                // If close enough, snap to target
                                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                                    node.position = (target_x, target_y);
                                    true // Done with this node
                                } else {
                                    // Lerp towards target
                                    node.position = (
                                        current_x + dx * lerp_factor,
                                        current_y + dy * lerp_factor,
                                    );
                                    false // Still animating
                                }
                            } else {
                                true // Node doesn't exist, remove from tracking
                            }
                        })
                        .unwrap_or(true);

                    // Remove if stale AND animation complete
                    if is_stale && is_done {
                        nodes_to_remove.push(node_id);
                    } else {
                        has_active_animations = true;
                    }
                }

                // Clean up completed/stale targets
                if !nodes_to_remove.is_empty() {
                    let mut targets_mut = targets.borrow_mut();
                    for node_id in nodes_to_remove {
                        targets_mut.remove(&node_id);
                    }
                    // Update the signal with current dragging nodes
                    let current_nodes: std::collections::HashSet<u32> =
                        targets_mut.keys().cloned().collect();
                    remote_dragging_signal.set(current_nodes);
                }

                // Continue animation if there are active targets
                if has_active_animations {
                    let targets_next = targets.clone();
                    let running_next = animation_running.clone();
                    let closure = Closure::once(move || {
                        animate(targets_next, running_next, remote_dragging_signal, graph);
                    });
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                    closure.forget();
                } else {
                    *animation_running.borrow_mut() = false;
                    // Clear the signal when animation stops
                    remote_dragging_signal.set(std::collections::HashSet::new());
                }
            }

            // Start the animation loop
            let targets_clone = targets.clone();
            let running_clone = animation_running.clone();
            let closure = Closure::once(move || {
                animate(targets_clone, running_clone, remote_dragging_signal, graph);
            });
            if let Some(window) = web_sys::window() {
                let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
            }
            closure.forget();
        }

        // Handler for individual graph operations
        let targets_for_handler = remote_drag_targets.clone();
        let animation_running_for_handler = animation_running.clone();
        Effect::new(move |_| {
            let window = web_sys::window().expect("no window");

            let graph_clone = graph;
            let targets = targets_for_handler.clone();
            let anim_running = animation_running_for_handler.clone();
            let handler =
                Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                    if let Some(detail) = e.detail().as_string() {
                        if let Ok(op) = serde_json::from_str::<GraphOperation>(&detail) {
                            leptos::logging::log!("Applying remote graph op: {:?}", op);
                            // For MoveTable, use interpolation instead of direct update
                            if let GraphOperation::MoveTable { node_id, position } = &op {
                                let now = js_sys::Date::now();
                                targets
                                    .borrow_mut()
                                    .insert(*node_id, (position.0, position.1, now));
                                // Update signal to mark this node as being remotely dragged
                                remote_dragging_nodes.update(|set| {
                                    set.insert(*node_id);
                                });
                                // Start animation loop if not already running
                                start_animation_loop(
                                    targets.clone(),
                                    anim_running.clone(),
                                    remote_dragging_nodes,
                                    graph_clone,
                                );
                            } else {
                                apply_remote_graph_op(graph_clone, op);
                            }
                        }
                    }
                });

            let _ = window.add_event_listener_with_callback(
                "liveshare-graph-op",
                handler.as_ref().unchecked_ref(),
            );
            // Intentionally leaked - global listener for app lifetime
            handler.forget();
        });

        // Handler for full graph state (initial sync when joining room)
        Effect::new(move |_| {
            let window = web_sys::window().expect("no window");

            let graph_clone = graph;
            let handler =
                Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                    if let Some(detail) = e.detail().as_string() {
                        if let Ok(state) = serde_json::from_str::<GraphStateSnapshot>(&detail) {
                            leptos::logging::log!(
                                "Applying graph state: {} tables, {} relationships",
                                state.tables.len(),
                                state.relationships.len()
                            );
                            apply_graph_state(graph_clone, state);
                        }
                    }
                });

            let _ = window.add_event_listener_with_callback(
                "liveshare-graph-state",
                handler.as_ref().unchecked_ref(),
            );
            // Intentionally leaked - global listener for app lifetime
            handler.forget();
        });

        // Handler for graph state requests from other users
        Effect::new(move |_| {
            let window = web_sys::window().expect("no window");

            let graph_clone = graph;
            let ctx_clone = liveshare_ctx;
            let handler =
                Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                    if let Some(detail) = e.detail().as_string() {
                        // Parse requester_id from the detail
                        if let Ok(requester_id) = uuid::Uuid::parse_str(&detail) {
                            // Create snapshot of current graph state
                            let state = create_graph_snapshot(graph_clone);
                            leptos::logging::log!(
                                "Sending graph state to {:?}: {} tables",
                                requester_id,
                                state.tables.len()
                            );
                            // Send it to the requester
                            ctx_clone.send_graph_state_response(requester_id, state);
                        }
                    }
                });

            let _ = window.add_event_listener_with_callback(
                "liveshare-request-graph-state",
                handler.as_ref().unchecked_ref(),
            );
            // Intentionally leaked - global listener for app lifetime
            handler.forget();
        });
    }

    // Helper function to send graph op when connected
    let send_graph_op = move |op: GraphOperation| {
        if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
            liveshare_ctx.send_graph_op(op);
        }
    };

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
            let dragging = _dragging_node.get();

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

            // Throttle for live sync during drag (send every 50ms)
            let last_sync_time = std::rc::Rc::new(std::cell::RefCell::new(0.0_f64));
            let last_sync_time_clone = last_sync_time.clone();

            // Обработчик перемещения - используем batch для группировки обновлений
            let move_closure = Closure::new(move |ev: web_sys::MouseEvent| {
                ev.prevent_default();

                // Учитываем трансформацию канваса при перетаскивании
                let current_zoom = zoom.get_untracked();
                let current_pan_x = pan_x.get_untracked();
                let current_pan_y = pan_y.get_untracked();

                let new_x = (ev.client_x() as f64 - offset_x - current_pan_x) / current_zoom;
                let new_y = (ev.client_y() as f64 - offset_y - current_pan_y) / current_zoom;

                // Mark that we've moved (dragged)
                was_dragged.set(true);

                // Используем batch для минимизации реактивных пересчётов
                batch(move || {
                    graph.update(|g| {
                        if let Some(node) = g.node_weight_mut(node_idx) {
                            node.position = (new_x, new_y);
                        }
                    });
                });

                // Send live sync update with throttling (every 50ms)
                let now = js_sys::Date::now();
                let last = *last_sync_time_clone.borrow();
                if now - last >= 50.0 {
                    *last_sync_time_clone.borrow_mut() = now;
                    if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected
                    {
                        liveshare_ctx.send_graph_op(GraphOperation::MoveTable {
                            node_id: node_idx.index() as u32,
                            position: (new_x, new_y),
                        });
                    }
                }
            });

            // Обработчик отпускания кнопки - отправляем sync при завершении перетаскивания
            let up_closure = Closure::new(move |_: web_sys::MouseEvent| {
                // Get final position and send sync op
                let final_pos =
                    graph.with_untracked(|g| g.node_weight(node_idx).map(|n| n.position));
                if let Some(position) = final_pos {
                    if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected
                    {
                        liveshare_ctx.send_graph_op(GraphOperation::MoveTable {
                            node_id: node_idx.index() as u32,
                            position,
                        });
                    }
                }
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

    // Обработчики для зума и панорамирования
    #[cfg(not(feature = "ssr"))]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        // Closures для панорамирования средней кнопкой
        let pan_closures: Rc<
            RefCell<
                Option<(
                    Closure<dyn Fn(web_sys::MouseEvent)>,
                    Closure<dyn Fn(web_sys::MouseEvent)>,
                )>,
            >,
        > = Rc::new(RefCell::new(None));
        let pan_closures_for_effect = pan_closures.clone();

        // Эффект для настройки обработчиков на элементе канваса
        Effect::new(move || {
            let Some(canvas_element) = canvas_ref.get() else {
                return;
            };

            // Обработчик колеса мыши для зума
            let wheel_handler =
                Closure::<dyn Fn(web_sys::WheelEvent)>::new(move |ev: web_sys::WheelEvent| {
                    if ev.ctrl_key() {
                        ev.prevent_default();
                        ev.stop_propagation();

                        let delta = ev.delta_y();
                        let zoom_factor = if delta < 0.0 { 1.1 } else { 0.9 };

                        set_zoom.update(|z| {
                            let new_zoom = (*z * zoom_factor).clamp(0.1, 5.0);
                            *z = new_zoom;
                        });
                    }
                });

            // Добавляем обработчик wheel с опцией passive: false
            let options = web_sys::AddEventListenerOptions::new();
            options.set_passive(false);

            canvas_element
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "wheel",
                    wheel_handler.as_ref().unchecked_ref(),
                    &options,
                )
                .unwrap();

            // Intentionally leaked - canvas element listener for app lifetime
            wheel_handler.forget();
        });

        // Эффект для обработки панорамирования средней кнопкой
        Effect::new(move || {
            let panning_state = panning.get();

            let document = web_sys::window()
                .and_then(|w| w.document())
                .expect("no document");

            // Удаляем старые обработчики
            if let Some((old_move, old_up)) = pan_closures_for_effect.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "mousemove",
                    old_move.as_ref().unchecked_ref(),
                );
                let _ = document.remove_event_listener_with_callback(
                    "mouseup",
                    old_up.as_ref().unchecked_ref(),
                );
            }

            if panning_state.is_none() {
                return;
            }

            let (start_x, start_y) = panning_state.unwrap();
            let initial_pan_x = pan_x.get_untracked();
            let initial_pan_y = pan_y.get_untracked();

            // Обработчик перемещения мыши
            let move_closure = Closure::new(move |ev: web_sys::MouseEvent| {
                ev.prevent_default();

                let dx = ev.client_x() as f64 - start_x;
                let dy = ev.client_y() as f64 - start_y;

                set_pan_x.set(initial_pan_x + dx);
                set_pan_y.set(initial_pan_y + dy);
            });

            // Обработчик отпускания кнопки
            let up_closure = Closure::new(move |_: web_sys::MouseEvent| {
                set_panning.set(None);
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

            *pan_closures_for_effect.borrow_mut() = Some((move_closure, up_closure));
        });

        // Обработчик клавиатуры для зума (Ctrl + "+"/"-")
        Effect::new(move || {
            let document = web_sys::window()
                .and_then(|w| w.document())
                .expect("no document");

            let keyboard_handler = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    // Проверяем Ctrl + "+" или Ctrl + "="
                    if ev.ctrl_key() && (ev.key() == "+" || ev.key() == "=") {
                        ev.prevent_default();
                        set_zoom.update(|z| {
                            let new_zoom = (*z * 1.1).clamp(0.1, 5.0);
                            *z = new_zoom;
                        });
                    }
                    // Проверяем Ctrl + "-"
                    else if ev.ctrl_key() && ev.key() == "-" {
                        ev.prevent_default();
                        set_zoom.update(|z| {
                            let new_zoom = (*z * 0.9).clamp(0.1, 5.0);
                            *z = new_zoom;
                        });
                    }
                },
            );

            document
                .add_event_listener_with_callback(
                    "keydown",
                    keyboard_handler.as_ref().unchecked_ref(),
                )
                .unwrap();

            // Intentionally leaked - global keyboard listener for app lifetime
            keyboard_handler.forget();
        });
    }

    view! {
        <div class="relative w-full h-screen bg-theme-canvas overflow-hidden flex theme-transition">
            // Сайдбар
            <Sidebar graph=graph on_table_focus=handle_table_focus editor_mode=editor_mode/>

            // Source Editor (показывается в режиме Source)
            <Show when=move || editor_mode.get() == EditorMode::Source>
                <div class="flex-1 ml-96">
                    <SourceEditor graph=graph readonly=false />
                </div>
            </Show>

            // Основной канвас (со смещением из-за сайдбара) - показывается в режиме Visual
            <div
                node_ref=canvas_ref
                class=move || {
                    if editor_mode.get() == EditorMode::Visual {
                        "flex-1 ml-96 relative bg-theme-canvas theme-transition"
                    } else {
                        "hidden"
                    }
                }
                on:mousedown=move |ev: web_sys::MouseEvent| {
                    // Средняя кнопка мыши (button = 1)
                    if ev.button() == 1 {
                        ev.prevent_default();
                        ev.stop_propagation();
                        set_panning.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                    }
                }
                on:click=move |ev: web_sys::MouseEvent| {
                    // Clear selection when clicking on empty canvas (not on table or edge)
                    // The event target should be the canvas itself or the grid
                    if ev.button() == 0 {
                        // Only clear if we didn't click on a table or edge (they stop propagation)
                        highlighted_edges.set(HashSet::new());
                        selected_table.set(None);
                    }
                }
                on:contextmenu=move |ev: web_sys::MouseEvent| {
                    // Отключаем контекстное меню при клике средней кнопкой
                    ev.prevent_default();
                }
            >
                // Сетка на фоне
                <div class="absolute inset-0 bg-grid-pattern opacity-20"></div>

                // SVG слой для отрисовки связей (ПОД таблицами)
                <svg class="absolute top-0 left-0 w-full h-full" style="z-index: 1;">
                    <defs>
                        <marker
                            id="arrowhead"
                            markerWidth="10"
                            markerHeight="10"
                            refX="9"
                            refY="3"
                            orient="auto"
                        >
                            <polygon points="0 0, 10 3, 0 6" class="fill-current text-gray-500 dark:text-gray-400" />
                        </marker>
                    </defs>

                    <g
                        transform=move || format!(
                            "translate({}, {}) scale({})",
                            pan_x.get(),
                            pan_y.get(),
                            zoom.get()
                        )
                    >
                        // Рендерим связи - используем мемоизированные индексы
                        {move || {
                        let current_highlighted = highlighted_edges.get();
                        let current_selected_table = selected_table.get();

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
                                    let (_start_x, _start_y, _end_x, _end_y, text_x, text_y, path_data) =
                                        calculate_edge_path(
                                            from_x, from_y, to_x, to_y,
                                            from_col_y, to_col_y,
                                            from_left, from_right, to_left, to_right,
                                            NODE_WIDTH, GAP
                                        );

                                    let rel_type = edge.relationship_type.to_string();

                                    // Check if this edge should be highlighted (skip rendering here if highlighted - will be in top layer)
                                    let is_highlighted = current_highlighted.contains(&edge_idx)
                                        || current_selected_table.map(|t| t == from_idx || t == to_idx).unwrap_or(false);

                                    // Clone path_data for the invisible click target
                                    let path_data_clone = path_data.clone();

                                    Some(view! {
                                        <g>
                                            // Invisible wider path for easier clicking
                                            <path
                                                d=path_data_clone
                                                stroke="transparent"
                                                stroke-width="15"
                                                fill="none"
                                                style="cursor: pointer;"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    // Toggle this edge highlight
                                                    let mut new_set = HashSet::new();
                                                    new_set.insert(edge_idx);
                                                    highlighted_edges.set(new_set);
                                                    selected_table.set(None);
                                                }
                                            />
                                            // Visible path (dimmed if highlighted, since highlighted version is on top layer)
                                            <path
                                                d=path_data
                                                class="stroke-current text-gray-500 dark:text-gray-400"
                                                stroke-width="2"
                                                fill="none"
                                                marker-end="url(#arrowhead)"
                                                style="pointer-events: none;"
                                                style:opacity=if is_highlighted { "0.3" } else { "1" }
                                            />
                                            <text
                                                x=text_x
                                                y=text_y
                                                class="fill-current text-gray-500 dark:text-gray-400 select-none"
                                                font-size="12"
                                                text-anchor="start"
                                                style="pointer-events: none;"
                                                style:opacity=if is_highlighted { "0.3" } else { "1" }
                                            >
                                                {rel_type}
                                            </text>
                                        </g>
                                    })
                                })
                            })
                            .collect_view()
                        }}
                    </g>
                </svg>

                // Контейнер с трансформацией для зума и панорамирования (таблицы НАД связями)
                <div
                    style:transform=move || format!(
                        "translate({}px, {}px) scale({})",
                        pan_x.get(),
                        pan_y.get(),
                        zoom.get()
                    )
                    style:transform-origin="0 0"
                    style:transition="none"
                    style:z-index="2"
                    class="absolute top-0 left-0"
                >
                    // Рендерим все узлы (таблицы) - используем мемоизированные индексы
                {move || {
                    let current_dragging = _dragging_node.get();
                    // Get which tables are being remotely dragged (for disabling CSS transitions)
                    let current_remote_dragging = remote_dragging_nodes.get();
                    let current_selected_table = selected_table.get();

                    node_indices.get()
                        .into_iter()
                        .filter_map(|idx| {
                            // Используем with вместо get для избежания клонирования всего графа
                            graph.with(|g| {
                                g.node_weight(idx).map(|node| {
                                    let node_clone = node.clone();
                                    let node_idx = idx;
                                    // Check if this specific node is being dragged locally or remotely
                                    let is_local_dragging = current_dragging.map(|(drag_idx, _, _)| drag_idx == idx).unwrap_or(false);
                                    let is_remote_dragging = current_remote_dragging.contains(&(idx.index() as u32));
                                    let is_dragging = is_local_dragging || is_remote_dragging;
                                    let is_selected = current_selected_table == Some(idx);

                                    view! {
                                        <TableNodeView
                                            node=node_clone
                                            is_being_dragged=is_dragging
                                            is_selected=is_selected
                                            on_mouse_down=move |ev: web_sys::MouseEvent| {
                                                if ev.button() != 0 {
                                                    return;
                                                }
                                                ev.prevent_default();
                                                ev.stop_propagation();

                                                // Reset drag flag at start of potential drag
                                                was_dragged.set(false);

                                                // Only start dragging, don't select here
                                                // Selection happens on click (mouseup without significant movement)
                                                graph.with_untracked(|g| {
                                                    if let Some(n) = g.node_weight(node_idx) {
                                                        let (x, y) = n.position;
                                                        let current_zoom = zoom.get_untracked();
                                                        let current_pan_x = pan_x.get_untracked();
                                                        let current_pan_y = pan_y.get_untracked();

                                                        // Учитываем трансформацию канваса при расчете offset
                                                        let transformed_x = x * current_zoom + current_pan_x;
                                                        let transformed_y = y * current_zoom + current_pan_y;
                                                        let offset_x = ev.client_x() as f64 - transformed_x;
                                                        let offset_y = ev.client_y() as f64 - transformed_y;
                                                        set_dragging_node.set(Some((node_idx, offset_x, offset_y)));
                                                    }
                                                });
                                            }
                                            on_click=move |ev: web_sys::MouseEvent| {
                                                if ev.button() != 0 {
                                                    return;
                                                }
                                                ev.stop_propagation();
                                                // Only select if we didn't drag
                                                if !was_dragged.get_untracked() {
                                                    // Select this table and highlight all its edges
                                                    selected_table.set(Some(node_idx));
                                                    highlighted_edges.set(HashSet::new());
                                                }
                                            }
                                            />
                                        }
                                    })
                                })
                            })
                            .collect_view()
                        }}
                </div>

                // SVG слой для ВЫДЕЛЕННЫХ связей (ПОВЕРХ всего)
                <svg class="absolute top-0 left-0 w-full h-full pointer-events-none" style="z-index: 100;">
                    <style>
                        {"
                        @keyframes dash-animation {
                            to {
                                stroke-dashoffset: -20;
                            }
                        }
                        .animated-edge {
                            animation: dash-animation 0.5s linear infinite;
                        }
                        "}
                    </style>
                    <defs>
                        <marker
                            id="arrowhead-white"
                            markerWidth="12"
                            markerHeight="12"
                            refX="10"
                            refY="4"
                            orient="auto"
                        >
                            <polygon points="0 0, 12 4, 0 8" fill="white" />
                        </marker>
                    </defs>

                    <g
                        transform=move || format!(
                            "translate({}, {}) scale({})",
                            pan_x.get(),
                            pan_y.get(),
                            zoom.get()
                        )
                    >
                        // Рендерим ТОЛЬКО выделенные связи
                        {move || {
                        let current_highlighted = highlighted_edges.get();
                        let current_selected_table = selected_table.get();

                        edge_indices.get()
                            .into_iter()
                            .filter_map(|edge_idx| {
                                graph.with(|g| {
                                    let (from_idx, to_idx) = g.edge_endpoints(edge_idx)?;

                                    // Only render if highlighted
                                    let is_highlighted = current_highlighted.contains(&edge_idx)
                                        || current_selected_table.map(|t| t == from_idx || t == to_idx).unwrap_or(false);

                                    if !is_highlighted {
                                        return None;
                                    }

                                    let from_node = g.node_weight(from_idx)?;
                                    let to_node = g.node_weight(to_idx)?;
                                    let edge = g.edge_weight(edge_idx)?;

                                    let (from_x, from_y) = from_node.position;
                                    let (to_x, to_y) = to_node.position;

                                    const NODE_WIDTH: f64 = 280.0;
                                    const HEADER_HEIGHT: f64 = 48.0;
                                    const ROW_HEIGHT: f64 = 36.0;
                                    const PADDING_TOP: f64 = 8.0;
                                    const GAP: f64 = 30.0;

                                    let from_col_idx = from_node.columns.iter()
                                        .position(|col| col.name == edge.from_column)
                                        .unwrap_or(0);

                                    let to_col_idx = to_node.columns.iter()
                                        .position(|col| col.name == edge.to_column)
                                        .unwrap_or(0);

                                    let from_col_y = from_y + HEADER_HEIGHT + PADDING_TOP
                                        + (from_col_idx as f64 * ROW_HEIGHT) + (ROW_HEIGHT / 2.0);
                                    let to_col_y = to_y + HEADER_HEIGHT + PADDING_TOP
                                        + (to_col_idx as f64 * ROW_HEIGHT) + (ROW_HEIGHT / 2.0);

                                    let from_right = from_x + NODE_WIDTH;
                                    let from_left = from_x;
                                    let to_right = to_x + NODE_WIDTH;
                                    let to_left = to_x;

                                    let (_start_x, _start_y, _end_x, _end_y, text_x, text_y, path_data) =
                                        calculate_edge_path(
                                            from_x, from_y, to_x, to_y,
                                            from_col_y, to_col_y,
                                            from_left, from_right, to_left, to_right,
                                            NODE_WIDTH, GAP
                                        );

                                    let rel_type = edge.relationship_type.to_string();
                                    let path_data_glow = path_data.clone();

                                    Some(view! {
                                        <g>
                                            // Glow effect (blurred white background)
                                            <path
                                                d=path_data_glow
                                                stroke="white"
                                                stroke-width="8"
                                                fill="none"
                                                style="filter: blur(4px); opacity: 0.5;"
                                            />
                                            // Main white animated line
                                            <path
                                                d=path_data
                                                stroke="white"
                                                stroke-width="4"
                                                fill="none"
                                                stroke-dasharray="10 10"
                                                class="animated-edge"
                                                marker-end="url(#arrowhead-white)"
                                            />
                                            // Relationship type label
                                            <text
                                                x=text_x
                                                y=text_y
                                                fill="white"
                                                font-size="13"
                                                font-weight="bold"
                                                text-anchor="start"
                                                style="text-shadow: 0 0 4px rgba(0,0,0,0.8);"
                                            >
                                                {rel_type}
                                            </text>
                                        </g>
                                    })
                                })
                            })
                            .collect_view()
                        }}
                    </g>
                </svg>

                // Settings button (правый верхний угол) and AI Chat button
                {
                    let settings_open = RwSignal::new(false);
                    let initial_room_id = RwSignal::new(String::new());
                    let ai_chat_open = RwSignal::new(false);

                    // Auto-open settings and connect when there's a pending room from URL
                    #[cfg(not(feature = "ssr"))]
                    {
                        let ctx = liveshare_ctx;
                        let settings_open_clone = settings_open;
                        let initial_room_id_clone = initial_room_id;

                        Effect::new(move |_| {
                            if let Some(room_id) = ctx.pending_join_room.get() {
                                // Clear the pending room immediately to avoid re-triggering
                                ctx.pending_join_room.set(None);

                                // Set initial room ID for the modal
                                initial_room_id_clone.set(room_id.clone());

                                // Open settings modal
                                settings_open_clone.set(true);

                                // Clear the URL query parameter
                                if let Some(window) = web_sys::window() {
                                    if let Ok(history) = window.history() {
                                        let _ = history.replace_state_with_url(
                                            &wasm_bindgen::JsValue::NULL,
                                            "",
                                            Some("/"),
                                        );
                                    }
                                }

                                // Connect to the room (without password for now)
                                ctx.connect(room_id, None);
                            }
                        });
                    }

                    view! {
                        <div class="absolute top-4 right-4 z-50">
                            <SettingsButton is_open=settings_open />
                        </div>
                        <SettingsModal is_open=settings_open initial_room_id=initial_room_id graph=graph />
                        // Auto Layout button (above AI chat button in bottom-right)
                        <button
                            class="fixed bottom-36 right-4 z-40 flex items-center justify-center w-12 h-12 bg-theme-surface border border-theme-primary text-theme-secondary hover:text-theme-accent hover:border-theme-accent theme-transition transition-colors"
                            style="border-radius: 12px; box-shadow: var(--shadow-lg);"
                            on:click=move |_| {
                                // Apply auto layout
                                graph.update(|g| {
                                    auto_layout(g);
                                });
                                // Sync all table positions to LiveShare
                                if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
                                    graph.with_untracked(|g| {
                                        for node_idx in g.node_indices() {
                                            if let Some(node) = g.node_weight(node_idx) {
                                                liveshare_ctx.send_graph_op(GraphOperation::MoveTable {
                                                    node_id: node_idx.index() as u32,
                                                    position: node.position,
                                                });
                                            }
                                        }
                                    });
                                }
                            }
                            title="Auto Layout - Arrange tables automatically based on relationships"
                        >
                            <Icon name=icons::SPARKLES class="w-6 h-6"/>
                        </button>
                        // AI Chat button (above settings button in bottom-right)
                        <AiChatButton is_open=ai_chat_open />
                        // AI Chat panel
                        <AiChatPanel is_open=ai_chat_open _graph=graph />
                    }
                }

                // Remote cursors overlay (показывает курсоры других пользователей)
                <RemoteCursors zoom=Signal::from(zoom) pan_x=Signal::from(pan_x) pan_y=Signal::from(pan_y) />

                // Cursor tracker (отслеживает и отправляет позицию локального курсора)
                <CursorTracker zoom=Signal::from(zoom) pan_x=Signal::from(pan_x) pan_y=Signal::from(pan_y) />

                // Empty State - показывается когда нет таблиц
                {move || {
                    let table_count = graph.with(|g| g.node_count());
                    if table_count == 0 {
                        view! {
                            <div class="absolute inset-0 flex items-center justify-center">
                                <div class="text-center max-w-md px-8">
                                    <div class="w-24 h-24 mx-auto mb-6 bg-gradient-to-br from-blue-100 to-purple-100 dark:from-blue-900 dark:to-purple-900 rounded-full flex items-center justify-center">
                                        <Icon name=icons::TABLE class="w-12 h-12 text-blue-600 dark:text-blue-400"/>
                                    </div>
                                    <h2 class="text-3xl font-bold text-theme-primary mb-3">
                                        "Welcome to Archischema"
                                    </h2>
                                    <p class="text-theme-tertiary mb-8 leading-relaxed">
                                        "Start designing your database schema by creating your first table, or load a demo to see how it works."
                                    </p>
                                    <div class="flex flex-col gap-3 justify-center items-stretch w-full max-w-xs mx-auto">
                                        <button
                                            class="w-full px-6 py-3 btn-theme-primary rounded-lg font-medium shadow-sm hover:shadow-md transition-all duration-200 flex items-center justify-center gap-2"
                                            on:click=move |_| {
                                                let node_idx = graph.write().create_table_auto((400.0, 300.0));
                                                let name = graph.with(|g| {
                                                    g.node_weight(node_idx).map(|n| n.name.clone()).unwrap_or_default()
                                                });
                                                send_graph_op(GraphOperation::CreateTable {
                                                    node_id: node_idx.index() as u32,
                                                    name,
                                                    position: (400.0, 300.0),
                                                });
                                            }
                                        >
                                            <Icon name=icons::PLUS class="w-5 h-5"/>
                                            "Create Your First Table"
                                        </button>
                                        <button
                                            class="w-full px-6 py-3 text-theme-secondary bg-theme-tertiary rounded-lg font-medium shadow-sm hover:shadow-md transition-all duration-200 flex items-center justify-center gap-2 theme-transition"
                                            on:click=move |_| {
                                                graph.set(create_demo_graph());
                                            }
                                        >
                                            <Icon name=icons::TABLE class="w-5 h-5"/>
                                            "Load Demo Schema"
                                        </button>
                                    </div>
                                    <div class="mt-6 text-sm text-theme-muted">
                                        "Or use the \"New Table\" button in the sidebar"
                                    </div>
                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Вычисляет путь SVG для рёбра графа с оптимизированной логикой
/// Возвращает: (start_x, start_y, end_x, end_y, label_x, label_y, path_data)
#[inline]
#[allow(clippy::too_many_arguments)]
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
) -> (f64, f64, f64, f64, f64, f64, String) {
    if to_left >= from_right + 10.0 {
        // Целевая таблица ЧЁТКО справа - стандартный путь
        let start_x = from_right;
        let end_x = to_left;
        let mid_x = (start_x + end_x) / 2.0;

        let path = format!(
            "M {} {} L {} {} L {} {} L {} {}",
            start_x, from_col_y, mid_x, from_col_y, mid_x, to_col_y, end_x, to_col_y
        );

        // Позиция текста - на первом горизонтальном сегменте
        let label_x = (start_x + mid_x) / 2.0;
        let label_y = from_col_y - 5.0;

        (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
    } else if from_left >= to_right + 10.0 {
        // Целевая таблица ЧЁТКО слева - зеркальный путь
        let start_x = from_left;
        let end_x = to_right;
        let mid_x = (start_x + end_x) / 2.0;

        let path = format!(
            "M {} {} L {} {} L {} {} L {} {}",
            start_x, from_col_y, mid_x, from_col_y, mid_x, to_col_y, end_x, to_col_y
        );

        // Позиция текста - на первом горизонтальном сегменте
        let label_x = (start_x + mid_x) / 2.0;
        let label_y = from_col_y - 5.0;

        (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
    } else {
        // Таблицы перекрываются по X или расположены по диагонали
        let from_center_x = from_x + node_width / 2.0;
        let to_center_x = to_x + node_width / 2.0;

        if to_center_x > from_center_x {
            // Целевая таблица правее по центру
            // Проверяем, не пересекается ли прямой путь from_right → to_left с целевой таблицей
            let direct_path_crosses_target = from_right > to_left && from_right < to_right;

            if direct_path_crosses_target {
                // Путь пересекает целевую таблицу - идем с левой стороны источника к левой стороне цели
                let start_x = from_left;
                let end_x = to_left;
                let min_left = from_left.min(to_left);
                let out_x = min_left - gap;

                let path = format!(
                    "M {} {} L {} {} L {} {} L {} {}",
                    start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
                );

                let label_x = (start_x + out_x) / 2.0;
                let label_y = from_col_y - 5.0;

                (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
            } else {
                // Обычный путь справа
                let start_x = from_right;
                let end_x = to_left;
                let max_right = from_right.max(to_right);
                let out_x = max_right + gap;

                let path = format!(
                    "M {} {} L {} {} L {} {} L {} {}",
                    start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
                );

                let label_x = (start_x + out_x) / 2.0;
                let label_y = from_col_y - 5.0;

                (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
            }
        } else {
            // Целевая таблица левее по центру
            // Проверяем, не пересекается ли прямой путь from_left → to_right с целевой таблицей
            let direct_path_crosses_target = from_left < to_right && from_left > to_left;

            if direct_path_crosses_target {
                // Путь пересекает целевую таблицу - идем с правой стороны источника к правой стороне цели
                let start_x = from_right;
                let end_x = to_right;
                let max_right = from_right.max(to_right);
                let out_x = max_right + gap;

                let path = format!(
                    "M {} {} L {} {} L {} {} L {} {}",
                    start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
                );

                let label_x = (start_x + out_x) / 2.0;
                let label_y = from_col_y - 5.0;

                (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
            } else {
                // Обычный путь слева
                let start_x = from_left;
                let end_x = to_right;
                let min_left = from_left.min(to_left);
                let out_x = min_left - gap;

                let path = format!(
                    "M {} {} L {} {} L {} {} L {} {}",
                    start_x, from_col_y, out_x, from_col_y, out_x, to_col_y, end_x, to_col_y
                );

                let label_x = (start_x + out_x) / 2.0;
                let label_y = from_col_y - 5.0;

                (start_x, from_col_y, end_x, to_col_y, label_x, label_y, path)
            }
        }
    }
}

/// Apply a remote graph operation received from another user
#[cfg(not(feature = "ssr"))]
fn apply_remote_graph_op(graph: RwSignal<SchemaGraph>, op: GraphOperation) {
    use crate::core::TableNode;

    match op {
        GraphOperation::CreateTable {
            node_id: _,
            name,
            position,
        } => {
            graph.update(|g| {
                // Check if table with this name already exists
                let exists = g
                    .node_indices()
                    .any(|idx| g.node_weight(idx).map(|n| n.name == name).unwrap_or(false));
                if !exists {
                    g.add_node(TableNode::new(&name).with_position(position.0, position.1));
                }
            });
        }
        GraphOperation::DeleteTable { node_id } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if g.node_weight(idx).is_some() {
                    g.remove_node(idx);
                }
            });
        }
        GraphOperation::RenameTable { node_id, new_name } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if let Some(node) = g.node_weight_mut(idx) {
                    node.name = new_name;
                }
            });
        }
        GraphOperation::MoveTable { node_id, position } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if let Some(node) = g.node_weight_mut(idx) {
                    node.position = position;
                }
            });
        }
        GraphOperation::AddColumn { node_id, column } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if let Some(node) = g.node_weight_mut(idx) {
                    use crate::core::Column;
                    let mut col = Column::new(&column.name, &column.data_type);
                    if column.is_primary_key {
                        col = col.primary_key();
                    }
                    if !column.is_nullable {
                        col = col.not_null();
                    }
                    if column.is_unique {
                        col = col.unique();
                    }
                    if let Some(default) = column.default_value {
                        col = col.with_default(&default);
                    }
                    node.columns.push(col);
                }
            });
        }
        GraphOperation::UpdateColumn {
            node_id,
            column_index,
            column,
        } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if let Some(node) = g.node_weight_mut(idx) {
                    if column_index < node.columns.len() {
                        use crate::core::Column;
                        let mut col = Column::new(&column.name, &column.data_type);
                        if column.is_primary_key {
                            col = col.primary_key();
                        }
                        if !column.is_nullable {
                            col = col.not_null();
                        }
                        if column.is_unique {
                            col = col.unique();
                        }
                        if let Some(default) = column.default_value {
                            col = col.with_default(&default);
                        }
                        node.columns[column_index] = col;
                    }
                }
            });
        }
        GraphOperation::DeleteColumn {
            node_id,
            column_index,
        } => {
            graph.update(|g| {
                let idx = NodeIndex::new(node_id as usize);
                if let Some(node) = g.node_weight_mut(idx) {
                    if column_index < node.columns.len() {
                        node.columns.remove(column_index);
                    }
                }
            });
        }
        GraphOperation::CreateRelationship {
            edge_id: _,
            from_node,
            to_node,
            relationship,
        } => {
            graph.update(|g| {
                use crate::core::{Relationship, RelationshipType};

                let from_idx = NodeIndex::new(from_node as usize);
                let to_idx = NodeIndex::new(to_node as usize);

                // Check if both nodes exist
                if g.node_weight(from_idx).is_none() || g.node_weight(to_idx).is_none() {
                    return;
                }

                // Check if relationship already exists
                let exists = g.edges_connecting(from_idx, to_idx).any(|e| {
                    e.weight().from_column == relationship.from_column
                        && e.weight().to_column == relationship.to_column
                });

                if !exists {
                    let rel_type = match relationship.relationship_type.as_str() {
                        "1:1" => RelationshipType::OneToOne,
                        "1:N" => RelationshipType::OneToMany,
                        "N:1" => RelationshipType::ManyToOne,
                        "N:M" => RelationshipType::ManyToMany,
                        _ => RelationshipType::ManyToOne, // Default to M:1 as most common FK type
                    };

                    let rel = Relationship::new(
                        &relationship.name,
                        rel_type,
                        &relationship.from_column,
                        &relationship.to_column,
                    );

                    g.add_edge(from_idx, to_idx, rel);
                }
            });
        }
        GraphOperation::DeleteRelationship { edge_id } => {
            graph.update(|g| {
                let idx = petgraph::graph::EdgeIndex::new(edge_id as usize);
                if g.edge_weight(idx).is_some() {
                    g.remove_edge(idx);
                }
            });
        }
    }
}

/// Apply a full graph state snapshot (for initial sync)
#[cfg(not(feature = "ssr"))]
fn apply_graph_state(graph: RwSignal<SchemaGraph>, state: GraphStateSnapshot) {
    use crate::core::TableNode;

    graph.update(|g| {
        // Clear existing graph if we're receiving state from another user
        // Only apply if we have no tables (we're the new user)
        if g.node_count() == 0 && !state.tables.is_empty() {
            // Apply tables from snapshot
            for table in state.tables {
                let mut node =
                    TableNode::new(&table.name).with_position(table.position.0, table.position.1);

                // Add columns
                for col_data in table.columns {
                    use crate::core::Column;
                    let mut col = Column::new(&col_data.name, &col_data.data_type);
                    if col_data.is_primary_key {
                        col = col.primary_key();
                    }
                    if !col_data.is_nullable {
                        col = col.not_null();
                    }
                    if col_data.is_unique {
                        col = col.unique();
                    }
                    if let Some(default) = col_data.default_value {
                        col = col.with_default(&default);
                    }
                    node.columns.push(col);
                }

                g.add_node(node);
            }

            // Apply relationships from snapshot
            for rel_snap in state.relationships {
                use crate::core::{Relationship, RelationshipType};

                let from_idx = NodeIndex::new(rel_snap.from_node as usize);
                let to_idx = NodeIndex::new(rel_snap.to_node as usize);

                // Only add if both nodes exist
                if g.node_weight(from_idx).is_some() && g.node_weight(to_idx).is_some() {
                    let rel_type = match rel_snap.data.relationship_type.as_str() {
                        "1:1" => RelationshipType::OneToOne,
                        "1:N" => RelationshipType::OneToMany,
                        "N:1" => RelationshipType::ManyToOne,
                        "N:M" => RelationshipType::ManyToMany,
                        _ => RelationshipType::ManyToOne, // Default to M:1 as most common FK type
                    };

                    let rel = Relationship::new(
                        &rel_snap.data.name,
                        rel_type,
                        &rel_snap.data.from_column,
                        &rel_snap.data.to_column,
                    );

                    g.add_edge(from_idx, to_idx, rel);
                }
            }
        }
    });
}

/// Create a snapshot of the current graph state
#[cfg(not(feature = "ssr"))]
fn create_graph_snapshot(graph: RwSignal<SchemaGraph>) -> GraphStateSnapshot {
    graph.with(|g| {
        let tables: Vec<TableSnapshot> = g
            .node_indices()
            .filter_map(|idx| {
                g.node_weight(idx).map(|node| {
                    let columns: Vec<ColumnData> = node
                        .columns
                        .iter()
                        .map(|col| ColumnData {
                            name: col.name.clone(),
                            data_type: col.data_type.to_string(),
                            is_primary_key: col.is_primary_key,
                            is_nullable: col.is_nullable,
                            is_unique: col.is_unique,
                            default_value: col.default_value.clone(),
                            foreign_key: None, // TODO: handle FK
                        })
                        .collect();

                    TableSnapshot {
                        node_id: idx.index() as u32,
                        name: node.name.clone(),
                        position: node.position,
                        columns,
                    }
                })
            })
            .collect();

        // Collect relationships (edges)
        let relationships: Vec<RelationshipSnapshot> = g
            .edge_indices()
            .filter_map(|idx| {
                let (from_idx, to_idx) = g.edge_endpoints(idx)?;
                let edge = g.edge_weight(idx)?;

                Some(RelationshipSnapshot {
                    edge_id: idx.index() as u32,
                    from_node: from_idx.index() as u32,
                    to_node: to_idx.index() as u32,
                    data: RelationshipData {
                        name: edge.name.clone(),
                        relationship_type: edge.relationship_type.to_string(),
                        from_column: edge.from_column.clone(),
                        to_column: edge.to_column.clone(),
                    },
                })
            })
            .collect();

        GraphStateSnapshot {
            tables,
            relationships,
        }
    })
}
