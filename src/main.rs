#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use diagramix::app::*;
    use diagramix::core::liveshare::{LiveshareState, liveshare_router, ws_handler};
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};

    // Initialize tracing
    tracing_subscriber::fmt::init();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Create LiveShare state
    // Use the server address for WebSocket URLs
    let liveshare_state = LiveshareState::with_host(addr.to_string(), false);

    // Build the Leptos router
    let leptos_router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // Build the LiveShare API router
    let liveshare_api = liveshare_router(liveshare_state.clone());

    // Build the main application router
    let app = Router::new()
        // WebSocket endpoint for real-time sync: ws://{host}/room/{room_id}
        .route(
            "/room/{room_id}",
            axum::routing::get(ws_handler).with_state(liveshare_state.clone()),
        )
        // REST API for room management
        .merge(liveshare_api)
        // Leptos routes (nested to avoid state conflicts)
        .merge(leptos_router);

    // Run our app with hyper
    log!("listening on http://{}", &addr);
    log!("LiveShare REST API: http://{}/room/{{uuid}}", &addr);
    log!("LiveShare WebSocket: ws://{}/room/{{uuid}}", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
