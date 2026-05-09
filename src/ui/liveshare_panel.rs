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

fn connection_label(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Connected => "Connected",
        ConnectionState::Connecting => "Connecting...",
        ConnectionState::Reconnecting => "Reconnecting...",
        ConnectionState::Error => "Connection lost",
        ConnectionState::Disconnected => "Not connected",
    }
}

fn connection_chip_style(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Connected => {
            "color: var(--primary); border-color: var(--success-border); background: var(--success-soft);"
        }
        ConnectionState::Connecting | ConnectionState::Reconnecting => {
            "color: var(--warning); border-color: var(--warning-border); background: var(--warning-soft);"
        }
        ConnectionState::Error => {
            "color: var(--destructive); border-color: var(--error-border); background: var(--destructive-soft);"
        }
        ConnectionState::Disconnected => {
            "color: var(--muted-foreground); border-color: color-mix(in oklab, var(--muted-foreground) 28%, transparent); background: color-mix(in oklab, var(--muted-foreground) 10%, transparent);"
        }
    }
}

fn connection_dot_style(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Connected => "background: var(--primary);",
        ConnectionState::Connecting | ConnectionState::Reconnecting => {
            "background: var(--warning);"
        }
        ConnectionState::Error => "background: var(--destructive);",
        ConnectionState::Disconnected => "background: var(--muted-foreground);",
    }
}

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
                    .set(
                        "X-User-ID",
                        &ctx_inner.user_id.with_untracked(|v| v.to_string()),
                    )
                    .unwrap();
                request
                    .headers()
                    .set(
                        "X-Username",
                        &ctx_inner.username.with_untracked(|v| v.clone()),
                    )
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
        <div class="absolute right-4 top-4 z-50 flex flex-col items-end gap-2">
            // Top status bar with connection and sync indicators
            <ConnectionStatusBar />

            // Toggle button
            <button
                type="button"
                class="btn shadow-theme-lg"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <Icon name=icons::USERS class="icon-text text-theme-accent" />
                <span class="font-medium">"LiveShare"</span>
                <span
                    class={move || {
                        if matches!(connection_state.get(), ConnectionState::Connecting | ConnectionState::Reconnecting) {
                            "h-2 w-2 rounded-full animate-pulse"
                        } else {
                            "h-2 w-2 rounded-full"
                        }
                    }}
                    style=move || connection_dot_style(connection_state.get())
                ></span>
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
                        <div class="mt-2 flex w-[360px] flex-col overflow-hidden rounded-xl border border-theme-primary bg-theme-surface shadow-theme-xl">
                            // Header with sync status
                            <div class="flex items-center gap-2 border-b border-theme-primary px-4 py-3">
                                <Icon name=icons::USERS class="w-3.5 h-3.5 text-theme-accent" />
                                <span class="text-sm font-semibold text-theme-primary">"LiveShare"</span>
                                <span class="chip h-[22px]" style=move || connection_chip_style(connection_state.get())>
                                    <span class="h-1.5 w-1.5 rounded-full" style=move || connection_dot_style(connection_state.get())></span>
                                    {move || connection_label(connection_state.get())}
                                </span>
                                <div class="flex-1"></div>
                                <button
                                    type="button"
                                    class="btn-icon btn-sm"
                                    on:click=move |_| set_is_open.set(false)
                                    title="Close LiveShare panel"
                                >
                                    <Icon name=icons::X class="icon-text" />
                                </button>
                            </div>

                            // Room info
                            <div class="border-b border-theme-primary p-4">
                                <div class="eyebrow mb-2">"Room"</div>
                                <div class="space-y-3 rounded-lg border border-theme-primary bg-theme-primary p-3">
                                    <div class="flex items-center gap-2">
                                        <code class="min-w-0 flex-1 truncate font-mono text-xs text-theme-primary">
                                            {move || room_info.get().map(|info| info.name).unwrap_or_else(|| "Active room".to_string())}
                                        </code>
                                        <span class="badge-default h-[18px] text-[10.5px]">"edit access"</span>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <input
                                            type="text"
                                            readonly
                                            class="input-base input-sm flex-1 font-mono"
                                            prop:value=move || room_id.get().unwrap_or_default()
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-sm"
                                            on:click={copy_link}
                                            title="Copy room link"
                                        >
                                            <Icon name=icons::DOCUMENT_DUPLICATE class="icon-text" />
                                            "Copy"
                                        </button>
                                    </div>
                                    <button
                                        type="button"
                                        class="btn-danger btn-sm"
                                        on:click={disconnect}
                                    >
                                        <Icon name=icons::X class="icon-text" />
                                        "Disconnect"
                                    </button>
                                </div>
                            </div>

                            // Status bar with sync and snapshot info
                            <div class="flex flex-wrap items-center gap-2 border-b border-theme-primary px-4 py-3 text-xs">
                                <div class="flex flex-wrap items-center gap-2">
                                    <SyncStatusBadge />
                                    <SnapshotSaveIndicator />
                                    <UserPresenceIndicator />
                                </div>
                            </div>

                            // Users list
                            <div class="max-h-64 flex-1 overflow-y-auto p-4 scroll">
                                <div class="eyebrow mb-2">
                                    {move || format!("Active · {}", ctx_users.get_all_users().len())}
                                </div>
                                <div class="space-y-2">
                                    {move || {
                                        let users = ctx_users.get_all_users();
                                        users.into_iter().map(|user| {
                                            let color = user.color.clone();
                                            let username = user.username.clone();
                                            let is_self = user.is_self;
                                            view! {
                                                <div class="flex items-center gap-3 rounded-lg border border-theme-primary bg-theme-primary px-3 py-2 text-sm">
                                                    <div
                                                        class="flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold text-white"
                                                        style=format!("background-color: {}", color)
                                                    >
                                                        {username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                                    </div>
                                                    <div class="min-w-0 flex-1">
                                                        <div class="truncate text-[12.5px] font-medium text-theme-primary">{username}</div>
                                                        <div class="truncate text-[11px] text-theme-muted">
                                                            {if is_self { "editing locally" } else { "collaborating" }}
                                                        </div>
                                                    </div>
                                                    {if is_self {
                                                        view! { <span class="badge-default h-[18px] text-[10.5px]">"you"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="h-1.5 w-1.5 rounded-full bg-theme-accent"></span> }.into_any()
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()
                                    }}
                                </div>
                            </div>

                            <div class="flex items-center justify-between border-t border-theme-primary px-4 py-3">
                                <span class="flex items-center gap-1.5 text-[11px] text-theme-muted">
                                    <span class="h-1.5 w-1.5 rounded-full bg-theme-accent"></span>
                                    "CRDT · snapshots enabled"
                                </span>
                                <button type="button" class="btn-icon btn-sm" title="LiveShare settings">
                                    <Icon name=icons::SETTINGS class="icon-text" />
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Disconnected view - join/create form
                    view! {
                        <div class="mt-2 w-[360px] overflow-hidden rounded-xl border border-theme-primary bg-theme-surface shadow-theme-xl">
                            <div class="flex items-center gap-2 border-b border-theme-primary px-4 py-3">
                                <Icon name=icons::USERS class="w-3.5 h-3.5 text-theme-accent" />
                                <span class="text-sm font-semibold text-theme-primary">"LiveShare"</span>
                                <span class="chip h-[22px]" style=move || connection_chip_style(connection_state.get())>
                                    <span class="h-1.5 w-1.5 rounded-full" style=move || connection_dot_style(connection_state.get())></span>
                                    {move || connection_label(connection_state.get())}
                                </span>
                            </div>

                            // Tab selector
                            <div class="m-4 mb-0 flex gap-1 rounded-lg border border-theme-primary bg-theme-tertiary p-1">
                                <button
                                    type="button"
                                    class={move || if mode.get() == "join" {
                                        "flex-1 btn btn-sm bg-theme-surface text-theme-primary border-theme-accent"
                                    } else {
                                        "flex-1 btn btn-sm btn-ghost text-theme-tertiary"
                                    }}
                                    on:click=move |_| set_mode.set("join")
                                >
                                    "Join Room"
                                </button>
                                <button
                                    type="button"
                                    class={move || if mode.get() == "create" {
                                        "flex-1 btn btn-sm bg-theme-surface text-theme-primary border-theme-accent"
                                    } else {
                                        "flex-1 btn btn-sm btn-ghost text-theme-tertiary"
                                    }}
                                    on:click=move |_| set_mode.set("create")
                                >
                                    "Create Room"
                                </button>
                            </div>

                            // Form content
                            <div class="space-y-4 p-4">
                                // Error message
                                {move || error.get().map(|err| view! {
                                    <div class="error-message rounded-lg border border-theme-error bg-theme-error p-3">
                                        <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                                        <span>{err}</span>
                                    </div>
                                })}

                                // Room ID input
                                <div class="space-y-1.5">
                                    <label class="label">
                                        "Room ID"
                                    </label>
                                    <div class="flex gap-2">
                                        <input
                                            type="text"
                                            class="input-base input-lg flex-1 font-mono"
                                            autocomplete="off"
                                            spellcheck="false"
                                            placeholder="Enter room ID or UUID"
                                            prop:value=move || room_id_input.get()
                                            on:input=move |ev| set_room_id_input.set(event_target_value(&ev))
                                        />
                                        {move || if mode.get() == "create" {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="btn-icon"
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
                                        <div class="space-y-1.5">
                                            <label class="label">
                                                "Room Name " <span class="text-theme-muted">"(optional)"</span>
                                            </label>
                                            <input
                                                type="text"
                                                class="input-base input-lg"
                                                autocomplete="off"
                                                spellcheck="false"
                                                placeholder="Architecture review"
                                                prop:value=move || room_name.get()
                                                on:input=move |ev| set_room_name.set(event_target_value(&ev))
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}

                                // Password input
                                <div class="space-y-1.5">
                                    <label class="label">
                                        "Password " <span class="text-theme-muted">"(optional)"</span>
                                    </label>
                                    <input
                                        type="password"
                                        class="input-base input-lg"
                                        autocomplete="current-password"
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
                                            type="button"
                                            class="btn-primary btn-lg w-full disabled:cursor-not-allowed disabled:opacity-50"
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
                            <div class="border-t border-theme-primary px-4 py-3">
                                <p class="text-center text-xs text-theme-muted">
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
