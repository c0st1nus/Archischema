//! LiveShare Panel component for room management
//!
//! Provides UI for:
//! - Creating new rooms
//! - Joining existing rooms
//! - Viewing connected users
//! - Room settings
use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
use crate::ui::{
    ConnectionStatusBar, Icon, SnapshotSaveIndicator, SyncStatusBadge, UserPresenceIndicator, icons,
};
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use leptos::wasm_bindgen;
#[cfg(not(feature = "ssr"))]
use leptos::web_sys;

/// LiveShare panel component
#[component]
pub fn LiveSharePanel() -> impl IntoView {
    // Get the LiveShare context
    let ctx = use_liveshare_context();

    // Panel open/closed state
    let (is_open, set_is_open) = signal(false);

    // Room ID input
    let (room_id_input, set_room_id_input) = signal(String::new());

    // Room name input (for creating)
    let (room_name, set_room_name) = signal(String::new());

    // Password input
    let (password, set_password) = signal(String::new());

    // Mode: "join" or "create"
    let (mode, set_mode) = signal("join");

    // Generate a new room ID
    let generate_room_id = move |_| {
        let id = uuid::Uuid::new_v4().to_string();
        set_room_id_input.set(id);
    };

    // Create room handler
    let ctx_create = ctx;
    let create_room = move |_| {
        let room_id_val = room_id_input.get();
        if room_id_val.is_empty() {
            ctx_create
                .error
                .set(Some("Please enter or generate a Room ID".to_string()));
            return;
        }

        ctx_create.error.set(None);

        // First create the room via REST API, then connect
        #[cfg(not(feature = "ssr"))]
        {
            use leptos::task::spawn_local;

            let ctx_inner = ctx_create.clone();
            let room_id = room_id_val.clone();
            let room_name_val = room_name.get();
            let password_val = password.get();

            spawn_local(async move {
                // Create room via REST API
                let window = web_sys::window().expect("no window");
                let location = window.location();
                let origin = location.origin().unwrap_or_default();

                let create_url = format!("{}/room/{}", origin, room_id);

                let body = serde_json::json!({
                    "name": if room_name_val.is_empty() { None } else { Some(room_name_val) },
                    "password": if password_val.is_empty() { None } else { Some(&password_val) },
                    "max_users": 50
                });

                // Use fetch to create the room
                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                opts.set_body(&wasm_bindgen::JsValue::from_str(
                    &serde_json::to_string(&body).unwrap(),
                ));

                let request = web_sys::Request::new_with_str_and_init(&create_url, &opts).unwrap();
                request
                    .headers()
                    .set("Content-Type", "application/json")
                    .unwrap();
                request
                    .headers()
                    .set("X-User-ID", &ctx_inner.user_id.get_untracked().to_string())
                    .unwrap();
                request
                    .headers()
                    .set("X-Username", &ctx_inner.username.get_untracked())
                    .unwrap();

                let window = web_sys::window().unwrap();
                match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                    .await
                {
                    Ok(resp) => {
                        let resp: web_sys::Response = resp.into();
                        if resp.ok() {
                            // Room created, now connect via WebSocket
                            let pwd = if password_val.is_empty() {
                                None
                            } else {
                                Some(password_val)
                            };
                            ctx_inner.connect(room_id, pwd);
                        } else {
                            ctx_inner
                                .error
                                .set(Some(format!("Failed to create room: {}", resp.status())));
                            ctx_inner.connection_state.set(ConnectionState::Error);
                        }
                    }
                    Err(e) => {
                        ctx_inner.error.set(Some(format!("Network error: {:?}", e)));
                        ctx_inner.connection_state.set(ConnectionState::Error);
                    }
                }
            });
        }
    };

    // Join room handler
    let ctx_join = ctx;
    let join_room = move |_| {
        let room_id_val = room_id_input.get();
        if room_id_val.is_empty() {
            ctx_join
                .error
                .set(Some("Please enter a Room ID".to_string()));
            return;
        }

        ctx_join.error.set(None);
        let password_val = password.get();
        let pwd = if password_val.is_empty() {
            None
        } else {
            Some(password_val)
        };
        ctx_join.connect(room_id_val, pwd);
    };

    // Disconnect handler
    let ctx_disconnect = ctx;
    let disconnect = move |_| {
        ctx_disconnect.disconnect();
        set_room_id_input.set(String::new());
        set_password.set(String::new());
    };

    // Copy room link to clipboard
    let _ctx_copy = ctx;
    let copy_link = move |_| {
        #[cfg(not(feature = "ssr"))]
        {
            if let Some(room_id) = _ctx_copy.room_id.get() {
                if let Some(window) = web_sys::window() {
                    let location = window.location();
                    // Build URL manually to ensure port is included
                    let protocol = location.protocol().unwrap_or_default();
                    let host = location.host().unwrap_or_default(); // host includes port
                    let link = format!("{}//{}?room={}", protocol, host, room_id);
                    let js_code = format!("navigator.clipboard.writeText('{}')", link);
                    let _ = js_sys::eval(&js_code);
                }
            }
        }
    };

    // Derived signals
    let connection_state = ctx.connection_state;
    let error = ctx.error;
    let room_id = ctx.room_id;
    let room_info = ctx.room_info;

    view! {
        <div class="absolute top-4 right-4 z-50 flex flex-col gap-2">
            // Top status bar with connection and sync indicators
            <ConnectionStatusBar />

            // Toggle button
            <button
                class="btn-secondary shadow-theme-lg"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                {move || {
                    let state = connection_state.get();
                    match state {
                        ConnectionState::Connected => view! {
                            <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                            <span class="label-sm">"Connected"</span>
                        }.into_any(),
                        ConnectionState::Connecting => view! {
                            <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                            <span class="label-sm">"Connecting..."</span>
                        }.into_any(),
                        ConnectionState::Reconnecting => view! {
                            <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                            <span class="label-sm">"Reconnecting..."</span>
                        }.into_any(),
                        ConnectionState::Error => view! {
                            <div class="w-2 h-2 bg-red-500 rounded-full"></div>
                            <span class="label-sm">"Error"</span>
                        }.into_any(),
                        ConnectionState::Disconnected => view! {
                            <div class="w-2 h-2 bg-gray-400 rounded-full"></div>
                            <span class="label-sm">"LiveShare"</span>
                        }.into_any(),
                    }
                }}
                <div
                    class="icon-text text-theme-tertiary transition-transform duration-200"
                    class=("rotate-180", move || is_open.get())
                >
                    <Icon name=icons::CHEVRON_DOWN class="icon-text" />
                </div>
            </button>

            // Dropdown panel
            {move || {
                if !is_open.get() {
                    return view! { <div class="hidden"></div> }.into_any();
                }

                let state = connection_state.get();

                if state == ConnectionState::Connected {
                    // Connected view
                    let ctx_users = ctx;
                    view! {
                        <div class="mt-2 w-80 card shadow-theme-xl overflow-hidden">
                            // Header with sync status
                            <div class="px-4 py-3 bg-gradient-to-r from-green-500 to-emerald-600 text-white">
                                <div class="flex items-center justify-between mb-2">
                                    <h3 class="font-semibold">"Connected"</h3>
                                    <button
                                        class="text-white/80 hover:text-white text-sm"
                                        on:click={disconnect}
                                    >
                                        "Disconnect"
                                    </button>
                                </div>
                                <div class="flex items-center justify-between gap-2">
                                    <div>
                                        {move || room_info.get().map(|info| view! {
                                            <p class="text-sm text-white/80">{info.name}</p>
                                        })}
                                    </div>
                                    <UserPresenceIndicator />
                                </div>
                            </div>

                            // Status bar with sync and snapshot info
                            <div class="px-4 py-2 bg-theme-secondary/20 border-b border-theme-tertiary flex items-center justify-between gap-2 text-xs">
                                <div class="flex items-center gap-3">
                                    <SyncStatusBadge />
                                    <SnapshotSaveIndicator />
                                </div>
                            </div>

                            // Room info
                            <div class="p-4 border-b border-theme-tertiary">
                                <div class="flex items-center justify-between text-sm">
                                    <span class="text-theme-muted">"Room ID"</span>
                                    <div class="flex items-center gap-2">
                                        <code class="text-xs bg-theme-tertiary text-theme-secondary px-2 py-1 rounded font-mono">
                                            {move || {
                                                room_id.get().map(|id| {
                                                    if id.len() > 8 {
                                                        format!("{}...", &id[..8])
                                                    } else {
                                                        id
                                                    }
                                                }).unwrap_or_default()
                                            }}
                                        </code>
                                        <button
                                            class="btn-icon text-theme-accent hover:opacity-80"
                                            on:click={copy_link}
                                            title="Copy room link"
                                        >
                                            <Icon name=icons::DOCUMENT_DUPLICATE class="icon-text" />
                                        </button>
                                    </div>
                                </div>
                            </div>

                            // Users list
                            <div class="p-4">
                                <h4 class="label-sm mb-2">
                                    {move || format!("Users ({})", ctx_users.get_all_users().len())}
                                </h4>
                                <div class="space-y-2 max-h-40 overflow-y-auto">
                                    {move || {
                                        let users = ctx_users.get_all_users();
                                        users.into_iter().map(|user| {
                                            let color = user.color.clone();
                                            let username = user.username.clone();
                                            let is_self = user.is_self;
                                            view! {
                                                <div class="flex items-center gap-2 text-sm">
                                                    <div
                                                        class="w-6 h-6 rounded-full flex items-center justify-center text-white text-xs font-medium"
                                                        style=format!("background-color: {}", color)
                                                    >
                                                        {username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                                    </div>
                                                    <span class="text-theme-secondary">{username}</span>
                                                    {if is_self {
                                                        view! { <span class="text-xs text-theme-muted">"(you)"</span> }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()
                                    }}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Disconnected view - join/create form
                    view! {
                        <div class="mt-2 w-80 card shadow-theme-xl overflow-hidden">
                            // Tab selector
                            <div class="flex divider-bottom">
                                <button
                                    class={move || if mode.get() == "join" {
                                        "flex-1 px-4 py-3 text-sm font-medium text-theme-accent border-b-2 border-theme-accent bg-theme-accent-light theme-transition"
                                    } else {
                                        "flex-1 px-4 py-3 text-sm font-medium text-theme-muted hover:text-theme-secondary theme-transition"
                                    }}
                                    on:click=move |_| set_mode.set("join")
                                >
                                    "Join Room"
                                </button>
                                <button
                                    class={move || if mode.get() == "create" {
                                        "flex-1 px-4 py-3 text-sm font-medium text-theme-accent border-b-2 border-theme-accent bg-theme-accent-light theme-transition"
                                    } else {
                                        "flex-1 px-4 py-3 text-sm font-medium text-theme-muted hover:text-theme-secondary theme-transition"
                                    }}
                                    on:click=move |_| set_mode.set("create")
                                >
                                    "Create Room"
                                </button>
                            </div>

                            // Form content
                            <div class="p-4 space-y-4">
                                // Error message
                                {move || error.get().map(|err| view! {
                                    <div class="error-message">
                                        <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                                        <span>{err}</span>
                                    </div>
                                })}

                                // Room ID input
                                <div>
                                    <label class="label">
                                        "Room ID"
                                    </label>
                                    <div class="flex gap-2">
                                        <input
                                            type="text"
                                            class="flex-1 input-base input-sm"
                                            placeholder="Enter room ID or UUID"
                                            prop:value=move || room_id_input.get()
                                            on:input=move |ev| set_room_id_input.set(event_target_value(&ev))
                                        />
                                        {move || if mode.get() == "create" {
                                            view! {
                                                <button
                                                    class="btn-icon btn-sm"
                                                    on:click=generate_room_id
                                                    title="Generate random ID"
                                                >
                                                    <Icon name=icons::DICES class="icon-standalone"/>
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                </div>

                                // Room name (create mode only)
                                {move || if mode.get() == "create" {
                                    view! {
                                        <div>
                                            <label class="label">
                                                "Room Name " <span class="text-theme-muted">"(optional)"</span>
                                            </label>
                                            <input
                                                type="text"
                                                class="input-base input-sm"
                                                placeholder="My awesome project"
                                                prop:value=move || room_name.get()
                                                on:input=move |ev| set_room_name.set(event_target_value(&ev))
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}

                                // Password input
                                <div>
                                    <label class="label">
                                        "Password " <span class="text-theme-muted">"(optional)"</span>
                                    </label>
                                    <input
                                        type="password"
                                        class="input-base input-sm"
                                        placeholder={move || if mode.get() == "create" { "Set a password" } else { "Enter room password" }}
                                        prop:value=move || password.get()
                                        on:input=move |ev| set_password.set(event_target_value(&ev))
                                    />
                                </div>

                                // Submit button
                                {
                                    let create_room = create_room;
                                    let join_room = join_room;
                                    view! {
                                        <button
                                            class="w-full btn-primary btn-sm disabled:opacity-50 disabled:cursor-not-allowed"
                                            disabled=move || connection_state.get() == ConnectionState::Connecting
                                            on:click=move |ev| {
                                                if mode.get() == "create" {
                                                    create_room(ev)
                                                } else {
                                                    join_room(ev)
                                                }
                                            }
                                        >
                                            {move || {
                                                if connection_state.get() == ConnectionState::Connecting {
                                                    "Connecting...".to_string()
                                                } else if mode.get() == "create" {
                                                    "Create & Join Room".to_string()
                                                } else {
                                                    "Join Room".to_string()
                                                }
                                            }}
                                        </button>
                                    }
                                }
                            </div>

                            // Help text
                            <div class="px-4 pb-4">
                                <p class="text-xs text-theme-muted text-center">
                                    {move || if mode.get() == "create" {
                                        "Share the Room ID with others to collaborate in real-time"
                                    } else {
                                        "Enter the Room ID shared by the room creator"
                                    }}
                                </p>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
