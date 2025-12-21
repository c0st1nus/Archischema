#![recursion_limit = "4096"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use archischema::app::*;
    use archischema::core::ai_api::{AiApiConfig, ai_api_router};
    use archischema::core::auth::{
        AuthApiState, AuthService, JwtConfig, JwtService, auth_api_router,
    };
    use archischema::core::config::Config;
    use archischema::core::db::{
        DbConfig, DiagramRepository, FolderRepository, SessionRepository, ShareRepository,
        UserRepository, create_pool_with_migrations,
    };
    use archischema::core::diagrams::{DiagramApiState, diagram_api_router};
    use archischema::core::folders::{FolderApiState, folder_api_router};
    use archischema::core::liveshare::{
        LiveshareState, init_jwt_service, liveshare_router, ws_handler,
    };
    use archischema::core::sharing::{ShareApiState, share_api_router};
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use tower_http::services::ServeDir;

    // Load .env file (if exists)
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load application config from environment variables
    let config = Config::from_env();

    // Load AI API config
    let ai_config = AiApiConfig::from_env();

    // Initialize database connection pool
    let db_pool = match DbConfig::from_env() {
        Ok(db_config) => match create_pool_with_migrations(&db_config).await {
            Ok(pool) => {
                tracing::info!("Database connected and migrations applied");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to database: {}. Auth features disabled.",
                    e
                );
                None
            }
        },
        Err(_) => {
            tracing::warn!("DATABASE_URL not set. Auth features disabled.");
            None
        }
    };

    // Initialize JWT service
    let jwt_service = match JwtConfig::from_env() {
        Ok(jwt_config) => {
            tracing::info!("JWT configured with custom secret");
            Some(JwtService::new(jwt_config))
        }
        Err(_) => {
            // Use a default secret for development (NOT for production!)
            if cfg!(debug_assertions) {
                tracing::warn!("JWT_SECRET not set. Using default secret (development only!)");
                Some(JwtService::new(JwtConfig::new(
                    "archischema_dev_secret_key_not_for_production_32chars",
                )))
            } else {
                tracing::error!("JWT_SECRET must be set in production!");
                None
            }
        }
    };

    // Initialize JWT service for LiveShare authentication
    init_jwt_service(jwt_service.clone());

    // Log config status (without revealing secrets)
    tracing::info!(
        "Config loaded: database={}, redis={}, secret_key={}, ai_token={}, jwt={}",
        config.has_database(),
        config.has_redis(),
        config.has_secret_key(),
        ai_config.has_token(),
        jwt_service.is_some()
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
    // This serves .br (brotli) and .gz (gzip) files automatically when available
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

    // Build the AI API router
    let ai_api = ai_api_router(ai_config);

    // Build the Auth, Diagram, Folder, and Share API routers (only if database and JWT are configured)
    let (auth_api, diagram_api, folder_api, share_api) = if let (Some(pool), Some(jwt)) =
        (db_pool, jwt_service)
    {
        // Auth API
        let user_repo = UserRepository::new(pool.clone());
        let session_repo = SessionRepository::new(pool.clone());
        let auth_service = AuthService::new(user_repo, session_repo, jwt.clone());
        let auth_state = AuthApiState { auth_service };
        let auth_router = auth_api_router(auth_state);

        // Diagram API
        let diagram_repo = DiagramRepository::new(pool.clone());
        let diagram_state = DiagramApiState {
            diagram_repo,
            jwt_service: jwt.clone(),
        };
        let diagram_router = diagram_api_router(diagram_state);

        // Folder API
        let folder_repo = FolderRepository::new(pool.clone());
        let folder_state = FolderApiState {
            folder_repo,
            jwt_service: jwt.clone(),
        };
        let folder_router = folder_api_router(folder_state);

        // Share API
        let share_repo = ShareRepository::new(pool);
        let share_state = ShareApiState {
            share_repo,
            jwt_service: jwt,
        };
        let share_router = share_api_router(share_state);

        (
            Some(auth_router),
            Some(diagram_router),
            Some(folder_router),
            Some(share_router),
        )
    } else {
        tracing::warn!(
            "Auth, Diagram, Folder, and Share APIs disabled due to missing database or JWT configuration"
        );
        (None, None, None, None)
    };

    // Build the main application router
    let mut app = Router::new()
        // WebSocket endpoint for real-time sync: ws://{host}/room/{room_id}
        .route(
            "/room/{room_id}",
            axum::routing::get(ws_handler).with_state(liveshare_state.clone()),
        )
        // REST API for room management
        .merge(liveshare_api)
        // AI API for chat completions
        .merge(ai_api);

    // Merge auth API if available
    if let Some(auth_router) = auth_api {
        app = app.merge(auth_router);
        tracing::info!("Auth API enabled");
    }

    // Merge diagram API if available
    if let Some(diagram_router) = diagram_api {
        app = app.merge(diagram_router);
        tracing::info!("Diagram API enabled");
    }

    // Merge folder API if available
    if let Some(folder_router) = folder_api {
        app = app.merge(folder_router);
        tracing::info!("Folder API enabled");
    }

    // Merge share API if available
    if let Some(share_router) = share_api {
        app = app.merge(share_router);
        tracing::info!("Share API enabled");
    }

    // Merge Leptos routes (nested to avoid state conflicts)
    let app = app.merge(leptos_router);

    // In release mode, add on-the-fly compression for responses
    // In debug mode, skip compression - use pre-compressed files if available,
    // otherwise serve uncompressed (faster for localhost development)
    #[cfg(debug_assertions)]
    let app = {
        log!("Debug mode: on-the-fly compression disabled");
        log!("Pre-compressed .br/.gz files will be served if available");
        app
    };

    #[cfg(not(debug_assertions))]
    let app = {
        use tower_http::compression::{CompressionLayer, CompressionLevel};
        log!("Release mode: on-the-fly compression enabled");
        app.layer(
            CompressionLayer::new()
                .br(true) // Brotli - best compression ratio
                .gzip(true) // Gzip - wide support fallback
                .quality(CompressionLevel::Best),
        )
    };

    // Run our app with hyper
    log!("listening on http://{}", &addr);
    log!("LiveShare REST API: http://{}/room/{{uuid}}", &addr);
    log!("LiveShare WebSocket: ws://{}/room/{{uuid}}", &addr);
    log!("AI Chat API: http://{}/api/ai/chat", &addr);
    log!("Auth API: http://{}/api/auth/*", &addr);
    log!("Diagram API: http://{}/api/diagrams/*", &addr);
    log!("Folder API: http://{}/api/folders/*", &addr);
    log!("Share API: http://{}/api/diagrams/{{id}}/shares/*", &addr);

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
