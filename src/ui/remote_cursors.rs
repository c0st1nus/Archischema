//! Remote Cursors component for displaying other users' cursors
//!
//! This component renders the cursors of other users in the collaborative
//! editing session, similar to how Excalidraw and Figma show collaborator cursors.

use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
use leptos::prelude::*;
use uuid::Uuid;

/// Sidebar width in pixels when expanded (ml-96 = 24rem = 384px)
const SIDEBAR_WIDTH_EXPANDED: f64 = 384.0;
/// Sidebar width in pixels when collapsed (ml-14 = 3.5rem = 56px)
const SIDEBAR_WIDTH_COLLAPSED: f64 = 56.0;

/// Remote cursors overlay component
///
/// This component should be placed inside the canvas container
/// and will render cursors for all remote users.
#[component]
pub fn RemoteCursors(
    #[prop(into)] zoom: Signal<f64>,
    #[prop(into)] pan_x: Signal<f64>,
    #[prop(into)] pan_y: Signal<f64>,
    #[prop(into)] sidebar_collapsed: Signal<bool>,
) -> impl IntoView {
    let ctx = use_liveshare_context();

    // Get list of user IDs that have cursors
    let cursor_user_ids = Memo::new(move |_| {
        if ctx.connection_state.get() != ConnectionState::Connected {
            return Vec::new();
        }

        ctx.remote_users.with(|users| {
            users
                .iter()
                .filter(|u| u.cursor.is_some())
                .map(|u| u.user_id)
                .collect::<Vec<_>>()
        })
    });

    view! {
        // z-[15] puts cursors above tables (z-10) but below sidebar (z-20) and other UI
        <div class="pointer-events-none fixed inset-0 z-[15]">
            <For
                each=move || cursor_user_ids.get()
                key=|user_id| *user_id
                children=move |user_id| {
                    view! {
                        <CursorView
                            user_id=user_id
                            zoom=zoom
                            pan_x=pan_x
                            pan_y=pan_y
                            sidebar_collapsed=sidebar_collapsed
                        />
                    }
                }
            />
        </div>
    }
}

/// Individual user cursor component with smooth animation
/// Looks up cursor data reactively so it updates when position changes
#[component]
fn CursorView(
    user_id: Uuid,
    #[prop(into)] zoom: Signal<f64>,
    #[prop(into)] pan_x: Signal<f64>,
    #[prop(into)] pan_y: Signal<f64>,
    #[prop(into)] sidebar_collapsed: Signal<bool>,
) -> impl IntoView {
    let ctx = use_liveshare_context();

    // Reactively get cursor data for this user
    let cursor_data = Memo::new(move |_| {
        ctx.remote_users.with(|users| {
            users.iter().find(|u| u.user_id == user_id).and_then(|u| {
                // Get activity status from activity_status signal for this user
                // We need to determine status based on is_active field
                // is_active=true means Active, is_active=false means Idle or Away
                u.cursor
                    .as_ref()
                    .map(|cursor| (u.username.clone(), u.color.clone(), *cursor, u.is_active))
            })
        })
    });

    view! {
        {move || {
            let data = cursor_data.get();
            match data {
                Some((username, color, (canvas_x, canvas_y), is_active)) => {
                    let opacity = if is_active { "1" } else { "0.5" };
                    let label_style = format!(
                        "background-color: {}; opacity: {}; box-shadow: 0 1px 3px rgba(0,0,0,0.2);",
                        color,
                        opacity
                    );
                    let username_clone = username.clone();
                    let status_text = if is_active { "editing" } else { "idle" };

                    view! {
                        <div
                            class="absolute pointer-events-none cursor-wrapper"
                            style:left=move || {
                                let current_zoom = zoom.get();
                                let current_pan_x = pan_x.get();
                                let sidebar_width = if sidebar_collapsed.get() { SIDEBAR_WIDTH_COLLAPSED } else { SIDEBAR_WIDTH_EXPANDED };
                                let viewport_x = canvas_x * current_zoom + current_pan_x + sidebar_width;
                                format!("{}px", viewport_x)
                            }
                            style:top=move || {
                                let current_zoom = zoom.get();
                                let current_pan_y = pan_y.get();
                                let viewport_y = canvas_y * current_zoom + current_pan_y;
                                format!("{}px", viewport_y)
                            }
                            style:transition="left 50ms linear, top 50ms linear"
                        >
                            // Cursor pointer SVG - classic arrow shape
                            <svg
                                width="16"
                                height="20"
                                viewBox="0 0 16 20"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                                class="drop-shadow-sm"
                                style="filter: drop-shadow(0 1px 1px rgba(0,0,0,0.3))"
                            >
                                // Arrow cursor path
                                <path
                                    d="M0 0L0 16L4.5 12L7.5 19L10 18L7 11L12 11L0 0Z"
                                    fill=color.clone()
                                />
                                <path
                                    d="M0.5 1.2L0.5 14.3L4.2 11L4.7 10.5L5.3 10.7L7.8 16.8L8.8 16.4L6.3 10.3L5.9 9.5H6.8H10.2L0.5 1.2Z"
                                    stroke="white"
                                    stroke-width="1"
                                />
                            </svg>

                            // Username label - positioned to the right of cursor with activity indicator
                            <div
                                class="absolute left-4 top-3 px-2 py-1 rounded-md text-xs font-medium text-white whitespace-nowrap flex items-center gap-1"
                                style=label_style
                                title={move || {
                                    if is_active {
                                        format!("{} is editing", username_clone)
                                    } else {
                                        format!("{} is {}", username_clone, status_text)
                                    }
                                }}
                            >
                                <span class={move || {
                                    if is_active {
                                        "w-1.5 h-1.5 rounded-full bg-white animate-pulse"
                                    } else {
                                        "w-1.5 h-1.5 rounded-full bg-white/50"
                                    }
                                }}></span>
                                {username}
                            </div>
                        </div>
                    }.into_any()
                }
                _ => view! { <div class="hidden"></div> }.into_any()
            }
        }}
    }
}

/// Component to track and send local cursor position
/// Converts viewport coordinates to canvas coordinates before sending
#[component]
pub fn CursorTracker(
    #[prop(into)] zoom: Signal<f64>,
    #[prop(into)] pan_x: Signal<f64>,
    #[prop(into)] pan_y: Signal<f64>,
    #[prop(into)] sidebar_collapsed: Signal<bool>,
) -> impl IntoView {
    // Suppress unused warnings for SSR builds
    let _ = (&zoom, &pan_x, &pan_y, &sidebar_collapsed);

    #[cfg(not(feature = "ssr"))]
    {
        use crate::ui::liveshare_client::ConnectionState;
        use leptos::wasm_bindgen::{JsCast, closure::Closure};

        let ctx = use_liveshare_context();

        Effect::new(move |_| {
            // Only track when connected
            if ctx.connection_state.get() != ConnectionState::Connected {
                return;
            }

            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");

            // Throttle cursor updates (send at most every 30ms for smoother experience)
            let last_update = std::rc::Rc::new(std::cell::RefCell::new(0.0_f64));

            let ctx_move = ctx;
            let last_update_move = last_update.clone();

            let mousemove = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                let now = js_sys::Date::now();
                let last = *last_update_move.borrow();

                // Throttle to 30ms (~33 updates per second) for smoother cursor movement
                if now - last < 30.0 {
                    return;
                }
                *last_update_move.borrow_mut() = now;

                // Get viewport coordinates
                let viewport_x = e.client_x() as f64;
                let viewport_y = e.client_y() as f64;

                // Get current transform values
                let current_zoom = zoom.get_untracked();
                let current_pan_x = pan_x.get_untracked();
                let current_pan_y = pan_y.get_untracked();

                // Convert viewport coordinates to canvas coordinates
                // Reverse the transform: viewport = canvas * zoom + pan + sidebar_offset
                // Therefore: canvas = (viewport - sidebar_offset - pan) / zoom
                let sidebar_width = if sidebar_collapsed.get_untracked() {
                    SIDEBAR_WIDTH_COLLAPSED
                } else {
                    SIDEBAR_WIDTH_EXPANDED
                };
                let canvas_x = (viewport_x - sidebar_width - current_pan_x) / current_zoom;
                let canvas_y = (viewport_y - current_pan_y) / current_zoom;

                ctx_move.send_awareness(Some((canvas_x, canvas_y)), vec![]);
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            let ctx_leave = ctx;
            let mouseleave = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                // Send None cursor when mouse leaves the window
                ctx_leave.send_awareness(None, vec![]);
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            let _ = document
                .add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref());
            let _ = document.add_event_listener_with_callback(
                "mouseleave",
                mouseleave.as_ref().unchecked_ref(),
            );

            // Leak the closures to keep them alive
            // In a real app, you'd want to clean these up properly
            mousemove.forget();
            mouseleave.forget();
        });
    }

    view! {
        // This component doesn't render anything visible
        // It just sets up the cursor tracking
        <div class="hidden"></div>
    }
}
