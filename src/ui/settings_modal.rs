//! Settings Modal component
//!
//! Provides a centered modal with:
//! - Backdrop blur effect
//! - LiveShare settings (room management)
//! - Canvas settings (theme switcher)
//! - Export settings

use crate::core::{ExportFormat, ExportOptions, SchemaExporter, SchemaGraph, SqlDialect};
use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
use crate::ui::theme::{ThemeMode, use_theme_context};
use crate::ui::{Icon, icons};
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use leptos::wasm_bindgen::{self, JsCast};
#[cfg(not(feature = "ssr"))]
use leptos::web_sys;

/// Settings modal component
#[component]
pub fn SettingsModal(
    /// Signal to control modal visibility
    is_open: RwSignal<bool>,
    /// Optional signal for initial room ID to pre-fill (e.g., from URL invite link)
    #[prop(optional)]
    initial_room_id: Option<RwSignal<String>>,
    /// Schema graph for export functionality
    #[prop(optional)]
    graph: Option<RwSignal<SchemaGraph>>,
) -> impl IntoView {
    // Get the LiveShare context
    let ctx = use_liveshare_context();

    // Get theme context
    let theme_ctx = use_theme_context();

    // Room ID input
    let (room_id_input, set_room_id_input) = signal(String::new());

    // Watch for initial_room_id changes and update room_id_input
    if let Some(initial_id_signal) = initial_room_id {
        Effect::new(move |_| {
            let id = initial_id_signal.get();
            if !id.is_empty() {
                set_room_id_input.set(id);
            }
        });
    }

    // Room name input (for creating)
    let (room_name, set_room_name) = signal(String::new());

    // Password input
    let (password, set_password) = signal(String::new());

    // Mode: "join" or "create"
    let (mode, set_mode) = signal("join");

    // Active settings tab
    let (active_tab, set_active_tab) = signal("liveshare");

    // Export state
    let (export_format, set_export_format) = signal("sql");
    let (_export_result, set_export_result) = signal::<Option<String>>(None);
    let (export_filename, set_export_filename) = signal(String::from("schema"));

    // Close modal handler
    let close_modal = move |_| {
        is_open.set(false);
    };

    // Close on Escape key
    #[cfg(not(feature = "ssr"))]
    {
        use leptos::ev::keydown;

        let handle_keydown = window_event_listener(keydown, move |ev| {
            if ev.key() == "Escape" && is_open.get_untracked() {
                is_open.set(false);
            }
        });

        on_cleanup(move || drop(handle_keydown));
    }

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

        #[cfg(not(feature = "ssr"))]
        {
            use leptos::task::spawn_local;

            let ctx_inner = ctx_create.clone();
            let room_id = room_id_val.clone();
            let room_name_val = room_name.get();
            let password_val = password.get();

            spawn_local(async move {
                let window = web_sys::window().expect("no window");
                let location = window.location();
                let origin = location.origin().unwrap_or_default();

                let create_url = format!("{}/room/{}", origin, room_id);

                let body = serde_json::json!({
                    "name": if room_name_val.is_empty() { None } else { Some(room_name_val) },
                    "password": if password_val.is_empty() { None } else { Some(&password_val) },
                    "max_users": 50
                });

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
            // Modal backdrop and container
            <Show when=move || is_open.get()>
                <div
                    class="fixed inset-0 z-50 flex items-center justify-center"
                    style="padding: 24px;"
                >
                    // Backdrop with blur
                    <div
                        class="absolute inset-0 modal-backdrop-theme"
                        on:click=close_modal
                    ></div>

                    // Modal content - 9:16 aspect ratio (phone format)
                    <div
                        class="relative flex flex-col bg-theme-surface border border-theme-primary theme-transition"
                        style="width: 380px; max-width: 100%; height: min(85vh, 680px); border-radius: 24px; box-shadow: var(--shadow-xl); overflow: hidden;"
                    >
                        // Header
                        <div
                            class="flex items-center justify-between shrink-0 bg-theme-surface border-b border-theme-primary theme-transition"
                            style="padding: 16px 20px;"
                        >
                            <h2 class="text-theme-primary" style="font-size: 18px; font-weight: 600;">"Settings"</h2>
                            <button
                                class="text-theme-tertiary hover:text-theme-secondary hover:bg-theme-tertiary transition-colors"
                                style="padding: 8px; border-radius: 12px;"
                                on:click=close_modal
                            >
                                <Icon name=icons::X class="w-5 h-5"/>
                            </button>
                        </div>

                        // Tabs
                        <div
                            class="flex items-center shrink-0 bg-theme-secondary border-b border-theme-primary theme-transition"
                            style="padding: 12px 16px;"
                        >
                            <button
                                class="flex-1 flex items-center justify-center transition-all"
                                style=move || {
                                    if active_tab.get() == "liveshare" {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--accent-primary); background-color: var(--accent-light); border-radius: 8px; box-shadow: var(--shadow-sm); border: 1px solid var(--accent-primary);"
                                    } else {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--text-tertiary); background-color: transparent; border-radius: 8px; border: 1px solid transparent;"
                                    }
                                }
                                on:click=move |_| set_active_tab.set("liveshare")
                            >
                                <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                                </svg>
                                "LiveShare"
                            </button>
                            <button
                                class="flex-1 flex items-center justify-center transition-all"
                                style=move || {
                                    if active_tab.get() == "canvas" {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--accent-primary); background-color: var(--accent-light); border-radius: 8px; box-shadow: var(--shadow-sm); border: 1px solid var(--accent-primary);"
                                    } else {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--text-tertiary); background-color: transparent; border-radius: 8px; border: 1px solid transparent;"
                                    }
                                }
                                on:click=move |_| set_active_tab.set("canvas")
                            >
                                <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
                                </svg>
                                "Canvas"
                            </button>
                            <button
                                class="flex-1 flex items-center justify-center transition-all"
                                style=move || {
                                    if active_tab.get() == "export" {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--accent-primary); background-color: var(--accent-light); border-radius: 8px; box-shadow: var(--shadow-sm); border: 1px solid var(--accent-primary);"
                                    } else {
                                        "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--text-tertiary); background-color: transparent; border-radius: 8px; border: 1px solid transparent;"
                                    }
                                }
                                on:click=move |_| set_active_tab.set("export")
                            >
                                <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                                </svg>
                                "Export"
                            </button>
                        </div>

                        // Content area
                        <div class="flex-1 overflow-y-auto bg-theme-surface theme-transition" style="padding: 20px;">
                            // LiveShare tab content
                            <Show when=move || active_tab.get() == "liveshare">
                                <div style="display: flex; flex-direction: column; gap: 16px;">
                                    // Connection status
                                    <div class="flex items-center bg-theme-secondary theme-transition" style="gap: 12px; padding: 12px; border-radius: 12px;">
                                    {move || {
                                        let state = connection_state.get();
                                        match state {
                                            ConnectionState::Connected => view! {
                                                <div class="w-2.5 h-2.5 bg-green-500 rounded-full animate-pulse"></div>
                                                <span class="text-theme-secondary" style="font-size: 14px; font-weight: 500;">"Connected"</span>
                                            }.into_any(),
                                            ConnectionState::Connecting => view! {
                                                <div class="w-2.5 h-2.5 bg-yellow-500 rounded-full animate-pulse"></div>
                                                <span class="text-theme-secondary" style="font-size: 14px; font-weight: 500;">"Connecting..."</span>
                                            }.into_any(),
                                            ConnectionState::Reconnecting => view! {
                                                <div class="w-2.5 h-2.5 bg-yellow-500 rounded-full animate-pulse"></div>
                                                <span class="text-theme-secondary" style="font-size: 14px; font-weight: 500;">"Reconnecting..."</span>
                                            }.into_any(),
                                            ConnectionState::Error => view! {
                                                <div class="w-2.5 h-2.5 bg-red-500 rounded-full"></div>
                                                <span class="text-theme-secondary" style="font-size: 14px; font-weight: 500;">"Error"</span>
                                            }.into_any(),
                                            ConnectionState::Disconnected => view! {
                                                <div class="w-2.5 h-2.5 bg-gray-400 rounded-full"></div>
                                                <span class="text-theme-secondary" style="font-size: 14px; font-weight: 500;">"Disconnected"</span>
                                            }.into_any(),
                                        }
                                    }}
                                </div>

                                {move || {
                                    let state = connection_state.get();

                                    if state == ConnectionState::Connected {
                                        // Connected view
                                        let ctx_users = ctx;
                                        view! {
                                            <div style="display: flex; flex-direction: column; gap: 16px;">
                                                // Room info
                                                <div class="bg-theme-success border border-theme-success theme-transition" style="padding: 12px; border-radius: 12px;">
                                                    <div class="flex items-center justify-between" style="margin-bottom: 8px;">
                                                        <span class="text-theme-success" style="font-size: 14px; font-weight: 500;">"Room Info"</span>
                                                        <button
                                                            class="text-theme-error"
                                                            style="font-size: 12px; font-weight: 500;"
                                                            on:click={disconnect}
                                                        >
                                                            "Disconnect"
                                                        </button>
                                                    </div>
                                                    {move || room_info.get().map(|info| view! {
                                                        <p class="text-theme-success" style="font-size: 14px; margin-bottom: 8px;">{info.name}</p>
                                                    })}
                                                    <div class="flex items-center" style="gap: 8px;">
                                                        <input
                                                            type="text"
                                                            readonly
                                                            class="bg-theme-surface border border-theme-success text-theme-success theme-transition"
                                                            style="font-size: 11px; padding: 4px 8px; border-radius: 6px; font-family: monospace; flex: 1; outline: none; cursor: text;"
                                                            prop:value=move || {
                                                                #[cfg(not(feature = "ssr"))]
                                                                {
                                                                    room_id.get().map(|id| {
                                                                        if let Some(window) = web_sys::window() {
                                                                            let location = window.location();
                                                                            let protocol = location.protocol().unwrap_or_default();
                                                                            let host = location.host().unwrap_or_default();
                                                                            format!("{}//{}?room={}", protocol, host, id)
                                                                        } else {
                                                                            id
                                                                        }
                                                                    }).unwrap_or_default()
                                                                }
                                                                #[cfg(feature = "ssr")]
                                                                {
                                                                    room_id.get().unwrap_or_default()
                                                                }
                                                            }
                                                        />
                                                        <button
                                                            class="flex items-center justify-center text-theme-success"
                                                            style="padding: 4px;"
                                                            on:click={copy_link}
                                                            title="Copy room link"
                                                        >
                                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                                                            </svg>
                                                        </button>
                                                    </div>
                                                </div>

                                                // Users list
                                                <div class="bg-theme-secondary theme-transition" style="padding: 12px; border-radius: 12px;">
                                                    <h4 class="text-theme-secondary" style="font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                                                        {move || format!("Users ({})", ctx_users.get_all_users().len())}
                                                    </h4>
                                                    <div style="display: flex; flex-direction: column; gap: 6px; max-height: 160px; overflow-y: auto;">
                                                        {move || {
                                                            let users = ctx_users.get_all_users();
                                                            users.into_iter().map(|user| {
                                                                let color = user.color.clone();
                                                                let username = user.username.clone();
                                                                let is_self = user.is_self;
                                                                view! {
                                                                    <div class="flex items-center bg-theme-surface border border-theme-primary theme-transition" style="gap: 8px; padding: 8px; border-radius: 8px;">
                                                                        <div
                                                                            class="flex items-center justify-center"
                                                                            style=format!("width: 28px; height: 28px; border-radius: 50%; background-color: {}; color: white; font-size: 12px; font-weight: 500;", color)
                                                                        >
                                                                            {username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                                                        </div>
                                                                        <span class="text-theme-secondary" style="font-size: 14px; overflow: hidden; text-overflow: ellipsis; flex: 1;">{username}</span>
                                                                        {if is_self {
                                                                            view! { <span class="text-theme-muted" style="font-size: 12px;">"(you)"</span> }.into_any()
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
                                            <div style="display: flex; flex-direction: column; gap: 16px;">
                                                // Tab selector
                                                <div class="flex bg-theme-tertiary theme-transition" style="border-radius: 12px; padding: 4px;">
                                                    <button
                                                        class="flex-1 transition-all"
                                                        style=move || if mode.get() == "join" {
                                                            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--accent-primary); background-color: var(--bg-surface); border-radius: 8px; box-shadow: var(--shadow-sm);"
                                                        } else {
                                                            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--text-tertiary); background-color: transparent; border-radius: 8px;"
                                                        }
                                                        on:click=move |_| set_mode.set("join")
                                                    >
                                                        "Join Room"
                                                    </button>
                                                    <button
                                                        class="flex-1 transition-all"
                                                        style=move || if mode.get() == "create" {
                                                            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--accent-primary); background-color: var(--bg-surface); border-radius: 8px; box-shadow: var(--shadow-sm);"
                                                        } else {
                                                            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--text-tertiary); background-color: transparent; border-radius: 8px;"
                                                        }
                                                        on:click=move |_| set_mode.set("create")
                                                    >
                                                        "Create Room"
                                                    </button>
                                                </div>

                                                // Error message
                                                {move || error.get().map(|err| view! {
                                                    <div class="bg-theme-error border border-theme-error text-theme-error theme-transition" style="padding: 12px; border-radius: 12px; font-size: 14px;">
                                                        {err}
                                                    </div>
                                                })}

                                                // Room ID input
                                                <div>
                                                    <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                                                        "Room ID"
                                                    </label>
                                                    <div class="flex" style="gap: 8px;">
                                                        <input
                                                            type="text"
                                                            class="input-theme"
                                                            style="flex: 1; padding: 10px 12px; border-radius: 12px; font-size: 14px;"
                                                            placeholder="Enter room ID or UUID"
                                                            prop:value=move || room_id_input.get()
                                                            on:input=move |ev| set_room_id_input.set(event_target_value(&ev))
                                                        />
                                                        {move || if mode.get() == "create" {
                                                            view! {
                                                                <button
                                                                    class="bg-theme-tertiary text-theme-secondary theme-transition"
                                                                    style="padding: 10px 12px; border-radius: 12px;"
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
                                                            <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                                                                "Room Name " <span class="text-theme-muted">"(optional)"</span>
                                                            </label>
                                                            <input
                                                                type="text"
                                                                class="input-theme"
                                                                style="width: 100%; padding: 10px 12px; border-radius: 12px; font-size: 14px; box-sizing: border-box;"
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
                                                    <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                                                        "Password " <span class="text-theme-muted">"(optional)"</span>
                                                    </label>
                                                    <input
                                                        type="password"
                                                        class="input-theme"
                                                        style="width: 100%; padding: 10px 12px; border-radius: 12px; font-size: 14px; box-sizing: border-box;"
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
                                                            class="btn-theme-primary"
                                                            style="width: 100%; padding: 12px 16px; border-radius: 12px; font-size: 14px; font-weight: 500;"
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

                                                // Help text
                                                <p class="text-theme-tertiary" style="font-size: 12px; text-align: center;">
                                                    {move || if mode.get() == "create" {
                                                        "Share the Room ID to collaborate"
                                                    } else {
                                                        "Enter the Room ID shared by the creator"
                                                    }}
                                                </p>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </Show>

                        // Canvas tab content with theme selector
                        <Show when=move || active_tab.get() == "canvas">
                            <div style="display: flex; flex-direction: column; gap: 20px;">
                                // Theme section
                                <div>
                                    <h3 class="text-theme-primary" style="font-size: 16px; font-weight: 600; margin-bottom: 16px;">"Appearance"</h3>

                                    // Theme selector
                                    <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px;">
                                        <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 12px;">
                                            "Theme"
                                        </label>
                                        <div style="display: flex; gap: 8px;">
                                            // Automatic
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if theme_ctx.mode.get() == ThemeMode::Auto {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| theme_ctx.set_mode(ThemeMode::Auto)
                                            >
                                                <svg class="w-6 h-6" style=move || if theme_ctx.mode.get() == ThemeMode::Auto { "color: var(--accent-primary);" } else { "color: var(--text-tertiary);" } fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                                                </svg>
                                                <span style=move || if theme_ctx.mode.get() == ThemeMode::Auto { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "Automatic"
                                                </span>
                                            </button>

                                            // Dark
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if theme_ctx.mode.get() == ThemeMode::Dark {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| theme_ctx.set_mode(ThemeMode::Dark)
                                            >
                                                <svg class="w-6 h-6" style=move || if theme_ctx.mode.get() == ThemeMode::Dark { "color: var(--accent-primary);" } else { "color: var(--text-tertiary);" } fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                                                </svg>
                                                <span style=move || if theme_ctx.mode.get() == ThemeMode::Dark { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "Dark"
                                                </span>
                                            </button>

                                            // Light
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if theme_ctx.mode.get() == ThemeMode::Light {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| theme_ctx.set_mode(ThemeMode::Light)
                                            >
                                                <svg class="w-6 h-6" style=move || if theme_ctx.mode.get() == ThemeMode::Light { "color: var(--accent-primary);" } else { "color: var(--text-tertiary);" } fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                                                </svg>
                                                <span style=move || if theme_ctx.mode.get() == ThemeMode::Light { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "Light"
                                                </span>
                                            </button>
                                        </div>

                                        // Current theme indicator
                                        <p class="text-theme-muted" style="font-size: 12px; margin-top: 12px; text-align: center;">
                                            {move || {
                                                match theme_ctx.mode.get() {
                                                    ThemeMode::Auto => {
                                                        if theme_ctx.is_dark.get() {
                                                            "Currently using dark theme (system preference)"
                                                        } else {
                                                            "Currently using light theme (system preference)"
                                                        }
                                                    }
                                                    ThemeMode::Dark => "Dark theme enabled",
                                                    ThemeMode::Light => "Light theme enabled",
                                                }
                                            }}
                                        </p>
                                    </div>
                                </div>

                                // Additional canvas settings placeholder
                                <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px;">
                                    <div class="flex items-center" style="gap: 12px;">
                                        <svg class="w-5 h-5 text-theme-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                        </svg>
                                        <p class="text-theme-tertiary" style="font-size: 14px;">
                                            "More canvas settings coming soon..."
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </Show>

                        // Export tab content
                        <Show when=move || active_tab.get() == "export">
                            <div style="display: flex; flex-direction: column; gap: 20px;">
                                // Export format selector
                                <div>
                                    <h3 class="text-theme-primary" style="font-size: 16px; font-weight: 600; margin-bottom: 16px;">"Export Schema"</h3>

                                    // Format selector
                                    <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px; margin-bottom: 16px;">
                                        <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 12px;">
                                            "Format"
                                        </label>
                                        <div style="display: flex; gap: 8px;">
                                            // SQL
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if export_format.get() == "sql" {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| set_export_format.set("sql")
                                            >
                                                <Icon name=icons::DATABASE class="w-5 h-5"/>
                                                <span style=move || if export_format.get() == "sql" { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "SQL"
                                                </span>
                                            </button>

                                            // JSON
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if export_format.get() == "json" {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| set_export_format.set("json")
                                            >
                                                <Icon name=icons::JSON class="w-5 h-5"/>
                                                <span style=move || if export_format.get() == "json" { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "JSON"
                                                </span>
                                            </button>

                                            // CSV
                                            <button
                                                class="flex-1 flex flex-col items-center transition-all theme-transition"
                                                style=move || {
                                                    let base = "padding: 12px 8px; border-radius: 12px; gap: 8px;";
                                                    if export_format.get() == "csv" {
                                                        format!("{} background-color: var(--accent-light); border: 2px solid var(--accent-primary);", base)
                                                    } else {
                                                        format!("{} background-color: var(--bg-surface); border: 2px solid var(--border-primary);", base)
                                                    }
                                                }
                                                on:click=move |_| set_export_format.set("csv")
                                            >
                                                <Icon name=icons::FILE class="w-5 h-5"/>
                                                <span style=move || if export_format.get() == "csv" { "font-size: 12px; font-weight: 500; color: var(--accent-primary);" } else { "font-size: 12px; font-weight: 500; color: var(--text-tertiary);" }>
                                                    "CSV"
                                                </span>
                                            </button>
                                        </div>
                                    </div>

                                    // Filename input
                                    <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px; margin-bottom: 16px;">
                                        <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                                            "Filename"
                                        </label>
                                        <div style="display: flex; align-items: center; gap: 8px;">
                                            <input
                                                type="text"
                                                class="input-theme"
                                                style="flex: 1; padding: 10px 12px; border-radius: 8px; font-size: 14px;"
                                                placeholder="schema"
                                                prop:value=move || export_filename.get()
                                                on:input=move |ev| set_export_filename.set(event_target_value(&ev))
                                            />
                                            <span class="text-theme-muted" style="font-size: 14px;">
                                                {move || format!(".{}", export_format.get())}
                                            </span>
                                        </div>
                                    </div>

                                    // Export button
                                    {move || {
                                        if let Some(g) = graph {
                                            view! {
                                                <button
                                                    class="btn-theme-primary"
                                                    style="width: 100%; padding: 14px 16px; border-radius: 12px; font-size: 14px; font-weight: 600; display: flex; align-items: center; justify-content: center; gap: 8px;"
                                                    on:click=move |_| {
                                                        let format = export_format.get();
                                                        let filename_input = export_filename.get();
                                                        #[allow(unused_variables)]
                                                        let filename = if filename_input.is_empty() { "schema".to_string() } else { filename_input };

                                                        let result = g.with(|graph| {
                                                            let options = ExportOptions {
                                                                format: match format {
                                                                    "json" => ExportFormat::Json,
                                                                    "csv" => ExportFormat::Csv,
                                                                    _ => ExportFormat::Sql,
                                                                },
                                                                sql_dialect: SqlDialect::MySQL,
                                                                include_positions: true,
                                                                include_drop_statements: false,
                                                                pretty_print: true,
                                                            };
                                                            SchemaExporter::export(graph, &options)
                                                        });

                                                        match result {
                                                            Ok(content) => {
                                                                set_export_result.set(Some(content.clone()));

                                                                // Trigger download
                                                                #[cfg(not(feature = "ssr"))]
                                                                {
                                                                    let mime_type = match format {
                                                                        "json" => "application/json",
                                                                        "csv" => "text/csv",
                                                                        _ => "text/plain",
                                                                    };
                                                                    let full_filename = format!("{}.{}", filename, format);

                                                                    // Create blob and download
                                                                    let blob_parts = js_sys::Array::new();
                                                                    blob_parts.push(&wasm_bindgen::JsValue::from_str(&content));

                                                                    let mut options = web_sys::BlobPropertyBag::new();
                                                                    options.set_type(mime_type);

                                                                    if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
                                                                        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                                                            if let Some(window) = web_sys::window() {
                                                                                if let Some(document) = window.document() {
                                                                                    if let Ok(a) = document.create_element("a") {
                                                                                        let _ = a.set_attribute("href", &url);
                                                                                        let _ = a.set_attribute("download", &full_filename);
                                                                                        a.dyn_ref::<web_sys::HtmlElement>().map(|el| el.click());
                                                                                        let _ = web_sys::Url::revoke_object_url(&url);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                set_export_result.set(Some(format!("Error: {}", e)));
                                                            }
                                                        }
                                                    }
                                                >
                                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                                                    </svg>
                                                    "Download"
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px; text-align: center;">
                                                    <p class="text-theme-tertiary" style="font-size: 14px;">
                                                        "No schema loaded"
                                                    </p>
                                                </div>
                                            }.into_any()
                                        }
                                    }}
                                </div>

                                // Format description
                                <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px;">
                                    <div class="flex items-start" style="gap: 12px;">
                                        <svg class="w-5 h-5 text-theme-muted flex-shrink-0" style="margin-top: 2px;" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                        </svg>
                                        <div>
                                            <p class="text-theme-secondary" style="font-size: 14px; font-weight: 500; margin-bottom: 4px;">
                                                {move || match export_format.get() {
                                                    "json" => "JSON Format",
                                                    "csv" => "CSV Format",
                                                    _ => "SQL Format",
                                                }}
                                            </p>
                                            <p class="text-theme-tertiary" style="font-size: 13px; line-height: 1.5;">
                                                {move || match export_format.get() {
                                                    "json" => "Structured format with tables, columns, relationships and positions. Ideal for backup and programmatic access.",
                                                    "csv" => "Tabular format with separate sections for tables, columns and relationships. Good for spreadsheet analysis.",
                                                    _ => "DDL statements (CREATE TABLE) compatible with MySQL. Ready for database deployment.",
                                                }}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>
        </Show>
    }
}

/// Settings button component
#[component]
pub fn SettingsButton(
    /// Signal to control modal visibility
    is_open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            class="fixed bottom-4 right-4 z-40 flex items-center justify-center w-12 h-12 bg-theme-surface border border-theme-primary text-theme-secondary hover:text-theme-accent hover:border-theme-accent theme-transition transition-colors"
            style="border-radius: 12px; box-shadow: var(--shadow-lg);"
            on:click=move |_| is_open.set(true)
            title="Settings"
        >
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
        </button>
    }
}
