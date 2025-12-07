#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use archischema::app::*;
    use archischema::core::config::Config;
    use archischema::core::liveshare::{LiveshareState, liveshare_router, ws_handler};
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use tower_http::compression::{CompressionLayer, CompressionLevel};
    use tower_http::services::ServeDir;

    // Load .env file (if exists)
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load application config from environment variables
    let config = Config::from_env();

    // Log config status (without revealing secrets)
    tracing::info!(
        "Config loaded: database={}, redis={}, secret_key={}",
        config.has_database(),
        config.has_redis(),
        config.has_secret_key()
    );

    // Load configuration from Cargo.toml [package.metadata.leptos]
    // Can be overridden via LEPTOS_SITE_ADDR env var for Docker/K8s
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Create LiveShare state
    // Use the server address for WebSocket URLs
    let liveshare_state = LiveshareState::with_host(addr.to_string(), false);

    // Create ServeDir for pkg with pre-compressed file support
    // This serves .br (brotli) and .gz (gzip) files automatically
    let pkg_service = ServeDir::new(format!("{}/pkg", leptos_options.site_root))
        .precompressed_br()
        .precompressed_gzip();

    // Build the Leptos router
    let leptos_router = Router::new()
        // Serve pre-compressed static assets from /pkg
        .nest_service("/pkg", pkg_service)
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // Build the LiveShare API router
    let liveshare_api = liveshare_router(liveshare_state.clone());

    // Build the main application router with compression
    let app = Router::new()
        // WebSocket endpoint for real-time sync: ws://{host}/room/{room_id}
        .route(
            "/room/{room_id}",
            axum::routing::get(ws_handler).with_state(liveshare_state.clone()),
        )
        // REST API for room management
        .merge(liveshare_api)
        // Leptos routes (nested to avoid state conflicts)
        .merge(leptos_router)
        // Add compression with Brotli priority (best compression for web)
        // Compresses responses > 1KB, skips already compressed formats
        .layer(
            CompressionLayer::new()
                .br(true) // Brotli - best compression ratio
                .gzip(true) // Gzip - wide support fallback
                .quality(CompressionLevel::Best),
        );

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
