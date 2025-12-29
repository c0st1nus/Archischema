//! Editor page component
//!
//! The diagram editor page that wraps SchemaCanvas and handles
//! diagram loading, saving, and persistence.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_params_map, use_query_map};

use crate::core::SchemaGraph;
use crate::ui::auth::{AuthState, use_auth_context};
use crate::ui::canvas::SchemaCanvas;
use crate::ui::liveshare_client::use_liveshare_context;
use leptos::prelude::Show;

/// Update diagram name via API
#[cfg(not(feature = "ssr"))]
async fn update_diagram_name(id: &str, name: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    // Debug logging
    web_sys::console::log_1(&format!("Updating diagram name: id={}, name={}", id, name).into());

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "name": name
    });

    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_body(&body.to_string().into());

    let req = Request::new_with_str_and_init(&format!("/api/diagrams/{}", id), &opts)
        .map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        let status = resp.status();
        web_sys::console::error_1(
            &format!("Failed to update diagram name: status={}", status).into(),
        );
        return Err(format!("Failed to update diagram name: status {}", status));
    }

    web_sys::console::log_1(&"Diagram name updated successfully".into());
    Ok(())
}

#[cfg(feature = "ssr")]
#[allow(dead_code)]
async fn update_diagram_name(_id: &str, _name: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

/// Editor page component
#[component]
pub fn EditorPage() -> impl IntoView {
    let auth = use_auth_context();
    let params = use_params_map();
    let query = use_query_map();
    let liveshare_ctx = use_liveshare_context();

    // Get diagram ID from URL params
    let diagram_id = Memo::new(move |_| params.get().get("id").unwrap_or_default());

    // Get room ID from query params
    let room_id_from_url = Memo::new(move |_| query.get().get("room").map(|s| s.to_string()));

    // Diagram state
    let graph = RwSignal::new(SchemaGraph::new());
    let diagram_name = RwSignal::new(String::from("Untitled Diagram"));
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let save_status = RwSignal::new(SaveStatus::Saved);
    let is_demo = Memo::new(move |_| diagram_id.get() == "demo");

    // Handle diagram name change
    let on_name_change = Callback::new(move |new_name: String| {
        let id = diagram_id.get_untracked();
        if id.is_empty() || id == "demo" {
            return;
        }
        #[cfg(not(feature = "ssr"))]
        {
            let id_clone = id.clone();
            let name_clone = new_name.clone();
            spawn_local(async move {
                let _ = update_diagram_name(&id_clone, &name_clone).await;
            });
        }
        #[cfg(feature = "ssr")]
        {
            let _ = (id, new_name); // Suppress unused variable warnings
        }
    });

    // Load diagram on mount
    Effect::new(move |_| {
        let id = diagram_id.get();

        // Demo mode - don't load from API
        if id == "demo" {
            graph.set(crate::core::create_demo_graph());
            diagram_name.set("Demo Diagram".to_string());
            loading.set(false);
            return;
        }

        // Skip loading if no ID
        if id.is_empty() {
            loading.set(false);
            return;
        }

        // Load diagram from API if authenticated
        if let AuthState::Authenticated(_) = auth.state.get() {
            spawn_local(async move {
                loading.set(true);
                error.set(None);

                match fetch_diagram(&id).await {
                    Ok(diagram) => {
                        diagram_name.set(diagram.name);
                        if let Some(schema) = diagram.schema_data
                            && let Ok(parsed) = serde_json::from_value(schema)
                        {
                            graph.set(parsed);
                        }
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }

                loading.set(false);
            });
        } else {
            // Not authenticated and not demo - show empty
            loading.set(false);
        }
    });

    // Auto-connect to LiveShare room if room parameter is present
    // Wait for auth to finish loading to ensure username is set correctly
    Effect::new(move |_| {
        // Track auth state to ensure we wait for it to load
        let auth_state = auth.state.get();

        // Only connect when auth is done loading (either Authenticated or Unauthenticated)
        if matches!(auth_state, AuthState::Loading) {
            return;
        }

        if let Some(room_id) = room_id_from_url.get() {
            // Connect to the room without password (shared links don't use passwords)
            liveshare_ctx.connect(room_id, None);
        }
    });

    // Explicit save function
    #[allow(unused_variables)]
    let perform_save = {
        Callback::new(move |reason: String| {
            let id = diagram_id.get_untracked();

            // Don't save demo or empty ID
            if id == "demo" || id.is_empty() {
                return;
            }

            // Don't save if not authenticated
            if !matches!(auth.state.get_untracked(), AuthState::Authenticated(_)) {
                return;
            }

            save_status.set(SaveStatus::Saving);

            let id_clone = id.clone();
            let graph_clone = serde_json::to_string(&graph.get_untracked()).unwrap_or_default();

            leptos::logging::log!("Saving diagram: {}", reason);

            spawn_local(async move {
                match autosave_diagram(&id_clone, &graph_clone).await {
                    Ok(_) => {
                        save_status.set(SaveStatus::Saved);
                        leptos::logging::log!("Diagram saved successfully");
                    }
                    Err(e) => {
                        save_status.set(SaveStatus::Error);
                        leptos::logging::error!("Failed to save diagram: {}", e);
                    }
                }
            });
        })
    };

    // Periodic autosave (every 1 minute)
    #[cfg(not(feature = "ssr"))]
    {
        use gloo_timers::callback::Interval;
        use std::cell::RefCell;
        use std::rc::Rc;

        let save_callback = perform_save.clone();
        let last_save_state = Rc::new(RefCell::new(String::new()));

        Effect::new(move |_| {
            let save_callback = save_callback.clone();
            let last_state = last_save_state.clone();
            let graph_signal = graph;

            // Setup interval for 1 minute (60000ms)
            let interval = Interval::new(60_000, move || {
                let current_state =
                    serde_json::to_string(&graph_signal.get_untracked()).unwrap_or_default();

                // Only save if graph has changed since last save
                if *last_state.borrow() != current_state {
                    save_callback.run("periodic_autosave".to_string());
                    *last_state.borrow_mut() = current_state;
                }
            });

            // Store interval in Rc<RefCell> so we can drop it on cleanup
            let interval_holder = Rc::new(RefCell::new(Some(interval)));
            let interval_clone = interval_holder.clone();

            // Return cleanup function
            move || {
                // Cancel the interval when effect re-runs or component unmounts
                interval_clone.borrow_mut().take();
            }
        });
    }

    // Listen for save events from canvas
    #[cfg(not(feature = "ssr"))]
    {
        use std::cell::RefCell;
        use std::rc::Rc;

        let save_callback = perform_save.clone();
        Effect::new(move |_| {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::closure::Closure;

            let save_cb = save_callback.clone();
            let handler = Rc::new(RefCell::new(None::<Closure<dyn Fn(web_sys::CustomEvent)>>));
            let handler_clone = handler.clone();

            if let Some(window) = web_sys::window() {
                let closure =
                    Closure::<dyn Fn(web_sys::CustomEvent)>::new(move |e: web_sys::CustomEvent| {
                        if let Some(reason) = e.detail().as_string() {
                            save_cb.run(reason);
                        }
                    });

                let _ = window.add_event_listener_with_callback(
                    "diagram-save-requested",
                    closure.as_ref().unchecked_ref(),
                );

                *handler_clone.borrow_mut() = Some(closure);
            }

            // Return cleanup function
            move || {
                if let Some(window) = web_sys::window() {
                    if let Some(closure) = handler.borrow_mut().take() {
                        let _ = window.remove_event_listener_with_callback(
                            "diagram-save-requested",
                            closure.as_ref().unchecked_ref(),
                        );
                    }
                }
            }
        });
    }

    view! {
        <div class="w-full h-screen bg-theme-primary relative">
            <Show when=move || loading.get()>
                <LoadingOverlay />
            </Show>

            <Show when=move || error.get().is_some()>
                <ErrorBanner error=error />
            </Show>

            <Show when=move || is_demo.get()>
                <DemoBanner />
            </Show>

            <Show when=move || !loading.get()>
                <SchemaCanvas
                    graph=graph
                    diagram_name=diagram_name
                    diagram_id=diagram_id.get()
                    is_demo=is_demo.get()
                    on_name_change=on_name_change
                />
            </Show>
        </div>
    }
}

#[component]
fn LoadingOverlay() -> impl IntoView {
    view! {
        <div class="absolute inset-0 z-50 flex items-center justify-center bg-theme-primary">
            <div class="flex flex-col items-center gap-4">
                <svg class="animate-spin h-8 w-8 text-accent-primary" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <p class="text-theme-secondary">"Loading diagram..."</p>
            </div>
        </div>
    }
}

#[component]
fn ErrorBanner(error: RwSignal<Option<String>>) -> impl IntoView {
    view! {
        <div class="absolute top-4 left-1/2 -translate-x-1/2 z-50 px-4 py-2 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 rounded-lg">
            <p class="text-sm text-red-700 dark:text-red-300">{move || error.get().unwrap_or_default()}</p>
        </div>
    }
}

#[component]
fn DemoBanner() -> impl IntoView {
    view! {
        <div class="absolute top-4 left-1/2 -translate-x-1/2 z-40 px-4 py-2 bg-yellow-100 dark:bg-yellow-900/30 border border-yellow-300 dark:border-yellow-700 rounded-lg">
            <p class="text-sm text-yellow-700 dark:text-yellow-300">
                "Demo mode — changes will not be saved. "
                <a href="/register" class="underline hover:no-underline">"Sign up"</a>
                " to save your diagrams."
            </p>
        </div>
    }
}

/// Save status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Saving,
    Error,
}

/// Diagram data from API
#[derive(Debug, Clone, serde::Deserialize)]
struct DiagramResponse {
    #[allow(dead_code)]
    id: String,
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    schema_data: Option<serde_json::Value>,
}

// API functions

#[cfg(not(feature = "ssr"))]
async fn fetch_diagram(id: &str) -> Result<DiagramResponse, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req = Request::new_with_str_and_init(&format!("/api/diagrams/{}", id), &opts)
        .map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to load diagram".to_string());
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

#[cfg(feature = "ssr")]
async fn fetch_diagram(_id: &str) -> Result<DiagramResponse, String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn autosave_diagram(id: &str, schema_json: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "schema_data": serde_json::from_str::<serde_json::Value>(schema_json).unwrap_or(serde_json::json!({}))
    });

    let opts = RequestInit::new();
    opts.set_method("PATCH");
    opts.set_body(&body.to_string().into());

    let req = Request::new_with_str_and_init(&format!("/api/diagrams/{}/autosave", id), &opts)
        .map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to save diagram".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn autosave_diagram(_id: &str, _schema_json: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}
