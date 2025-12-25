//! Activity Tracker component for monitoring user activity
//!
//! This component tracks user activity (mousemove, keypress) and page visibility
//! to detect idle and away states. It sends idle status updates to the server.

use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;

#[cfg(not(feature = "ssr"))]
use gloo_timers::callback::Interval;

/// Activity tracker component
///
/// This component should be placed at the root of the application and sets up
/// event listeners for tracking user activity and updating idle status.
#[component]
pub fn ActivityTracker() -> impl IntoView {
    #[cfg(not(feature = "ssr"))]
    {
        use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
        use wasm_bindgen::closure::Closure;

        let ctx = use_liveshare_context();

        Effect::new(move |_| {
            // Only track when connected
            if ctx.connection_state.get() != ConnectionState::Connected {
                return;
            }

            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");

            // Activity detection: mousemove and keypress
            let ctx_activity = ctx;
            let mousemove = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                ctx_activity.record_activity();
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            let ctx_activity = ctx;
            let keypress = Closure::wrap(Box::new(move |_: web_sys::KeyboardEvent| {
                ctx_activity.record_activity();
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            // Page visibility detection
            let ctx_visible = ctx;
            let visibility_change = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let document = web_sys::window()
                    .expect("no window")
                    .document()
                    .expect("no document");

                let is_visible = !document.hidden();
                if is_visible {
                    ctx_visible.record_page_visible();
                } else {
                    ctx_visible.record_page_hidden();
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            // Register event listeners
            let _ = document
                .add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref());
            let _ = document
                .add_event_listener_with_callback("keypress", keypress.as_ref().unchecked_ref());
            let _ = document.add_event_listener_with_callback(
                "visibilitychange",
                visibility_change.as_ref().unchecked_ref(),
            );

            // Leak the closures to keep them alive
            mousemove.forget();
            keypress.forget();
            visibility_change.forget();

            // Update activity status periodically (every 5 seconds)
            let ctx_update = ctx;
            let _interval = Interval::new(5000, move || {
                ctx_update.update_activity_status();
            });
            // Keep interval alive for the duration of the component
            std::mem::forget(_interval);
        });
    }

    #[cfg(feature = "ssr")]
    {
        // SSR stub - no activity tracking on server
    }

    view! {
        // This component doesn't render anything visible
        <div class="hidden"></div>
    }
}
