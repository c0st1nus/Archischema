//! Editor page component
//!
//! The diagram editor page that wraps SchemaCanvas and handles
//! diagram loading, saving, and persistence.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::core::SchemaGraph;
use crate::ui::auth::{AuthState, use_auth_context};
use crate::ui::canvas::SchemaCanvas;
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

    // Get diagram ID from URL params
    let diagram_id = Memo::new(move |_| params.get().get("id").unwrap_or_default());

    // Diagram state
    let graph = RwSignal::new(SchemaGraph::new());
    let diagram_name = RwSignal::new(String::from("Untitled Diagram"));
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let save_status = RwSignal::new(SaveStatus::Saved);
    let is_demo = Memo::new(move |_| diagram_id.get() == "demo");

    // Handle diagram name change
    let on_name_change = Callback::new(move |new_name: String| {
        let id = diagram_id.get();
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

    // Autosave effect
    Effect::new(move |prev_graph: Option<String>| {
        let id = diagram_id.get();

        // Don't autosave demo or empty ID
        if id == "demo" || id.is_empty() {
            return "".to_string();
        }

        // Don't autosave if not authenticated
        if !matches!(auth.state.get(), AuthState::Authenticated(_)) {
            return "".to_string();
        }

        let current_graph = serde_json::to_string(&graph.get()).unwrap_or_default();

        // Skip first render and unchanged graphs
        if let Some(prev) = prev_graph
            && prev != current_graph
            && !prev.is_empty()
        {
            // Graph changed, trigger autosave
            save_status.set(SaveStatus::Saving);

            let id_clone = id.clone();
            let graph_clone = current_graph.clone();

            spawn_local(async move {
                // Debounce - wait 2 seconds
                #[cfg(not(feature = "ssr"))]
                {
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                }

                match autosave_diagram(&id_clone, &graph_clone).await {
                    Ok(_) => {
                        save_status.set(SaveStatus::Saved);
                    }
                    Err(_) => {
                        save_status.set(SaveStatus::Error);
                    }
                }
            });
        }

        current_graph
    });

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
