//! Remote Cursors component for displaying other users' cursors
//!
//! This component renders the cursors of other users in the collaborative
//! editing session, similar to how Excalidraw and Figma show collaborator cursors.

use crate::ui::liveshare_client::{ConnectionState, RemoteUser, use_liveshare_context};
use leptos::prelude::*;

/// Sidebar width in pixels (ml-96 = 24rem = 384px)
const SIDEBAR_WIDTH: f64 = 384.0;

/// Remote cursors overlay component
///
/// This component should be placed inside the canvas container
/// and will render cursors for all remote users.
#[component]
pub fn RemoteCursors(
    #[prop(into)] zoom: Signal<f64>,
    #[prop(into)] pan_x: Signal<f64>,
    #[prop(into)] pan_y: Signal<f64>,
) -> impl IntoView {
    let ctx = use_liveshare_context();

    view! {
        <div class="pointer-events-none fixed inset-0 overflow-hidden z-50">
            {move || {
                // Only show cursors when connected
                if ctx.connection_state.get() != ConnectionState::Connected {
                    return view! { <div></div> }.into_any();
                }

                view! {
                    <For
                        each=move || ctx.remote_users.get()
                        key=|user| user.user_id
                        children=move |user| {
                            view! { <UserCursor
                                user=user
                                zoom=zoom
                                pan_x=pan_x
                                pan_y=pan_y
                            /> }
                        }
                    />
                }.into_any()
            }}
        </div>
    }
}

/// Individual user cursor component with smooth animation
#[component]
fn UserCursor(
    user: RemoteUser,
    #[prop(into)] zoom: Signal<f64>,
    #[prop(into)] pan_x: Signal<f64>,
    #[prop(into)] pan_y: Signal<f64>,
) -> impl IntoView {
    let color = user.color.clone();
    let username = user.username.clone();
    let user_id = user.user_id;

    // Create a derived signal to track this user's cursor position
    let ctx = use_liveshare_context();

    let cursor_pos = Memo::new(move |_| {
        ctx.remote_users.with(|users| {
            users
                .iter()
                .find(|u| u.user_id == user_id)
                .and_then(|u| u.cursor)
        })
    });

    let is_active = Memo::new(move |_| {
        ctx.remote_users.with(|users| {
            users
                .iter()
                .find(|u| u.user_id == user_id)
                .map(|u| u.is_active)
                .unwrap_or(false)
        })
    });

    view! {
        {move || {
            let pos = cursor_pos.get();
            let active = is_active.get();
            let color = color.clone();
            let username = username.clone();

            // Get current transform values
            let current_zoom = zoom.get();
            let current_pan_x = pan_x.get();
            let current_pan_y = pan_y.get();

            match pos {
                Some((canvas_x, canvas_y)) => {
                    // Convert canvas coordinates back to viewport coordinates
                    // The remote cursor sends canvas coordinates, we need to display in viewport
                    let viewport_x = canvas_x * current_zoom + current_pan_x + SIDEBAR_WIDTH;
                    let viewport_y = canvas_y * current_zoom + current_pan_y;

                    view! {
                        <div
                            class="absolute pointer-events-none cursor-wrapper"
                            style=move || format!(
                                "left: {}px; top: {}px; transition: left 80ms linear, top 80ms linear;",
                                viewport_x, viewport_y
                            )
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

                            // Username label - positioned to the right of cursor
                            <div
                                class="absolute left-4 top-3 px-2 py-1 rounded-md text-xs font-medium text-white whitespace-nowrap"
                                style=format!(
                                    "background-color: {}; opacity: {}; box-shadow: 0 1px 3px rgba(0,0,0,0.2); transition: opacity 150ms ease;",
                                    color,
                                    if active { "1" } else { "0.7" }
                                )
                            >
                                {username}
                            </div>
                        </div>
                    }.into_any()
                }
                None => {
                    // User has no cursor position, don't render anything
                    view! { <div></div> }.into_any()
                }
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
) -> impl IntoView {
    // Suppress unused warnings for SSR builds
    let _ = (&zoom, &pan_x, &pan_y);

    #[cfg(not(feature = "ssr"))]
    let ctx = use_liveshare_context();

    // Track mouse movement on the canvas
    #[cfg(not(feature = "ssr"))]
    {
        use leptos::wasm_bindgen::{JsCast, closure::Closure};

        Effect::new(move |_| {
            let ctx = ctx.clone();

            // Only track when connected
            if ctx.connection_state.get() != ConnectionState::Connected {
                return;
            }

            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");

            // Throttle cursor updates (send at most every 30ms for smoother experience)
            let last_update = std::rc::Rc::new(std::cell::RefCell::new(0.0_f64));

            let ctx_move = ctx.clone();
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

                // Only track if cursor is in the canvas area (past the sidebar)
                if viewport_x < SIDEBAR_WIDTH {
                    // Cursor is over the sidebar, don't send
                    return;
                }

                // Get current transform values
                let current_zoom = zoom.get_untracked();
                let current_pan_x = pan_x.get_untracked();
                let current_pan_y = pan_y.get_untracked();

                // Convert viewport coordinates to canvas coordinates
                // Reverse the transform: viewport = canvas * zoom + pan + sidebar_offset
                // Therefore: canvas = (viewport - sidebar_offset - pan) / zoom
                let canvas_x = (viewport_x - SIDEBAR_WIDTH - current_pan_x) / current_zoom;
                let canvas_y = (viewport_y - current_pan_y) / current_zoom;

                ctx_move.send_awareness(Some((canvas_x, canvas_y)), vec![]);
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            let ctx_leave = ctx.clone();
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
