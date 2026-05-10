use crate::core::{SchemaGraph, TableOps, auto_layout};
use crate::ui::ai_chat::AiChatPanel;
use crate::ui::auth::UserMenu;
use crate::ui::common::{BrandMark, KbdKey};
#[cfg(not(feature = "ssr"))]
use crate::ui::liveshare_client::{
    ColumnData, GraphStateSnapshot, RelationshipData, RelationshipSnapshot, TableSnapshot,
};
use crate::ui::liveshare_client::{ConnectionState, GraphOperation, use_liveshare_context};
use crate::ui::notifications::{NotificationManager, NotificationsContainer};
use crate::ui::remote_cursors::{CursorTracker, RemoteCursors};
use crate::ui::settings_modal::SettingsModal;
use crate::ui::sidebar::Sidebar;
use crate::ui::source_editor::{EditorMode, SourceEditor};
use crate::ui::table::{
    TABLE_BODY_PADDING_Y, TABLE_EDGE_GAP, TABLE_HEADER_HEIGHT, TABLE_NODE_WIDTH, TABLE_ROW_HEIGHT,
    TableNodeView,
};
use crate::ui::{EmptyState, Icon, icons};
use leptos::prelude::*;
use leptos::{html, web_sys};
use leptos_router::components::A;
use petgraph::graph::{EdgeIndex, NodeIndex};
#[cfg(not(feature = "ssr"))]
use std::collections::HashMap;
use std::collections::HashSet;

/// Helper function to dispatch a save event to the parent component
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
#[allow(dead_code)]
fn dispatch_save_event(_reason: &str) {
    // No-op on server
}

#[component]
pub fn SchemaCanvas(
    graph: RwSignal<SchemaGraph>,
    /// Diagram name (editable)
    #[prop(optional)]
    diagram_name: Option<RwSignal<String>>,
    /// Diagram ID (for API calls)
    #[prop(optional)]
    diagram_id: Option<String>,
    /// Whether this is demo mode
    #[prop(default = false)]
    is_demo: bool,
    /// Callback when diagram name changes
    #[prop(optional, into)]
    on_name_change: Option<Callback<String>>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    // Notification manager for canvas notifications
    let notification_manager = NotificationManager::new();

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

    // Sidebar collapsed state (shared with Sidebar for layout coordination)
    let sidebar_collapsed = RwSignal::new(false);

    // State for highlighted edges (when clicking on edge or table)
    let highlighted_edges: RwSignal<HashSet<EdgeIndex>> = RwSignal::new(HashSet::new());
    // State for selected table (to highlight all its edges)
    let selected_table: RwSignal<Option<NodeIndex>> = RwSignal::new(None);
    // Shared column editor target used for sidebar and canvas row highlighting.
    let active_column_editor: RwSignal<Option<(NodeIndex, Option<usize>)>> = RwSignal::new(None);
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

                            if let GraphOperation::MoveTable {
                                node_id,
                                table_uuid: _,
                                position,
                            } = &op
                            {
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
                                // For non-move operations, apply directly
                                apply_remote_graph_op(graph_clone, op);
                            }
                        }
                    }
                });

            let _ = window.add_event_listener_with_callback(
                "liveshare-graph-op",
                handler.as_ref().unchecked_ref(),
            );
            handler.forget();
        });

        // Handler for table drag start
        Effect::new(move |_| {
            let window = web_sys::window().expect("no window");

            let handler = Closure::<dyn Fn(web_sys::CustomEvent)>::new(
                move |e: web_sys::CustomEvent| {
                    if let Some(detail) = e.detail().as_string() {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&detail) {
                            if let (Some(user_id_str), Some(node_id), Some(offset)) = (
                                data["user_id"].as_str(),
                                data["node_id"].as_u64(),
                                data["offset"].as_array(),
                            ) {
                                if let (Some(offset_x), Some(offset_y)) = (
                                    offset.get(0).and_then(|v| v.as_f64()),
                                    offset.get(1).and_then(|v| v.as_f64()),
                                ) {
                                    leptos::logging::log!(
                                        "Remote user {} started dragging table {} with offset ({}, {})",
                                        user_id_str,
                                        node_id,
                                        offset_x,
                                        offset_y
                                    );
                                    // Mark node as being remotely dragged
                                    remote_dragging_nodes.update(|set| {
                                        set.insert(node_id as u32);
                                    });
                                }
                            }
                        }
                    }
                },
            );

            let _ = window.add_event_listener_with_callback(
                "liveshare-table-drag-start",
                handler.as_ref().unchecked_ref(),
            );
            handler.forget();
        });

        // Handler for table drag end
        let targets_for_drag_end = remote_drag_targets.clone();
        let animation_running_for_drag_end = animation_running.clone();
        Effect::new(move |_| {
            let window = web_sys::window().expect("no window");

            let graph_clone = graph;
            let targets = targets_for_drag_end.clone();
            let anim_running = animation_running_for_drag_end.clone();
            let handler =
                Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                    if let Some(detail) = e.detail().as_string() {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&detail) {
                            if let (Some(user_id_str), Some(node_id), Some(position)) = (
                                data["user_id"].as_str(),
                                data["node_id"].as_u64(),
                                data["position"].as_array(),
                            ) {
                                if let (Some(pos_x), Some(pos_y)) = (
                                    position.get(0).and_then(|v| v.as_f64()),
                                    position.get(1).and_then(|v| v.as_f64()),
                                ) {
                                    leptos::logging::log!(
                                        "Remote user {} finished dragging table {} to ({}, {})",
                                        user_id_str,
                                        node_id,
                                        pos_x,
                                        pos_y
                                    );

                                    let now = js_sys::Date::now();
                                    targets
                                        .borrow_mut()
                                        .insert(node_id as u32, (pos_x, pos_y, now));

                                    // Start animation loop to smoothly move to final position
                                    start_animation_loop(
                                        targets.clone(),
                                        anim_running.clone(),
                                        remote_dragging_nodes,
                                        graph_clone,
                                    );
                                }
                            }
                        }
                    }
                });

            let _ = window.add_event_listener_with_callback(
                "liveshare-table-drag-end",
                handler.as_ref().unchecked_ref(),
            );
            handler.forget();
        });

        // Effect to update remote table positions based on remote user cursor positions
        let targets_for_cursor = remote_drag_targets.clone();
        let animation_running_for_cursor = animation_running.clone();
        Effect::new(move |_| {
            let remote_users = liveshare_ctx.remote_users.get();
            let targets = targets_for_cursor.clone();
            let anim_running = animation_running_for_cursor.clone();

            // For each remote user that is dragging a table
            for user in remote_users.iter() {
                if let (Some(cursor), Some((node_id, offset_x, offset_y))) =
                    (user.cursor, user.dragging_table)
                {
                    // Calculate table position from cursor position and offset
                    let table_x = cursor.0 - offset_x;
                    let table_y = cursor.1 - offset_y;

                    let now = js_sys::Date::now();
                    targets
                        .borrow_mut()
                        .insert(node_id, (table_x, table_y, now));

                    // Ensure animation loop is running
                    start_animation_loop(
                        targets.clone(),
                        anim_running.clone(),
                        remote_dragging_nodes,
                        graph,
                    );
                }
            }
        });

        // Handler for full graph state (initial sync when joining room)
        // Use thread_local static to ensure listeners are only set up once
        {
            use std::cell::Cell;
            use wasm_bindgen::JsCast;
            use wasm_bindgen::closure::Closure;

            thread_local! {
                static GRAPH_STATE_LISTENER_SETUP: Cell<bool> = Cell::new(false);
            }

            let already_setup = GRAPH_STATE_LISTENER_SETUP.with(|v| {
                let was_setup = v.get();
                if !was_setup {
                    v.set(true);
                }
                was_setup
            });

            if !already_setup {
                let window = web_sys::window().expect("no window");
                let graph_clone = graph;
                let notif_manager = notification_manager;

                let closure = Closure::<dyn Fn(web_sys::CustomEvent)>::new(
                    move |e: web_sys::CustomEvent| {
                        if let Some(detail) = e.detail().as_string() {
                            if let Ok(state) = serde_json::from_str::<GraphStateSnapshot>(&detail) {
                                // Check if signal is still valid before accessing
                                let had_local_data = graph_clone
                                    .try_with_untracked(|g| g.node_count() > 0)
                                    .unwrap_or(false);

                                leptos::logging::log!(
                                    "Applying graph state: {} tables, {} relationships",
                                    state.tables.len(),
                                    state.relationships.len()
                                );

                                // Only apply if signal is still valid
                                if graph_clone
                                    .try_update_untracked(|g| {
                                        apply_graph_state_internal(g, state.clone());
                                    })
                                    .is_some()
                                {
                                    // Show notification if local data was replaced
                                    if had_local_data {
                                        notif_manager.warning(
                                            "Session Joined",
                                            "Your local tables were replaced with the LiveShare session state"
                                        );
                                    }
                                } else {
                                    leptos::logging::log!(
                                        "Ignoring graph state: component has been disposed"
                                    );
                                }
                            }
                        }
                    },
                );

                let _ = window.add_event_listener_with_callback(
                    "liveshare-graph-state",
                    closure.as_ref().unchecked_ref(),
                );

                // Intentionally leak - global listener for app lifetime
                closure.forget();
            }
        }

        // Handler for graph state requests from other users
        {
            use std::cell::Cell;
            use wasm_bindgen::JsCast;
            use wasm_bindgen::closure::Closure;

            thread_local! {
                static REQUEST_GRAPH_STATE_LISTENER_SETUP: Cell<bool> = Cell::new(false);
            }

            let already_setup = REQUEST_GRAPH_STATE_LISTENER_SETUP.with(|v| {
                let was_setup = v.get();
                if !was_setup {
                    v.set(true);
                }
                was_setup
            });

            if !already_setup {
                let window = web_sys::window().expect("no window");
                let graph_clone = graph;
                let ctx_clone = liveshare_ctx;

                let closure =
                    Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                        if let Some(detail) = e.detail().as_string() {
                            // Parse requester_id from the detail
                            if let Ok(requester_id) = uuid::Uuid::parse_str(&detail) {
                                // Check if signal is still valid before creating snapshot
                                if let Some(state) = graph_clone
                                    .try_with_untracked(|g| create_graph_snapshot_internal(g))
                                {
                                    leptos::logging::log!(
                                        "Sending graph state to {:?}: {} tables",
                                        requester_id,
                                        state.tables.len()
                                    );
                                    // Send it to the requester
                                    ctx_clone.send_graph_state_response(requester_id, state);
                                } else {
                                    leptos::logging::log!(
                                        "Cannot send graph state: component has been disposed"
                                    );
                                }
                            }
                        }
                    });

                let _ = window.add_event_listener_with_callback(
                    "liveshare-request-graph-state",
                    closure.as_ref().unchecked_ref(),
                );

                // Intentionally leak - global listener for app lifetime
                closure.forget();
            }
        }
    }

    // Helper function to send graph op when connected
    let send_graph_op = move |op: GraphOperation| {
        if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
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

            // Обработчик перемещения - используем batch для группировки обновлений
            let move_closure = Closure::new(move |ev: web_sys::MouseEvent| {
                ev.prevent_default();

                // Учитываем трансформацию канваса при перетаскивании
                let current_zoom = zoom.with_untracked(|v| *v);
                let current_pan_x = pan_x.with_untracked(|v| *v);
                let current_pan_y = pan_y.with_untracked(|v| *v);

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

                // No longer sending position updates here - cursor tracker handles it
            });

            // Обработчик отпускания кнопки - отправляем TableDragEnd
            let up_closure = Closure::new(move |_: web_sys::MouseEvent| {
                // Get final position and send TableDragEnd
                let final_pos =
                    graph.with_untracked(|g| g.node_weight(node_idx).map(|n| n.position));
                if let Some(position) = final_pos {
                    if liveshare_ctx.connection_state.with_untracked(|v| *v)
                        == ConnectionState::Connected
                    {
                        liveshare_ctx.send_table_drag_end(node_idx.index() as u32, position);
                    }
                }
                set_dragging_node.set(None);

                // Trigger save after drag ends
                dispatch_save_event("table_moved");
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
    let handle_table_focus = Callback::new(move |_node_idx: NodeIndex| {
        // TODO: Центрировать таблицу на канвасе
    });

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
            let initial_pan_x = pan_x.with_untracked(|v| *v);
            let initial_pan_y = pan_y.with_untracked(|v| *v);

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

    let settings_open = RwSignal::new(false);
    let initial_room_id = RwSignal::new(String::new());
    let ai_chat_open = RwSignal::new(false);

    // Auto-open settings and connect when there's a pending room from URL.
    #[cfg(not(feature = "ssr"))]
    {
        let ctx = liveshare_ctx;
        let settings_open_clone = settings_open;
        let initial_room_id_clone = initial_room_id;

        Effect::new(move |_| {
            if let Some(room_id) = ctx.pending_join_room.get() {
                ctx.pending_join_room.set(None);
                initial_room_id_clone.set(room_id.clone());
                settings_open_clone.set(true);

                if let Some(window) = web_sys::window() {
                    if let Ok(history) = window.history() {
                        let _ = history.replace_state_with_url(
                            &wasm_bindgen::JsValue::NULL,
                            "",
                            Some("/"),
                        );
                    }
                }

                ctx.connect(room_id, None);
            }
        });
    }

    view! {
        <div class="editor-shell app relative h-screen w-full overflow-hidden theme-transition">
            // Notification container
            <NotificationsContainer notifications=notification_manager.notifications() />

            <EditorTopBar
                diagram_name=diagram_name
                is_demo=is_demo
                ai_chat_open=ai_chat_open
                settings_open=settings_open
            />

            <div class="editor-body">
                <Show when=move || editor_mode.get() == EditorMode::Source>
                    <SourceEditor graph=graph readonly=false editor_mode=editor_mode />
                </Show>

                <Show when=move || editor_mode.get() == EditorMode::Visual>
                    <Sidebar
                        graph=graph
                        on_table_focus=handle_table_focus
                        column_editor_target=active_column_editor
                        editor_mode=editor_mode
                        is_collapsed=sidebar_collapsed
                    />

                    // Основной канвас - показывается в режиме Visual
                    <div
                        node_ref=canvas_ref
                        class="editor-canvas"
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
                            if ev.button() == 0 {
                                highlighted_edges.set(HashSet::new());
                                selected_table.set(None);
                            }
                        }
                        on:contextmenu=move |ev: web_sys::MouseEvent| {
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
                            <polygon points="0 0, 10 3, 0 6" fill="var(--relation)" />
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

                                    // Находим индекс колонки в исходной таблице
                                    let from_col_idx = from_node.columns.iter()
                                        .position(|col| col.name == edge.from_column)
                                        .unwrap_or(0);

                                    // Находим индекс колонки в целевой таблице
                                    let to_col_idx = to_node.columns.iter()
                                        .position(|col| col.name == edge.to_column)
                                        .unwrap_or(0);

                                    // Вычисляем Y координаты для конкретных колонок
                                    let from_col_y = from_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                        + (from_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);
                                    let to_col_y = to_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                        + (to_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);

                                    // Определяем границы таблиц
                                    let from_right = from_x + TABLE_NODE_WIDTH;
                                    let from_left = from_x;
                                    let to_right = to_x + TABLE_NODE_WIDTH;
                                    let to_left = to_x;

                                    // Умная логика выбора пути стрелки
                                    let (_start_x, _start_y, _end_x, _end_y, text_x, text_y, path_data) =
                                        calculate_edge_path(
                                            from_x, from_y, to_x, to_y,
                                            from_col_y, to_col_y,
                                            from_left, from_right, to_left, to_right,
                                            TABLE_NODE_WIDTH, TABLE_EDGE_GAP
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
                                                class="schema-relation"
                                                stroke-width="1.75"
                                                fill="none"
                                                marker-end="url(#arrowhead)"
                                                style="pointer-events: none;"
                                                style:opacity=if is_highlighted { "0.3" } else { "1" }
                                            />
                                            <text
                                                x=text_x
                                                y=text_y
                                                class="schema-relation-label select-none"
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
                    let current_active_column = active_column_editor.get();

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
                                    let active_column_index = current_active_column
                                        .and_then(|(active_node, active_col)| if active_node == idx { active_col } else { None });

                                    view! {
                                        <TableNodeView
                                            node=node_clone
                                            is_being_dragged=is_dragging
                                            is_selected=is_selected
                                            active_column_index=active_column_index
                                            on_mouse_down=Callback::new(move |ev: web_sys::MouseEvent| {
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
                                                        let current_zoom = zoom.with_untracked(|v| *v);
                                                        let current_pan_x = pan_x.with_untracked(|v| *v);
                                                        let current_pan_y = pan_y.with_untracked(|v| *v);

                                                        // Convert mouse position to canvas coordinates
                                                        // Canvas is full-width, so no sidebar offset needed
                                                        let canvas_mouse_x = (ev.client_x() as f64 - current_pan_x) / current_zoom;
                                                        let canvas_mouse_y = (ev.client_y() as f64 - current_pan_y) / current_zoom;

                                                        // Calculate offset in canvas space
                                                        let offset_x = canvas_mouse_x - x;
                                                        let offset_y = canvas_mouse_y - y;

                                                        set_dragging_node.set(Some((node_idx, offset_x, offset_y)));

                                                        // Send TableDragStart to attach cursor to table
                                                        if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
                                                            liveshare_ctx.send_table_drag_start(
                                                                node_idx.index() as u32,
                                                                (offset_x, offset_y)
                                                            );
                                                        }
                                                    }
                                                });
                                            })
                                            on_click=Callback::new(move |ev: web_sys::MouseEvent| {
                                                if ev.button() != 0 {
                                                    return;
                                                }
                                                ev.stop_propagation();
                                                // Only select if we didn't drag
                                                if !was_dragged.with_untracked(|v| *v) {
                                                    // Select this table and highlight all its edges
                                                    selected_table.set(Some(node_idx));
                                                    highlighted_edges.set(HashSet::new());
                                                }
                                            })
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
                            id="arrowhead-selected"
                            markerWidth="12"
                            markerHeight="12"
                            refX="10"
                            refY="4"
                            orient="auto"
                        >
                            <polygon points="0 0, 12 4, 0 8" fill="var(--relation-selected)" />
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

                                    let from_col_idx = from_node.columns.iter()
                                        .position(|col| col.name == edge.from_column)
                                        .unwrap_or(0);

                                    let to_col_idx = to_node.columns.iter()
                                        .position(|col| col.name == edge.to_column)
                                        .unwrap_or(0);

                                    let from_col_y = from_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                        + (from_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);
                                    let to_col_y = to_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                        + (to_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);

                                    let from_right = from_x + TABLE_NODE_WIDTH;
                                    let from_left = from_x;
                                    let to_right = to_x + TABLE_NODE_WIDTH;
                                    let to_left = to_x;

                                    let (_start_x, _start_y, _end_x, _end_y, text_x, text_y, path_data) =
                                        calculate_edge_path(
                                            from_x, from_y, to_x, to_y,
                                            from_col_y, to_col_y,
                                            from_left, from_right, to_left, to_right,
                                            TABLE_NODE_WIDTH, TABLE_EDGE_GAP
                                        );

                                    let rel_type = edge.relationship_type.to_string();
                                    let path_data_glow = path_data.clone();

                                    Some(view! {
                                        <g>
                                            // Glow effect (blurred accent background)
                                            <path
                                                d=path_data_glow
                                                class="schema-relation-highlight-glow"
                                                stroke-width="8"
                                                fill="none"
                                            />
                                            // Main animated line
                                            <path
                                                d=path_data
                                                class="schema-relation-highlight animated-edge"
                                                stroke-width="3"
                                                fill="none"
                                                stroke-dasharray="10 10"
                                                marker-end="url(#arrowhead-selected)"
                                            />
                                            // Relationship type label
                                            <text
                                                x=text_x
                                                y=text_y
                                                class="schema-relation-highlight-label"
                                                font-size="13"
                                                text-anchor="start"
                                                style="filter: drop-shadow(0 0 5px var(--background));"
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

                // Canvas tool dock
                <div class="canvas-tool-dock" aria-label="Canvas tools">
                    <button type="button" class="btn-icon is-active" title="Select">
                        <Icon name=icons::TABLE class="h-3.5 w-3.5"/>
                    </button>
                    <button
                        type="button"
                        class="btn-icon"
                        title="New table"
                        on:click=move |_| {
                            let node_idx = graph.write().create_table_auto((400.0, 300.0));
                            let (name, uuid) = graph.with(|g| {
                                g.node_weight(node_idx)
                                    .map(|n| (n.name.clone(), n.uuid))
                                    .unwrap_or_default()
                            });
                            send_graph_op(GraphOperation::CreateTable {
                                node_id: node_idx.index() as u32,
                                table_uuid: uuid,
                                name,
                                position: (400.0, 300.0),
                            });
                            dispatch_save_event("table_created");
                        }
                    >
                        <Icon name=icons::PLUS class="h-3.5 w-3.5"/>
                    </button>
                    <button
                        type="button"
                        class="btn-icon"
                        title="Auto layout"
                        on:click=move |_| {
                            graph.update(auto_layout);
                            if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
                                graph.with_untracked(|g| {
                                    for node_idx in g.node_indices() {
                                        if let Some(node) = g.node_weight(node_idx) {
                                            liveshare_ctx.send_graph_op(GraphOperation::MoveTable {
                                                node_id: node_idx.index() as u32,
                                                table_uuid: node.uuid,
                                                position: node.position,
                                            });
                                        }
                                    }
                                });
                            }
                            dispatch_save_event("auto_layout");
                        }
                    >
                        <Icon name=icons::SPARKLES class="h-3.5 w-3.5"/>
                    </button>
                    <button type="button" class="btn-icon" title="Snap to grid">
                        <Icon name=icons::SQUARES_2X2 class="h-3.5 w-3.5"/>
                    </button>
                </div>

                // Selection mini-toolbar
                {move || {
                    selected_table.get().and_then(|idx| {
                        graph.with(|g| {
                            g.node_weight(idx).map(|node| {
                                let (x, y) = node.position;
                                let left = pan_x.get() + x * zoom.get();
                                let top = pan_y.get() + y * zoom.get() - 44.0;
                                let table_name = node.name.clone();
                                view! {
                                    <div class="canvas-selection-toolbar" style:left=format!("{}px", left) style:top=format!("{}px", top.max(8.0))>
                                        <span class="mono text-theme-muted">{table_name}</span>
                                        <span class="hairline-v h-4"></span>
                                        <button type="button" class="btn-ghost btn-sm" title="Rename from the sidebar inspector">
                                            <Icon name=icons::EDIT class="h-3 w-3"/>"Rename"
                                        </button>
                                        <button type="button" class="btn-ghost btn-sm" title="Add a column from the sidebar inspector">
                                            <Icon name=icons::PLUS class="h-3 w-3"/>"Column"
                                        </button>
                                        <button type="button" class="btn-ghost btn-sm" disabled=true title="Use a column foreign key to create a relation">
                                            <Icon name=icons::LIGHTNING class="h-3 w-3"/>"Relate"
                                        </button>
                                    </div>
                                }
                            })
                        })
                    })
                }}

                // Relationship hovercard for selected edge
                {move || {
                    let selected_edge = highlighted_edges.get().iter().next().copied();
                    selected_edge.and_then(|edge_idx| {
                        graph.with(|g| {
                            let (from_idx, to_idx) = g.edge_endpoints(edge_idx)?;
                            let from_node = g.node_weight(from_idx)?;
                            let to_node = g.node_weight(to_idx)?;
                            let edge = g.edge_weight(edge_idx)?;
                            let from_col_idx = from_node.columns.iter()
                                .position(|col| col.name == edge.from_column)
                                .unwrap_or(0);
                            let to_col_idx = to_node.columns.iter()
                                .position(|col| col.name == edge.to_column)
                                .unwrap_or(0);
                            let (from_x, from_y) = from_node.position;
                            let (to_x, to_y) = to_node.position;
                            let from_col_y = from_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                + (from_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);
                            let to_col_y = to_y + TABLE_HEADER_HEIGHT + TABLE_BODY_PADDING_Y
                                + (to_col_idx as f64 * TABLE_ROW_HEIGHT) + (TABLE_ROW_HEIGHT / 2.0);
                            let (_, _, _, _, text_x, text_y, _) = calculate_edge_path(
                                from_x, from_y, to_x, to_y, from_col_y, to_col_y,
                                from_x, from_x + TABLE_NODE_WIDTH, to_x, to_x + TABLE_NODE_WIDTH,
                                TABLE_NODE_WIDTH, TABLE_EDGE_GAP,
                            );
                            let left = pan_x.get() + text_x * zoom.get() + 12.0;
                            let top = pan_y.get() + text_y * zoom.get() + 8.0;
                            let relation = format!("{}.{} -> {}.{}", from_node.name, edge.from_column, to_node.name, edge.to_column);
                            let rel_type = edge.relationship_type.to_string();
                            Some(view! {
                                <div class="relation-hovercard" style:left=format!("{}px", left) style:top=format!("{}px", top)>
                                    <div class="flex items-center gap-1.5 text-xs text-theme-primary">
                                        <Icon name=icons::LIGHTNING class="h-3 w-3 text-theme-accent"/>
                                        <span class="mono">{relation}</span>
                                    </div>
                                    <div class="mt-1 flex gap-1.5">
                                        <span class="chip h-[18px] text-[10px]">{rel_type}</span>
                                        <span class="chip h-[18px] text-[10px]">"relationship"</span>
                                    </div>
                                </div>
                            })
                        })
                    })
                }}

                // Bottom-center floating action bar
                <div class="canvas-action-bar">
                    <button
                        type="button"
                        class="btn-secondary btn-sm btn-pill"
                        on:click=move |_| {
                            graph.update(auto_layout);
                            if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
                                graph.with_untracked(|g| {
                                    for node_idx in g.node_indices() {
                                        if let Some(node) = g.node_weight(node_idx) {
                                            liveshare_ctx.send_graph_op(GraphOperation::MoveTable {
                                                node_id: node_idx.index() as u32,
                                                table_uuid: node.uuid,
                                                position: node.position,
                                            });
                                        }
                                    }
                                });
                            }
                            dispatch_save_event("auto_layout");
                        }
                    >
                        <Icon name=icons::SPARKLES class="h-3 w-3"/>"Auto layout"
                    </button>
                    <button type="button" class="btn-secondary btn-sm btn-pill" disabled=true title="Relationship creation is handled from column editor">
                        <Icon name=icons::LIGHTNING class="h-3 w-3"/>"Add relation"
                    </button>
                    <button
                        type="button"
                        class="btn-secondary btn-sm btn-pill"
                        on:click=move |_| {
                            let node_idx = graph.write().create_table_auto((400.0, 300.0));
                            let (name, uuid) = graph.with(|g| {
                                g.node_weight(node_idx)
                                    .map(|n| (n.name.clone(), n.uuid))
                                    .unwrap_or_default()
                            });
                            send_graph_op(GraphOperation::CreateTable {
                                node_id: node_idx.index() as u32,
                                table_uuid: uuid,
                                name,
                                position: (400.0, 300.0),
                            });
                            dispatch_save_event("table_created");
                        }
                    >
                        <Icon name=icons::PLUS class="h-3 w-3"/>"Add table"
                    </button>
                    <span class="hairline-v h-4"></span>
                    <button type="button" class="btn-primary btn-sm btn-pill" on:click=move |_| ai_chat_open.set(true)>
                        <Icon name=icons::SPARKLES class="h-3 w-3"/>"Ask AI" <KbdKey class="bg-black/20 text-white border-black/25".to_string()>"Ctrl J"</KbdKey>
                    </button>
                </div>

                // Bottom-right zoom controls
                <div class="canvas-zoom-dock">
                    <button type="button" class="btn-ghost btn-sm btn-pill" title="Zoom out" on:click=move |_| set_zoom.update(|z| *z = (*z * 0.9).clamp(0.1, 5.0))>"-"</button>
                    <button type="button" class="btn-ghost btn-sm btn-pill mono min-w-[56px]" title="Reset zoom" on:click=move |_| {
                        set_zoom.set(1.0);
                        set_pan_x.set(0.0);
                        set_pan_y.set(0.0);
                    }>{move || format!("{:.0}%", zoom.get() * 100.0)}</button>
                    <button type="button" class="btn-ghost btn-sm btn-pill" title="Zoom in" on:click=move |_| set_zoom.update(|z| *z = (*z * 1.1).clamp(0.1, 5.0))>"+"</button>
                </div>

                // Remote cursors overlay (показывает курсоры других пользователей)
                <RemoteCursors zoom=Signal::from(zoom) pan_x=Signal::from(pan_x) pan_y=Signal::from(pan_y) />

                // Cursor tracker (отслеживает и отправляет позицию локального курсора)
                <CursorTracker zoom=Signal::from(zoom) pan_x=Signal::from(pan_x) pan_y=Signal::from(pan_y) />

                // Empty State - показывается когда нет таблиц
                {move || {
                    let table_count = graph.with(|g| g.node_count());
                    if table_count == 0 {
                        view! {
                            <div class="pointer-events-none absolute inset-0 flex items-center justify-center px-6">
                                <EmptyState
                                    title="A blank canvas, ready to model".to_string()
                                    description="Start by sketching a table, pasting SQL, or describing your domain to the AI assistant.".to_string()
                                    icon=icons::TABLE
                                    class="pointer-events-auto rounded-2xl border border-theme bg-theme-surface/80 p-8 shadow-theme-xl backdrop-blur".to_string()
                                >
                                    <div class="flex w-full max-w-sm flex-col items-stretch justify-center gap-3 sm:flex-row">
                                        <button
                                            class="btn-primary btn-lg min-w-[130px]"
                                            on:click=move |_| {
                                                let node_idx = graph.write().create_table_auto((400.0, 300.0));
                                                let (name, uuid) = graph.with(|g| {
                                                    g.node_weight(node_idx).map(|n| (n.name.clone(), n.uuid)).unwrap_or_default()
                                                });
                                                send_graph_op(GraphOperation::CreateTable {
                                                    node_id: node_idx.index() as u32,
                                                    table_uuid: uuid,
                                                    name,
                                                    position: (400.0, 300.0),
                                                });
                                                dispatch_save_event("table_created");
                                            }
                                        >
                                            <Icon name=icons::PLUS class="w-5 h-5"/>
                                            "Add table"
                                        </button>
                                        <button
                                            class="btn-secondary btn-lg min-w-[130px]"
                                            on:click=move |_| editor_mode.set(EditorMode::Source)
                                        >
                                            <Icon name=icons::CODE class="w-5 h-5"/>
                                            "Paste SQL"
                                        </button>
                                        <button
                                            class="btn-secondary btn-lg min-w-[130px]"
                                            on:click=move |_| ai_chat_open.set(true)
                                        >
                                            <Icon name=icons::SPARKLES class="w-5 h-5"/>
                                            "Describe to AI"
                                        </button>
                                    </div>
                                    <div class="text-sm text-theme-muted">
                                        "Use the left sidebar for existing tables and the bottom bar for layout actions."
                                    </div>
                                </EmptyState>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
                    </div>

                    <AiChatPanel is_open=ai_chat_open _graph=graph />
                </Show>
            </div>

            <SettingsModal
                is_open=settings_open
                initial_room_id=initial_room_id
                graph=graph
                diagram_name=diagram_name.map(|s| s.with_untracked(|v| v.clone()))
                diagram_id=diagram_id
                is_demo=is_demo
                on_name_change=on_name_change
            />
        </div>
    }
}

#[component]
fn EditorTopBar(
    #[prop(default = None)] diagram_name: Option<RwSignal<String>>,
    #[prop(default = false)] is_demo: bool,
    ai_chat_open: RwSignal<bool>,
    settings_open: RwSignal<bool>,
) -> impl IntoView {
    let liveshare_ctx = use_liveshare_context();

    let title = move || {
        diagram_name
            .map(|name| name.get())
            .unwrap_or_else(|| "Untitled diagram".to_string())
    };

    let sync_label = move || match liveshare_ctx.connection_state.get() {
        ConnectionState::Connected => "Live",
        ConnectionState::Connecting => "Connecting",
        ConnectionState::Reconnecting => "Reconnecting",
        ConnectionState::Error => "Sync error",
        ConnectionState::Disconnected => "Local",
    };

    let sync_class = move || match liveshare_ctx.connection_state.get() {
        ConnectionState::Connected => "status-dot status-dot-live",
        ConnectionState::Connecting | ConnectionState::Reconnecting => {
            "status-dot status-dot-pending"
        }
        ConnectionState::Error => "status-dot status-dot-error",
        ConnectionState::Disconnected => "status-dot",
    };

    view! {
        <header class="editor-topbar">
            <div class="flex min-w-0 items-center gap-2 text-[12.5px] text-theme-muted">
                <BrandMark />
                <A href="/dashboard" attr:class="editor-breadcrumb-link" attr:title="Back to dashboard">
                    <Icon name=icons::FOLDER class="h-3 w-3" />
                    <span class="hidden sm:inline">"Client work"</span>
                </A>
                <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                <span class="truncate font-medium text-theme-primary">{title}</span>
                {if is_demo {
                    view! { <span class="chip h-[18px] text-[10.5px]">"Demo"</span> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                <span class="chip h-[18px] text-[10.5px]">
                    <span class=sync_class></span>
                    {sync_label}
                </span>
            </div>

            <div class="flex flex-1 items-center justify-center gap-2 text-theme-muted">
                <button type="button" class="btn-ghost btn-sm" title="Current branch">
                    <Icon name=icons::GIT_BRANCH class="h-3 w-3" />
                    "main"
                </button>
            </div>

            <div class="flex items-center gap-2">
                <div class="hidden items-center sm:flex">
                    {move || {
                        liveshare_ctx.remote_users.get()
                            .into_iter()
                            .take(3)
                            .enumerate()
                            .map(|(index, user)| {
                                let initial = user.username.chars().next()
                                    .map(|c| c.to_uppercase().to_string())
                                    .unwrap_or_else(|| "?".to_string());
                                let margin = if index == 0 { "0" } else { "-6px" };
                                view! {
                                    <span
                                        class="editor-avatar"
                                        style=format!("background: {}; margin-left: {};", user.color, margin)
                                        title=user.username
                                    >
                                        {initial}
                                    </span>
                                }
                            })
                            .collect_view()
                    }}
                </div>
                <button type="button" class="btn-secondary btn-sm" on:click=move |_| settings_open.set(true)>
                    <Icon name=icons::USER_PLUS class="h-3 w-3" />
                    "Share"
                </button>
                <span class="hairline-v h-[18px]"></span>
                <button type="button" class="btn-ghost btn-sm" on:click=move |_| ai_chat_open.set(true)>
                    <Icon name=icons::SPARKLES class="h-3 w-3 text-theme-accent" />
                    "Ask AI"
                    <KbdKey>"Ctrl J"</KbdKey>
                </button>
                <button type="button" class="btn-icon" title="Settings" on:click=move |_| settings_open.set(true)>
                    <Icon name=icons::SETTINGS class="icon-standalone" />
                </button>
                <UserMenu />
            </div>
        </header>
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

    // Helper function to find node by UUID
    let find_node_by_uuid = |g: &SchemaGraph, table_uuid: uuid::Uuid| -> Option<NodeIndex> {
        g.node_indices().find(|&idx| {
            g.node_weight(idx)
                .map(|n| n.uuid == table_uuid)
                .unwrap_or(false)
        })
    };

    match op {
        GraphOperation::CreateTable {
            node_id: _,
            table_uuid,
            name,
            position,
        } => {
            graph.update(|g| {
                // Check if table with this UUID already exists
                let exists = find_node_by_uuid(g, table_uuid).is_some();
                if !exists {
                    let mut table = TableNode::new(&name).with_position(position.0, position.1);
                    table.uuid = table_uuid; // Use the UUID from the operation
                    g.add_node(table);
                }
            });
        }
        GraphOperation::DeleteTable {
            node_id,
            table_uuid,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
                    g.remove_node(idx);
                }
            });
        }
        GraphOperation::RenameTable {
            node_id,
            table_uuid,
            new_name,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
                    if let Some(node) = g.node_weight_mut(idx) {
                        node.name = new_name;
                    }
                }
            });
        }
        GraphOperation::MoveTable {
            node_id,
            table_uuid,
            position,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
                    if let Some(node) = g.node_weight_mut(idx) {
                        node.position = position;
                    }
                }
            });
        }
        GraphOperation::AddColumn {
            node_id,
            table_uuid,
            column,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
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
                }
            });
        }
        GraphOperation::UpdateColumn {
            node_id,
            table_uuid,
            column_index,
            column,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
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
                }
            });
        }
        GraphOperation::DeleteColumn {
            node_id,
            table_uuid,
            column_index,
        } => {
            graph.update(|g| {
                // Try to find by UUID first, then fall back to node_id
                let idx = find_node_by_uuid(g, table_uuid).or_else(|| {
                    let idx = NodeIndex::new(node_id as usize);
                    if g.node_weight(idx).is_some() {
                        Some(idx)
                    } else {
                        None
                    }
                });

                if let Some(idx) = idx {
                    if let Some(node) = g.node_weight_mut(idx) {
                        if column_index < node.columns.len() {
                            node.columns.remove(column_index);
                        }
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
                    )
                    .with_actions(&relationship.on_delete, &relationship.on_update);

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
#[allow(dead_code)]
fn apply_graph_state(graph: RwSignal<SchemaGraph>, state: GraphStateSnapshot) {
    graph.update(|g| {
        apply_graph_state_internal(g, state);
    });
}

/// Internal function to apply graph state without going through signal
#[cfg(not(feature = "ssr"))]
fn apply_graph_state_internal(g: &mut SchemaGraph, state: GraphStateSnapshot) {
    use crate::core::TableNode;

    // When receiving graph state from a LiveShare session, we should:
    // 1. Clear ALL local tables and relationships
    // 2. Apply the state from the session
    // This ensures that joining a session replaces local work with session state

    let had_local_data = g.node_count() > 0;

    if had_local_data {
        leptos::logging::log!(
            "Replacing {} local tables with {} tables from LiveShare session",
            g.node_count(),
            state.tables.len()
        );
    }

    // Clear the entire graph
    *g = SchemaGraph::new();

    // Apply tables from snapshot
    for table in state.tables {
        let mut node =
            TableNode::new(&table.name).with_position(table.position.0, table.position.1);

        // Preserve the UUID from the snapshot
        node.uuid = table.table_uuid;

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
            )
            .with_actions(&rel_snap.data.on_delete, &rel_snap.data.on_update);

            g.add_edge(from_idx, to_idx, rel);
        }
    }

    if had_local_data {
        leptos::logging::log!(
            "Successfully replaced local state with LiveShare session state: {} tables, {} relationships",
            g.node_count(),
            g.edge_count()
        );
    }
}

/// Create a snapshot of the current graph state
#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
fn create_graph_snapshot(graph: RwSignal<SchemaGraph>) -> GraphStateSnapshot {
    graph.with_untracked(|g| create_graph_snapshot_internal(g))
}

/// Internal function to create a graph snapshot without going through signal
#[cfg(not(feature = "ssr"))]
fn create_graph_snapshot_internal(g: &SchemaGraph) -> GraphStateSnapshot {
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
                    table_uuid: node.uuid,
                    name: node.name.clone(),
                    position: node.position,
                    columns,
                    version: 0,
                    last_modified_at: 0,
                    is_deleted: false,
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
                    on_delete: edge.on_delete.clone(),
                    on_update: edge.on_update.clone(),
                },
                version: 0,
                last_modified_at: 0,
                is_deleted: false,
            })
        })
        .collect();

    GraphStateSnapshot {
        tables,
        relationships,
    }
}
