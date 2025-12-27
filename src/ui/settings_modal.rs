//! Settings Modal component
//!
//! Provides a centered modal with:
//! - Backdrop blur effect
//! - Diagram settings (info, rename, LiveShare, delete)
//! - Canvas settings (theme switcher)
//! - Export settings

use crate::core::{ExportFormat, ExportOptions, SchemaExporter, SchemaGraph, SqlDialect};
use crate::ui::liveshare_client::{ConnectionState, LiveShareContext, use_liveshare_context};
use crate::ui::theme::{ThemeMode, use_theme_context};
use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

#[cfg(not(feature = "ssr"))]
use crate::ui::auth_utils;

#[cfg(not(feature = "ssr"))]
use leptos::wasm_bindgen::{self, JsCast};
#[cfg(not(feature = "ssr"))]
use leptos::web_sys;
/// Mode tab button for join/create selection
#[component]
fn ModeTabButton(
    mode_value: &'static str,
    current_mode: Memo<&'static str>,
    label: &'static str,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_mode.get() == mode_value;

    let button_style = move || {
        if is_selected() {
            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--accent-primary); background-color: var(--bg-surface); border-radius: 8px; box-shadow: var(--shadow-sm);"
        } else {
            "padding: 8px 12px; font-size: 14px; font-weight: 500; color: var(--text-tertiary); background-color: transparent; border-radius: 8px;"
        }
    };

    view! {
        <button
            class="flex-1 transition-all"
            style=button_style
            on:click=move |_| on_click.run(())
        >
            {label}
        </button>
    }
    .into_any()
}

/// LiveShare tab content - Disconnected view (join/create form)
#[component]
fn LiveShareDisconnectedView(
    ctx: LiveShareContext,
    room_id_input: RwSignal<String>,
    room_name: RwSignal<String>,
    password: RwSignal<String>,
    mode: RwSignal<&'static str>,
    connection_state: RwSignal<ConnectionState>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    // Create room handler
    let ctx_create = ctx;
    let create_room = move |_| {
        ctx_create.error.set(None);

        #[cfg(not(feature = "ssr"))]
        {
            use leptos::task::spawn_local;

            let ctx_inner = ctx_create.clone();
            let room_name_val = room_name.get();
            let password_val = password.get();

            spawn_local(async move {
                let window = web_sys::window().expect("no window");
                let location = window.location();
                let origin = location.origin().unwrap_or_default();

                // Use new endpoint without UUID in URL
                let create_url = format!("{}/room", origin);

                let body = serde_json::json!({
                    "name": if room_name_val.is_empty() { None } else { Some(room_name_val) },
                    "password": if password_val.is_empty() { None } else { Some(&password_val) },
                    "max_users": 50
                });

                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                opts.set_credentials(web_sys::RequestCredentials::Include);
                opts.set_body(&wasm_bindgen::JsValue::from_str(
                    &serde_json::to_string(&body).unwrap(),
                ));

                let request = web_sys::Request::new_with_str_and_init(&create_url, &opts).unwrap();
                request
                    .headers()
                    .set("Content-Type", "application/json")
                    .unwrap();

                // Add Authorization header with JWT token from localStorage
                let _ = auth_utils::add_auth_header(&request);

                let window = web_sys::window().unwrap();
                match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                    .await
                {
                    Ok(resp) => {
                        let resp: web_sys::Response = resp.into();
                        if resp.ok() {
                            // Parse response to get the generated room ID
                            match wasm_bindgen_futures::JsFuture::from(resp.json().unwrap()).await {
                                Ok(json_value) => {
                                    let json_obj = js_sys::Object::from(json_value);
                                    if let Some(room_id_js) = js_sys::Reflect::get(
                                        &json_obj,
                                        &wasm_bindgen::JsValue::from_str("id"),
                                    )
                                    .ok()
                                    {
                                        if let Some(room_id) = room_id_js.as_string() {
                                            let pwd = if password_val.is_empty() {
                                                None
                                            } else {
                                                Some(password_val)
                                            };
                                            ctx_inner.connect(room_id, pwd);
                                        } else {
                                            ctx_inner.error.set(Some(
                                                "Invalid room ID in response".to_string(),
                                            ));
                                            ctx_inner.connection_state.set(ConnectionState::Error);
                                        }
                                    } else {
                                        ctx_inner
                                            .error
                                            .set(Some("Room ID not found in response".to_string()));
                                        ctx_inner.connection_state.set(ConnectionState::Error);
                                    }
                                }
                                Err(e) => {
                                    ctx_inner
                                        .error
                                        .set(Some(format!("Failed to parse response: {:?}", e)));
                                    ctx_inner.connection_state.set(ConnectionState::Error);
                                }
                            }
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

    let current_mode = Memo::new(move |_| mode.get());
    let set_join = Callback::new(move |_: ()| mode.set("join"));
    let set_create = Callback::new(move |_: ()| mode.set("create"));

    view! {
        <div style="display: flex; flex-direction: column; gap: 16px;">
            // Tab selector
            <div class="flex bg-theme-tertiary theme-transition" style="border-radius: 12px; padding: 4px;">
                <ModeTabButton
                    mode_value="join"
                    current_mode=current_mode
                    label="Join Room"
                    on_click=set_join
                />
                <ModeTabButton
                    mode_value="create"
                    current_mode=current_mode
                    label="Create Room"
                    on_click=set_create
                />
            </div>

            // Error message
            {move || error.get().map(|err| view! {
                <div class="bg-theme-error border border-theme-error text-theme-error theme-transition" style="padding: 12px; border-radius: 12px; font-size: 14px;">
                    {err}
                </div>
            })}

            // Room ID input (only in join mode)
            {move || if mode.get() == "join" {
                view! {
                    <div>
                        <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 8px;">
                            "Room ID"
                        </label>
                        <input
                            type="text"
                            class="input-theme"
                            style="width: 100%; padding: 10px 12px; border-radius: 12px; font-size: 14px; box-sizing: border-box;"
                            placeholder="Enter room ID or UUID"
                            prop:value=move || room_id_input.get()
                            on:input=move |ev| room_id_input.set(event_target_value(&ev))
                        />
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

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
                            on:input=move |ev| room_name.set(event_target_value(&ev))
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
                    placeholder=move || if mode.get() == "create" { "Set a password" } else { "Enter room password" }
                    prop:value=move || password.get()
                    on:input=move |ev| password.set(event_target_value(&ev))
                />
            </div>

            // Submit button
            {
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
                    "Room ID will be generated automatically for sharing"
                } else {
                    "Enter the Room ID shared by the creator"
                }}
            </p>
        </div>
    }
}

/// Settings tab button component - isolated to reduce type nesting
#[component]
fn SettingsTabButton(
    tab_value: &'static str,
    current_tab: Memo<&'static str>,
    label: &'static str,
    icon_path: &'static str,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_tab.get() == tab_value;

    let button_style = move || {
        if is_selected() {
            "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--accent-primary); background-color: var(--accent-light); border-radius: 8px; box-shadow: var(--shadow-sm); border: 1px solid var(--accent-primary);"
        } else {
            "gap: 6px; padding: 8px 4px; font-size: 13px; font-weight: 600; color: var(--text-tertiary); background-color: transparent; border-radius: 8px; border: 1px solid transparent;"
        }
    };

    view! {
        <button
            class="flex-1 flex items-center justify-center transition-all"
            style=button_style
            on:click=move |_| on_click.run(())
        >
            <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=icon_path />
            </svg>
            {label}
        </button>
    }.into_any()
}

/// Theme button component - isolated to reduce type nesting
#[component]
fn ThemeButton(
    mode: ThemeMode,
    current_mode: Memo<ThemeMode>,
    label: &'static str,
    icon_path: &'static str,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_mode.get() == mode;

    let button_style = move || {
        if is_selected() {
            "padding: 12px 8px; border-radius: 12px; gap: 8px; background-color: var(--accent-light); border: 2px solid var(--accent-primary);"
        } else {
            "padding: 12px 8px; border-radius: 12px; gap: 8px; background-color: var(--bg-surface); border: 2px solid var(--border-primary);"
        }
    };

    let icon_style = move || {
        if is_selected() {
            "color: var(--accent-primary);"
        } else {
            "color: var(--text-tertiary);"
        }
    };

    let text_style = move || {
        if is_selected() {
            "font-size: 12px; font-weight: 500; color: var(--accent-primary);"
        } else {
            "font-size: 12px; font-weight: 500; color: var(--text-tertiary);"
        }
    };

    view! {
        <button
            class="flex-1 flex flex-col items-center transition-all theme-transition"
            style=button_style
            on:click=move |_| on_click.run(())
        >
            <svg class="w-6 h-6" style=icon_style fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=icon_path />
            </svg>
            <span style=text_style>
                {label}
            </span>
        </button>
    }.into_any()
}

/// Theme selector component
#[component]
fn ThemeSelector() -> impl IntoView {
    let theme_ctx = use_theme_context();
    let current_mode = Memo::new(move |_| theme_ctx.mode.get());

    let set_auto = Callback::new(move |_: ()| theme_ctx.set_mode(ThemeMode::Auto));
    let set_dark = Callback::new(move |_: ()| theme_ctx.set_mode(ThemeMode::Dark));
    let set_light = Callback::new(move |_: ()| theme_ctx.set_mode(ThemeMode::Light));

    view! {
        <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px;">
            <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 12px;">
                "Theme"
            </label>
            <div style="display: flex; gap: 8px;">
                <ThemeButton
                    mode=ThemeMode::Auto
                    current_mode=current_mode
                    label="Automatic"
                    icon_path="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                    on_click=set_auto
                />
                <ThemeButton
                    mode=ThemeMode::Dark
                    current_mode=current_mode
                    label="Dark"
                    icon_path="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
                    on_click=set_dark
                />
                <ThemeButton
                    mode=ThemeMode::Light
                    current_mode=current_mode
                    label="Light"
                    icon_path="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
                    on_click=set_light
                />
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
    }.into_any()
}

/// Canvas/Theme tab content component
#[component]
fn CanvasTab() -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; gap: 20px;">
            // Theme section
            <div>
                <h3 class="text-theme-primary" style="font-size: 16px; font-weight: 600; margin-bottom: 16px;">"Appearance"</h3>
                <ThemeSelector/>
            </div>

            // Additional canvas settings placeholder
            <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px;">
                <div class="flex items-center" style="gap: 12px;">
                    <Icon name=icons::INFORMATION_CIRCLE class="w-5 h-5 text-theme-muted" />
                    <p class="text-theme-tertiary" style="font-size: 14px;">
                        "More canvas settings coming soon..."
                    </p>
                </div>
            </div>
        </div>
    }
}

/// Export format button component - isolated to reduce type nesting
#[component]
fn ExportFormatButton(
    format_value: &'static str,
    current_format: Memo<&'static str>,
    label: &'static str,
    icon_name: &'static str,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_format.get() == format_value;

    let button_style = move || {
        if is_selected() {
            "padding: 12px 8px; border-radius: 12px; gap: 8px; background-color: var(--accent-light); border: 2px solid var(--accent-primary);"
        } else {
            "padding: 12px 8px; border-radius: 12px; gap: 8px; background-color: var(--bg-surface); border: 2px solid var(--border-primary);"
        }
    };

    let text_style = move || {
        if is_selected() {
            "font-size: 12px; font-weight: 500; color: var(--accent-primary);"
        } else {
            "font-size: 12px; font-weight: 500; color: var(--text-tertiary);"
        }
    };

    view! {
        <button
            class="flex-1 flex flex-col items-center transition-all theme-transition"
            style=button_style
            on:click=move |_| on_click.run(())
        >
            <Icon name=icon_name class="w-5 h-5"/>
            <span style=text_style>
                {label}
            </span>
        </button>
    }
    .into_any()
}

/// Export format selector component
#[component]
fn ExportFormatSelector(
    export_format: ReadSignal<&'static str>,
    set_export_format: WriteSignal<&'static str>,
) -> impl IntoView {
    let current_format = Memo::new(move |_| export_format.get());

    let set_sql = Callback::new(move |_: ()| set_export_format.set("sql"));
    let set_json = Callback::new(move |_: ()| set_export_format.set("json"));
    let set_csv = Callback::new(move |_: ()| set_export_format.set("csv"));

    view! {
        <div class="bg-theme-secondary theme-transition" style="padding: 16px; border-radius: 12px; margin-bottom: 16px;">
            <label class="text-theme-secondary" style="display: block; font-size: 14px; font-weight: 500; margin-bottom: 12px;">
                "Format"
            </label>
            <div style="display: flex; gap: 8px;">
                <ExportFormatButton
                    format_value="sql"
                    current_format=current_format
                    label="SQL"
                    icon_name=icons::DATABASE
                    on_click=set_sql
                />
                <ExportFormatButton
                    format_value="json"
                    current_format=current_format
                    label="JSON"
                    icon_name=icons::JSON
                    on_click=set_json
                />
                <ExportFormatButton
                    format_value="csv"
                    current_format=current_format
                    label="CSV"
                    icon_name=icons::FILE
                    on_click=set_csv
                />
            </div>
        </div>
    }.into_any()
}

/// Export tab content component
#[component]
fn ExportTab(graph: Option<RwSignal<SchemaGraph>>) -> impl IntoView {
    let (export_format, set_export_format) = signal("sql");
    let (_export_result, set_export_result) = signal::<Option<String>>(None);
    let (export_filename, set_export_filename) = signal(String::from("schema"));

    view! {
        <div style="display: flex; flex-direction: column; gap: 20px;">
            // Export format selector
            <div>
                <h3 class="text-theme-primary" style="font-size: 16px; font-weight: 600; margin-bottom: 16px;">"Export Schema"</h3>

                // Format selector
                <ExportFormatSelector export_format=export_format set_export_format=set_export_format/>

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

                                                let options = web_sys::BlobPropertyBag::new();
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
                                <Icon name=icons::ARROW_DOWN_TO_LINE class="w-5 h-5" />
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
                    <Icon name=icons::INFORMATION_CIRCLE class="w-5 h-5 text-theme-muted flex-shrink-0 mt-0.5" />
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
    }
}

/// Diagram tab content - info, rename, liveshare, and delete functionality
#[component]
fn DiagramTab(
    diagram_name: Option<String>,
    diagram_id: Option<String>,
    is_demo: bool,
    on_name_change: Option<Callback<String>>,
    ctx: LiveShareContext,
    room_id_input: RwSignal<String>,
    room_name: RwSignal<String>,
    password: RwSignal<String>,
    mode: RwSignal<&'static str>,
) -> impl IntoView {
    // Store original name in signal for use in closures
    let original_name = RwSignal::new(
        diagram_name
            .clone()
            .unwrap_or_else(|| "Untitled".to_string()),
    );

    // Clone diagram_id for use in closures
    let has_diagram_id = diagram_id.is_some();
    let diagram_id_for_liveshare = diagram_id.clone();

    // Rename state
    let is_editing_name = RwSignal::new(false);
    let name_input = RwSignal::new(original_name.with_untracked(|v| v.clone()));
    let renaming = RwSignal::new(false);
    let rename_error = RwSignal::new(None::<String>);

    // Delete state
    let deleting = RwSignal::new(false);
    let delete_error = RwSignal::new(None::<String>);
    let show_delete_confirm = RwSignal::new(false);

    // LiveShare state
    let connection_state = ctx.connection_state;
    let liveshare_error = ctx.error;
    let room_id = ctx.room_id;
    let room_info = ctx.room_info;

    let ctx_disconnect = ctx;
    let disconnect = Callback::new(move |_: ()| {
        ctx_disconnect.disconnect();
        room_id_input.set(String::new());
        password.set(String::new());
    });

    // Rename handler
    let diagram_id_for_rename = diagram_id.clone();
    let handle_rename = Callback::new(move |_: ()| {
        let new_name = name_input.get_untracked();
        if new_name.trim().is_empty() {
            rename_error.set(Some("Name cannot be empty".to_string()));
            return;
        }

        if let Some(ref id) = diagram_id_for_rename {
            let diagram_id = id.clone();
            renaming.set(true);
            rename_error.set(None);

            spawn_local(async move {
                match rename_diagram_api(&diagram_id, &new_name).await {
                    Ok(_) => {
                        renaming.set(false);
                        is_editing_name.set(false);
                        original_name.set(new_name.clone());
                        if let Some(cb) = on_name_change {
                            cb.run(new_name);
                        }
                    }
                    Err(e) => {
                        rename_error.set(Some(e));
                        renaming.set(false);
                    }
                }
            });
        }
    });

    // Delete handler
    let diagram_id_for_delete = diagram_id.clone();
    let handle_delete = Callback::new(move |_: ()| {
        if let Some(ref id) = diagram_id_for_delete {
            let diagram_id = id.clone();
            deleting.set(true);
            delete_error.set(None);

            spawn_local(async move {
                match delete_diagram_api(&diagram_id).await {
                    Ok(_) => {
                        deleting.set(false);
                        let navigate = use_navigate();
                        navigate("/dashboard", Default::default());
                    }
                    Err(e) => {
                        delete_error.set(Some(e));
                        deleting.set(false);
                    }
                }
            });
        }
    });

    // Copy link handler
    #[allow(unused_variables)]
    let diagram_id_for_copy_link = diagram_id.clone();
    let copy_link = StoredValue::new_local(move |_| {
        #[cfg(not(feature = "ssr"))]
        {
            if let Some(rid) = room_id.get() {
                if let Some(ref diagram_id_val) = diagram_id_for_copy_link {
                    if let Some(window) = web_sys::window() {
                        let location = window.location();
                        let protocol = location.protocol().unwrap_or_default();
                        let host = location.host().unwrap_or_default();
                        let link = format!(
                            "{}//{}editor/{}?room={}",
                            protocol, host, diagram_id_val, rid
                        );
                        let js_code = format!("navigator.clipboard.writeText('{}')", link);
                        let _ = js_sys::eval(&js_code);
                    }
                }
            }
        }
    });

    view! {
        <div class="space-y-6">
            // Diagram info section with editable name
            <div class="space-y-3">
                <h3 class="text-sm font-medium text-theme-primary">"Diagram Info"</h3>
                <div class="p-4 bg-theme-tertiary rounded-xl">
                    <div class="flex items-center gap-3">
                        <div class="w-10 h-10 rounded-lg bg-theme-secondary flex items-center justify-center flex-shrink-0">
                            <Icon name=icons::DATABASE class="w-5 h-5 text-theme-secondary"/>
                        </div>
                        <div class="flex-1 min-w-0">
                            {move || if is_editing_name.get() {
                                view! {
                                    <div class="flex flex-col gap-2">
                                        <input
                                            type="text"
                                            class="w-full px-2 py-1 text-sm font-medium text-theme-primary bg-theme-surface border border-theme-primary rounded-lg focus:outline-none focus:border-theme-accent"
                                            prop:value=move || name_input.get()
                                            on:input=move |ev| {
                                                name_input.set(event_target_value(&ev));
                                            }
                                            on:keydown=move |ev| {
                                                if ev.key() == "Enter" {
                                                    handle_rename.run(());
                                                } else if ev.key() == "Escape" {
                                                    is_editing_name.set(false);
                                                    name_input.set(original_name.get());
                                                }
                                            }
                                        />
                                        {move || rename_error.get().map(|e| view! {
                                            <p class="text-xs text-red-500">{e}</p>
                                        })}
                                        <div class="flex items-center gap-2">
                                            <button
                                                class="px-2 py-1 text-xs font-medium text-theme-secondary border border-theme-primary rounded hover:bg-theme-secondary transition-colors"
                                                on:click=move |_| {
                                                    is_editing_name.set(false);
                                                    name_input.set(original_name.get());
                                                }
                                                disabled=move || renaming.get()
                                            >
                                                "Cancel"
                                            </button>
                                            <button
                                                class="px-2 py-1 text-xs font-medium text-white bg-accent-primary hover:bg-accent-secondary rounded transition-colors disabled:opacity-50 flex items-center gap-1"
                                                on:click=move |_| handle_rename.run(())
                                                disabled=move || renaming.get()
                                            >
                                                {move || if renaming.get() {
                                                    view! {
                                                        <Icon name=icons::LOADER class="w-3 h-3 animate-spin"/>
                                                        "Saving..."
                                                    }.into_any()
                                                } else {
                                                    view! { "Save" }.into_any()
                                                }}
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex items-center gap-2 group">
                                        <p class="text-sm font-medium text-theme-primary truncate">
                                            {move || original_name.get()}
                                        </p>
                                        {if !is_demo && has_diagram_id {
                                            view! {
                                                <button
                                                    class="p-1 text-theme-muted hover:text-theme-primary opacity-0 group-hover:opacity-100 transition-all"
                                                    on:click=move |_| is_editing_name.set(true)
                                                    title="Rename diagram"
                                                >
                                                    <Icon name=icons::EDIT class="w-3.5 h-3.5"/>
                                                </button>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                    <p class="text-xs text-theme-muted">
                                        {if is_demo { "Demo diagram" } else { "Saved diagram" }}
                                    </p>
                                }.into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>

            // LiveShare section
            <div class="space-y-3">
                <h3 class="text-sm font-medium text-theme-primary">"LiveShare"</h3>

                // Connection status
                <div class="flex items-center bg-theme-secondary theme-transition gap-3 p-3 rounded-xl">
                    {move || {
                        let state = connection_state.get();
                        match state {
                            ConnectionState::Connected => view! {
                                <div class="w-2.5 h-2.5 bg-green-500 rounded-full animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Connected"</span>
                            }.into_any(),
                            ConnectionState::Connecting => view! {
                                <div class="w-2.5 h-2.5 bg-yellow-500 rounded-full animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Connecting..."</span>
                            }.into_any(),
                            ConnectionState::Reconnecting => view! {
                                <div class="w-2.5 h-2.5 bg-yellow-500 rounded-full animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Reconnecting..."</span>
                            }.into_any(),
                            ConnectionState::Error => view! {
                                <div class="w-2.5 h-2.5 bg-red-500 rounded-full"></div>
                                <span class="text-sm text-theme-secondary">"Error"</span>
                            }.into_any(),
                            ConnectionState::Disconnected => view! {
                                <div class="w-2.5 h-2.5 bg-gray-400 rounded-full"></div>
                                <span class="text-sm text-theme-secondary">"Disconnected"</span>
                            }.into_any(),
                        }
                    }}
                </div>

                // LiveShare content based on connection state
                {
                    #[allow(unused_variables)]
                    let diagram_id = diagram_id_for_liveshare.clone();
                    move || {
                        let state = connection_state.get();
                        if state == ConnectionState::Connected {
                            let diagram_id = diagram_id.clone();
                            // Connected view
                            view! {
                                <div class="p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-xl space-y-3">
                                    // Room info
                                    {move || room_info.get().map(|info| view! {
                                        <p class="text-sm font-medium text-green-700 dark:text-green-300">{info.name}</p>
                                    })}

                                    // Room link
                                    <div class="flex items-center gap-2">
                                        <input
                                            type="text"
                                            readonly
                                            class="flex-1 px-2 py-1.5 text-xs font-mono bg-theme-surface border border-green-300 dark:border-green-700 text-theme-primary rounded-lg"
                                            prop:value={
                                                let _diagram_id = diagram_id.clone();
                                                move || {
                                                    #[cfg(not(feature = "ssr"))]
                                                    {
                                                        room_id.get().map(|id| {
                                                            if let Some(diagram_id_val) = _diagram_id.as_ref() {
                                                                if let Some(window) = web_sys::window() {
                                                                    let location = window.location();
                                                                    let protocol = location.protocol().unwrap_or_default();
                                                                    let host = location.host().unwrap_or_default();
                                                                    format!("{}//{}editor/{}?room={}", protocol, host, diagram_id_val, id)
                                                                } else {
                                                                    id
                                                                }
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
                                            }
                                    />
                                    <button
                                        class="p-1.5 text-green-600 dark:text-green-400 hover:bg-green-100 dark:hover:bg-green-900/30 rounded-lg transition-colors"
                                        on:click=move |ev| copy_link.with_value(|f| f(ev))
                                        title="Copy room link"
                                    >
                                        <Icon name=icons::DOCUMENT_DUPLICATE class="w-4 h-4" />
                                    </button>
                                </div>

                                // Disconnect button
                                <button
                                    class="w-full px-3 py-2 text-xs font-medium text-red-600 dark:text-red-400 border border-red-300 dark:border-red-700 rounded-lg hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
                                    on:click=move |_| disconnect.run(())
                                >
                                    "Disconnect"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        // Disconnected view
                        view! {
                            <LiveShareDisconnectedView
                                ctx=ctx
                                room_id_input=room_id_input
                                room_name=room_name
                                password=password
                                mode=mode
                                connection_state=connection_state
                                error=liveshare_error
                            />
                        }.into_any()
                    }
                }}
            </div>

            // Danger zone section
            {if !is_demo && diagram_id.is_some() {
                view! {
                    <div class="space-y-3">
                        <h3 class="text-sm font-medium text-red-500">"Danger Zone"</h3>
                        <div class="p-4 border border-red-200 dark:border-red-800 rounded-xl bg-red-50/50 dark:bg-red-900/10">
                            <div class="flex items-start gap-3">
                                <div class="flex-shrink-0 w-10 h-10 rounded-lg bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
                                    <Icon name=icons::TRASH class="w-5 h-5 text-red-600 dark:text-red-400"/>
                                </div>
                                <div class="flex-1">
                                    <p class="text-sm font-medium text-theme-primary">"Delete Diagram"</p>
                                    <p class="text-xs text-theme-muted mt-1">
                                        "Permanently delete this diagram and all its data. This action cannot be undone."
                                    </p>

                                    // Error message
                                    {move || delete_error.get().map(|e| view! {
                                        <div class="mt-3 p-2 bg-red-100 dark:bg-red-900/30 border border-red-200 dark:border-red-700 rounded-lg">
                                            <p class="text-xs text-red-700 dark:text-red-300">{e}</p>
                                        </div>
                                    })}

                                    {move || if show_delete_confirm.get() {
                                        view! {
                                            <div class="mt-3 flex items-center gap-2">
                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium text-theme-secondary border border-theme-primary rounded-lg hover:bg-theme-tertiary transition-colors"
                                                    on:click=move |_| show_delete_confirm.set(false)
                                                    disabled=move || deleting.get()
                                                >
                                                    "Cancel"
                                                </button>
                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium text-white bg-red-600 hover:bg-red-700 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-1.5"
                                                    on:click=move |_| handle_delete.run(())
                                                    disabled=move || deleting.get()
                                                >
                                                    {move || if deleting.get() {
                                                        view! {
                                                            <Icon name=icons::LOADER class="w-3 h-3 animate-spin"/>
                                                            "Deleting..."
                                                        }.into_any()
                                                    } else {
                                                        view! { "Confirm Delete" }.into_any()
                                                    }}
                                                </button>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <button
                                                class="mt-3 px-3 py-1.5 text-xs font-medium text-red-600 dark:text-red-400 border border-red-300 dark:border-red-700 rounded-lg hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors"
                                                on:click=move |_| show_delete_confirm.set(true)
                                            >
                                                "Delete this diagram"
                                            </button>
                                        }.into_any()
                                    }}
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="p-4 bg-theme-tertiary rounded-xl">
                        <p class="text-sm text-theme-muted text-center">
                            {if is_demo {
                                "Demo diagrams cannot be deleted."
                            } else {
                                "Save this diagram to enable deletion."
                            }}
                        </p>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// API function to rename diagram
#[cfg(not(feature = "ssr"))]
async fn rename_diagram_api(diagram_id: &str, new_name: &str) -> Result<(), String> {
    use crate::ui::auth::use_auth_context;

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("PUT");

    let body = format!(r#"{{"name":"{}"}}"#, new_name);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("/api/diagrams/{}", diagram_id);
    let req =
        web_sys::Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: web_sys::Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to rename diagram".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn rename_diagram_api(_diagram_id: &str, _new_name: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

/// API function to delete diagram
#[cfg(not(feature = "ssr"))]
async fn delete_diagram_api(diagram_id: &str) -> Result<(), String> {
    use crate::ui::auth::use_auth_context;

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("DELETE");

    let url = format!("/api/diagrams/{}", diagram_id);
    let req =
        web_sys::Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: web_sys::Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to delete diagram".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn delete_diagram_api(_diagram_id: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

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
    /// Diagram name for diagram tab
    #[prop(default = None)]
    diagram_name: Option<String>,
    /// Diagram ID for deletion
    #[prop(default = None)]
    diagram_id: Option<String>,
    /// Whether this is a demo diagram
    #[prop(default = false)]
    is_demo: bool,
    /// Callback when diagram name changes
    #[prop(default = None)]
    on_name_change: Option<Callback<String>>,
) -> impl IntoView {
    // Get the LiveShare context
    let ctx = use_liveshare_context();

    // Room ID input
    let room_id_input = RwSignal::new(String::new());

    // Watch for initial_room_id changes and update room_id_input
    if let Some(initial_id_signal) = initial_room_id {
        Effect::new(move |_| {
            let id = initial_id_signal.get();
            if !id.is_empty() {
                room_id_input.set(id);
            }
        });
    }

    // Room name input (for creating)
    let room_name = RwSignal::new(String::new());

    // Password input
    let password = RwSignal::new(String::new());

    // Mode: "join" or "create"
    let mode = RwSignal::new("join");

    // Active settings tab
    let (active_tab, set_active_tab) = signal("diagram");

    // Close modal handler
    let close_modal = move |_| {
        is_open.set(false);
    };

    // Close on Escape key
    #[cfg(not(feature = "ssr"))]
    {
        use leptos::ev::keydown;

        let handle_keydown = window_event_listener(keydown, move |ev| {
            if ev.key() == "Escape" && is_open.with_untracked(|v| *v) {
                is_open.set(false);
            }
        });

        on_cleanup(move || drop(handle_keydown));
    }

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
                        class="card-header"
                        style="padding: 16px 20px;"
                    >
                        <h2 class="title-lg">"Settings"</h2>
                        <button
                            class="btn-icon"
                            on:click=close_modal
                            title="Close"
                        >
                            <Icon name=icons::X class="icon-standalone"/>
                        </button>
                    </div>

                    // Tabs
                    <div
                        class="flex items-center shrink-0 bg-theme-secondary border-b border-theme-primary theme-transition"
                        style="padding: 12px 16px;"
                    >
                        {
                            let current_tab = Memo::new(move |_| active_tab.get());
                            let set_diagram = Callback::new(move |_: ()| set_active_tab.set("diagram"));
                            let set_canvas = Callback::new(move |_: ()| set_active_tab.set("canvas"));
                            let set_export = Callback::new(move |_: ()| set_active_tab.set("export"));
                            view! {
                                <SettingsTabButton
                                    tab_value="diagram"
                                    current_tab=current_tab
                                    label="Diagram"
                                    icon_path="M4 7v10c0 2 1 3 3 3h10c2 0 3-1 3-3V7c0-2-1-3-3-3H7c-2 0-3 1-3 3zm5-1v4m-2-2h4m4 1h.01M15 14h.01"
                                    on_click=set_diagram
                                />
                                <SettingsTabButton
                                    tab_value="canvas"
                                    current_tab=current_tab
                                    label="Canvas"
                                    icon_path="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"
                                    on_click=set_canvas
                                />
                                <SettingsTabButton
                                    tab_value="export"
                                    current_tab=current_tab
                                    label="Export"
                                    icon_path="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                                    on_click=set_export
                                />
                            }
                        }
                    </div>

                    // Content area
                    <div class="flex-1 overflow-y-auto bg-theme-surface theme-transition" style="padding: 20px;">
                        // Diagram tab content
                        {
                            let diagram_name_clone = diagram_name.clone();
                            let diagram_id_clone = diagram_id.clone();
                            view! {
                                <Show when=move || active_tab.get() == "diagram">
                                    <DiagramTab
                                        diagram_name=diagram_name_clone.clone()
                                        diagram_id=diagram_id_clone.clone()
                                        is_demo=is_demo
                                        on_name_change=on_name_change
                                        ctx=ctx
                                        room_id_input=room_id_input
                                        room_name=room_name
                                        password=password
                                        mode=mode
                                    />
                                </Show>
                            }
                        }

                        // Canvas tab content
                        <Show when=move || active_tab.get() == "canvas">
                            <CanvasTab/>
                        </Show>

                        // Export tab content
                        <Show when=move || active_tab.get() == "export">
                            <ExportTab graph=graph/>
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
            <Icon name=icons::SETTINGS class="w-6 h-6" />
        </button>
    }
}
