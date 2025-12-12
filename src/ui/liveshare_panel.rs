//! LiveShare Panel component for room management
//!
//! Provides UI for:
//! - Creating new rooms
//! - Joining existing rooms
//! - Viewing connected users
//! - Room settings
use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
use crate::ui::{Icon, icons};
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
        <div class="absolute top-4 right-4 z-50">
            // Toggle button
            <button
                class="flex items-center gap-2 px-4 py-2 bg-theme-surface border border-theme-primary rounded-lg shadow-theme-lg hover:bg-theme-secondary theme-transition"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                {move || {
                    let state = connection_state.get();
                    match state {
                        ConnectionState::Connected => view! {
                            <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                            <span class="text-sm font-medium text-theme-secondary">"Connected"</span>
                        }.into_any(),
                        ConnectionState::Connecting => view! {
                            <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                            <span class="text-sm font-medium text-theme-secondary">"Connecting..."</span>
                        }.into_any(),
                        ConnectionState::Reconnecting => view! {
                            <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                            <span class="text-sm font-medium text-theme-secondary">"Reconnecting..."</span>
                        }.into_any(),
                        ConnectionState::Error => view! {
                            <div class="w-2 h-2 bg-red-500 rounded-full"></div>
                            <span class="text-sm font-medium text-theme-secondary">"Error"</span>
                        }.into_any(),
                        ConnectionState::Disconnected => view! {
                            <div class="w-2 h-2 bg-gray-400 rounded-full"></div>
                            <span class="text-sm font-medium text-theme-secondary">"LiveShare"</span>
                        }.into_any(),
                    }
                }}
                <svg
                    class={move || if is_open.get() { "w-4 h-4 rotate-180 transition-transform duration-200 text-theme-tertiary" } else { "w-4 h-4 transition-transform duration-200 text-theme-tertiary" }}
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
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
                        <div class="mt-2 w-80 bg-theme-surface border border-theme-primary rounded-xl shadow-theme-xl overflow-hidden theme-transition">
                            // Header
                            <div class="px-4 py-3 bg-gradient-to-r from-green-500 to-emerald-600 text-white">
                                <div class="flex items-center justify-between">
                                    <h3 class="font-semibold">"Connected"</h3>
                                    <button
                                        class="text-white/80 hover:text-white text-sm"
                                        on:click={disconnect}
                                    >
                                        "Disconnect"
                                    </button>
                                </div>
                                {move || room_info.get().map(|info| view! {
                                    <p class="text-sm text-white/80 mt-1">{info.name}</p>
                                })}
                            </div>

                            // Room info
                            <div class="p-4 border-b border-theme-primary">
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
                                            class="flex items-center justify-center text-theme-accent hover:opacity-80"
                                            on:click={copy_link}
                                            title="Copy room link"
                                        >
                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            </div>

                            // Users list
                            <div class="p-4">
                                <h4 class="text-sm font-medium text-theme-secondary mb-2">
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
                        <div class="mt-2 w-80 bg-theme-surface border border-theme-primary rounded-xl shadow-theme-xl overflow-hidden theme-transition">
                            // Tab selector
                            <div class="flex border-b border-theme-primary">
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
                                    <div class="p-3 bg-theme-error border border-theme-error rounded-lg text-sm text-theme-error theme-transition">
                                        {err}
                                    </div>
                                })}

                                // Room ID input
                                <div>
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "Room ID"
                                    </label>
                                    <div class="flex gap-2">
                                        <input
                                            type="text"
                                            class="flex-1 px-3 py-2 input-theme rounded-lg text-sm"
                                            placeholder="Enter room ID or UUID"
                                            prop:value=move || room_id_input.get()
                                            on:input=move |ev| set_room_id_input.set(event_target_value(&ev))
                                        />
                                        {move || if mode.get() == "create" {
                                            view! {
                                                <button
                                                    class="flex items-center justify-center px-3 py-2 bg-theme-tertiary hover:bg-theme-secondary text-theme-secondary rounded-lg text-sm theme-transition"
                                                    on:click=generate_room_id
                                                    title="Generate random ID"
                                                >
                                                    <Icon name=icons::DICES class="w-5 h-5"/>
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
                                            <label class="block text-sm font-medium text-theme-secondary mb-1">
                                                "Room Name " <span class="text-theme-muted">"(optional)"</span>
                                            </label>
                                            <input
                                                type="text"
                                                class="w-full px-3 py-2 input-theme rounded-lg text-sm"
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
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "Password " <span class="text-theme-muted">"(optional)"</span>
                                    </label>
                                    <input
                                        type="password"
                                        class="w-full px-3 py-2 input-theme rounded-lg text-sm"
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
                                            class="w-full px-4 py-2.5 btn-theme-primary rounded-lg font-medium text-sm transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
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
