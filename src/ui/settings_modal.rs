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

    let button_class = move || {
        if is_selected() {
            "flex-1 btn btn-sm bg-theme-surface text-theme-primary border-theme-accent"
        } else {
            "flex-1 btn btn-sm btn-ghost text-theme-tertiary"
        }
    };

    view! {
        <button
            type="button"
            class=button_class
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
                let Some(window) = web_sys::window() else {
                    ctx_inner
                        .error
                        .set(Some("Browser window unavailable".to_string()));
                    ctx_inner.connection_state.set(ConnectionState::Error);
                    return;
                };
                let location = window.location();
                let origin = location.origin().unwrap_or_default();

                // Use new endpoint without UUID in URL
                let create_url = format!("{}/room", origin);

                let body = serde_json::json!({
                    "name": if room_name_val.is_empty() { None } else { Some(room_name_val) },
                    "password": if password_val.is_empty() { None } else { Some(&password_val) },
                    "max_users": 50
                });

                let body_str = match serde_json::to_string(&body) {
                    Ok(s) => s,
                    Err(e) => {
                        ctx_inner
                            .error
                            .set(Some(format!("Failed to serialize request: {}", e)));
                        ctx_inner.connection_state.set(ConnectionState::Error);
                        return;
                    }
                };

                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                opts.set_credentials(web_sys::RequestCredentials::Include);
                opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));

                let request = match web_sys::Request::new_with_str_and_init(&create_url, &opts) {
                    Ok(r) => r,
                    Err(e) => {
                        ctx_inner
                            .error
                            .set(Some(format!("Failed to build request: {:?}", e)));
                        ctx_inner.connection_state.set(ConnectionState::Error);
                        return;
                    }
                };
                let _ = request.headers().set("Content-Type", "application/json");

                // Add Authorization header with JWT token from localStorage
                let _ = auth_utils::add_auth_header(&request);

                match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                    .await
                {
                    Ok(resp) => {
                        let resp: web_sys::Response = resp.into();
                        if resp.ok() {
                            // Parse response to get the generated room ID
                            let json_promise = match resp.json() {
                                Ok(p) => p,
                                Err(e) => {
                                    ctx_inner.error.set(Some(format!(
                                        "Failed to read response body: {:?}",
                                        e
                                    )));
                                    ctx_inner.connection_state.set(ConnectionState::Error);
                                    return;
                                }
                            };
                            match wasm_bindgen_futures::JsFuture::from(json_promise).await {
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
        <div class="space-y-4">
            // Tab selector
            <div class="flex gap-1 rounded-lg border border-theme-primary bg-theme-tertiary p-1 theme-transition">
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
                <div class="rounded-lg border border-theme-error bg-theme-error p-3 text-sm text-theme-error theme-transition">
                    {err}
                </div>
            })}

            // Room ID input (only in join mode)
            {move || if mode.get() == "join" {
                view! {
                    <div class="space-y-1.5">
                        <label class="label">
                            "Room ID"
                        </label>
                        <input
                            type="text"
                            class="input-theme input-lg font-mono"
                            autocomplete="off"
                            spellcheck="false"
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
                    <div class="space-y-1.5">
                        <label class="label">
                            "Room Name " <span class="text-theme-muted">"(optional)"</span>
                        </label>
                        <input
                            type="text"
                            class="input-theme input-lg"
                            autocomplete="off"
                            spellcheck="false"
                            placeholder="Architecture review"
                            prop:value=move || room_name.get()
                            on:input=move |ev| room_name.set(event_target_value(&ev))
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
                    class="input-theme input-lg"
                    autocomplete="current-password"
                    placeholder=move || if mode.get() == "create" { "Set a password" } else { "Enter room password" }
                    prop:value=move || password.get()
                    on:input=move |ev| password.set(event_target_value(&ev))
                />
            </div>

            // Submit button
            {
                view! {
                    <button
                        type="button"
                        class="btn-primary btn-lg w-full disabled:opacity-50"
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
            <p class="text-center text-xs text-theme-muted">
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
    icon_name: &'static str,
    #[prop(default = false)] danger: bool,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_tab.get() == tab_value;

    let button_class = move || {
        if danger && is_selected() {
            "w-full min-h-8 inline-flex items-center justify-start gap-2 rounded-md border border-theme-error bg-theme-error px-2.5 text-left text-[12.5px] font-medium text-theme-error theme-transition"
        } else if danger {
            "w-full min-h-8 inline-flex items-center justify-start gap-2 rounded-md border border-transparent px-2.5 text-left text-[12.5px] font-medium text-theme-error hover:bg-theme-error theme-transition"
        } else if is_selected() {
            "w-full min-h-8 inline-flex items-center justify-start gap-2 rounded-md border border-theme-primary bg-theme-tertiary px-2.5 text-left text-[12.5px] font-medium text-theme-primary theme-transition"
        } else {
            "w-full min-h-8 inline-flex items-center justify-start gap-2 rounded-md border border-transparent px-2.5 text-left text-[12.5px] font-medium text-theme-secondary hover:bg-theme-tertiary hover:text-theme-primary theme-transition"
        }
    };

    view! {
        <button
            type="button"
            class=button_class
            on:click=move |_| on_click.run(())
        >
            <Icon name=icon_name class="w-3.5 h-3.5 flex-shrink-0" />
            {label}
        </button>
    }
    .into_any()
}

/// Two-column row used by the command-center settings layout.
#[component]
fn SettingsRow(label: &'static str, hint: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="settings-row">
            <div>
                <div class="text-[12.5px] font-medium text-theme-primary">{label}</div>
                {(!hint.is_empty()).then(|| view! {
                    <div class="mt-1 text-[11.5px] leading-snug text-theme-muted">{hint}</div>
                })}
            </div>
            <div class="min-w-0">{children()}</div>
        </div>
    }
}

/// Disabled controls keep reference-only settings visible without persisting new model fields.
#[component]
fn DisabledOption(label: &'static str, selected: bool) -> impl IntoView {
    let class = move || {
        if selected {
            "btn btn-sm border-theme-accent bg-theme-accent-light text-theme-accent"
        } else {
            "btn btn-sm opacity-50"
        }
    };

    view! {
        <button type="button" class=class disabled=true>
            {label}
        </button>
    }
}

#[component]
fn DisabledPreference(label: &'static str, enabled: bool) -> impl IntoView {
    let switch_style = move || {
        if enabled {
            "background: var(--primary); border-color: var(--primary); justify-content: flex-end;"
        } else {
            "background: var(--muted); border-color: var(--border); justify-content: flex-start;"
        }
    };

    view! {
        <div class="flex items-center gap-3 rounded-lg border border-theme-primary bg-theme-secondary px-3 py-2 opacity-70">
            <span
                class="inline-flex h-[18px] w-[30px] items-center rounded-full border p-[2px] theme-transition"
                style=switch_style
            >
                <span class="h-3 w-3 rounded-full bg-theme-surface shadow-theme-sm"></span>
            </span>
            <span class="text-[12.5px] text-theme-secondary">{label}</span>
        </div>
    }
}

/// Theme button component - isolated to reduce type nesting
#[component]
fn ThemeButton(
    mode: ThemeMode,
    current_mode: Memo<ThemeMode>,
    label: &'static str,
    icon_name: &'static str,
    on_click: Callback<()>,
) -> impl IntoView {
    let is_selected = move || current_mode.get() == mode;

    let button_class = move || {
        if is_selected() {
            "flex min-h-[60px] flex-1 flex-col items-start justify-between rounded-lg border border-theme-accent bg-theme-accent-light px-3 py-2 text-left theme-transition"
        } else {
            "flex min-h-[60px] flex-1 flex-col items-start justify-between rounded-lg border border-theme-primary bg-theme-secondary px-3 py-2 text-left hover:border-theme-secondary theme-transition"
        }
    };

    let label_class = move || {
        if is_selected() {
            "inline-flex items-center gap-1.5 text-[12.5px] font-medium text-theme-accent"
        } else {
            "inline-flex items-center gap-1.5 text-[12.5px] font-medium text-theme-secondary"
        }
    };

    let preview_style = match mode {
        ThemeMode::Auto => {
            "background: linear-gradient(90deg, var(--canvas) 50%, var(--card-elev) 50%);"
        }
        ThemeMode::Dark => "background: linear-gradient(90deg, var(--canvas), var(--muted));",
        ThemeMode::Light => "background: linear-gradient(90deg, var(--card-elev), var(--muted));",
    };

    view! {
        <button
            type="button"
            class=button_class
            on:click=move |_| on_click.run(())
        >
            <span class=label_class>
                <Icon name=icon_name class="w-3.5 h-3.5" />
                {label}
            </span>
            <span class="h-1.5 w-full rounded-full" style=preview_style></span>
        </button>
    }
    .into_any()
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
        <div class="space-y-3">
            <div class="flex gap-2">
                <ThemeButton
                    mode=ThemeMode::Auto
                    current_mode=current_mode
                    label="Auto"
                    icon_name=icons::SETTINGS
                    on_click=set_auto
                />
                <ThemeButton
                    mode=ThemeMode::Dark
                    current_mode=current_mode
                    label="Dark"
                    icon_name=icons::MOON
                    on_click=set_dark
                />
                <ThemeButton
                    mode=ThemeMode::Light
                    current_mode=current_mode
                    label="Light"
                    icon_name=icons::SUN
                    on_click=set_light
                />
            </div>

            // Current theme indicator
            <p class="text-[11.5px] text-theme-muted">
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
    }
    .into_any()
}

/// Canvas/Theme tab content component
#[component]
fn CanvasTab() -> impl IntoView {
    view! {
        <div>
            <SettingsRow
                label="Theme"
                hint="Editor color scheme. Auto follows your OS preference."
            >
                <ThemeSelector/>
            </SettingsRow>

            <SettingsRow
                label="Accent color"
                hint="One strong color for selection, primary actions, and relation highlights."
            >
                <div class="flex flex-wrap gap-1.5">
                    <DisabledOption label="Verdant" selected=true />
                    <DisabledOption label="Cobalt" selected=false />
                    <DisabledOption label="Amber" selected=false />
                    <DisabledOption label="Plum" selected=false />
                    <DisabledOption label="Mist" selected=false />
                </div>
                <p class="mt-2 text-[11.5px] text-theme-muted">
                    "Palette editing is disabled in this pass; ArchiSchema Foundations remains the source of truth."
                </p>
            </SettingsRow>

            <SettingsRow
                label="Grid background"
                hint="Visual reference inside the canvas."
            >
                <div class="flex flex-wrap gap-1.5">
                    <DisabledOption label="Dots" selected=true />
                    <DisabledOption label="Lines" selected=false />
                    <DisabledOption label="Off" selected=false />
                </div>
            </SettingsRow>

            <SettingsRow
                label="Density"
                hint="Compact fits more on screen; comfy is easier to scan."
            >
                <div class="flex flex-wrap gap-1.5">
                    <DisabledOption label="Compact" selected=false />
                    <DisabledOption label="Cozy" selected=true />
                    <DisabledOption label="Comfy" selected=false />
                </div>
            </SettingsRow>

            <SettingsRow label="Editor preferences" hint="Shown for parity with the design reference; not persisted yet.">
                <div class="grid gap-2 sm:grid-cols-2">
                    <DisabledPreference label="Snap tables to grid" enabled=true />
                    <DisabledPreference label="Auto-route relation lines" enabled=true />
                    <DisabledPreference label="Show column types inline" enabled=true />
                    <DisabledPreference label="Highlight orphaned tables" enabled=false />
                    <DisabledPreference label="Animate panel transitions" enabled=false />
                </div>
            </SettingsRow>
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

    let button_class = move || {
        if is_selected() {
            "flex min-h-[58px] flex-1 flex-col items-start justify-between rounded-lg border border-theme-accent bg-theme-accent-light px-3 py-2 text-left text-theme-accent theme-transition"
        } else {
            "flex min-h-[58px] flex-1 flex-col items-start justify-between rounded-lg border border-theme-primary bg-theme-secondary px-3 py-2 text-left text-theme-secondary hover:border-theme-secondary theme-transition"
        }
    };

    view! {
        <button
            type="button"
            class=button_class
            on:click=move |_| on_click.run(())
        >
            <Icon name=icon_name class="w-4 h-4"/>
            <span class="text-[12.5px] font-medium">
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
        <div>
            <div class="flex gap-2">
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
    }
    .into_any()
}

/// Export tab content component
#[component]
fn ExportTab(graph: Option<RwSignal<SchemaGraph>>) -> impl IntoView {
    let (export_format, set_export_format) = signal("sql");
    let (_export_result, set_export_result) = signal::<Option<String>>(None);
    let (export_filename, set_export_filename) = signal(String::from("schema"));
    let schema_stats = move || {
        graph
            .map(|g| {
                g.with(|graph| {
                    let table_count = graph.node_count();
                    let column_count = graph
                        .node_weights()
                        .map(|table| table.columns.len())
                        .sum::<usize>();
                    let relation_count = graph.edge_count();
                    format!(
                        "{} tables · {} columns · {} relations",
                        table_count, column_count, relation_count
                    )
                })
            })
            .unwrap_or_else(|| "No schema loaded".to_string())
    };

    view! {
        <div>
            <SettingsRow label="Format" hint="Choose the generated file format.">
                <ExportFormatSelector export_format=export_format set_export_format=set_export_format/>
            </SettingsRow>

            <SettingsRow label="Filename" hint="Downloaded file name; extension follows the selected format.">
                <div class="flex items-center gap-2">
                    <input
                        type="text"
                        class="input-theme input-lg font-mono"
                        placeholder="schema"
                        autocomplete="off"
                        spellcheck="false"
                        prop:value=move || export_filename.get()
                        on:input=move |ev| set_export_filename.set(event_target_value(&ev))
                    />
                    <span class="font-mono text-xs text-theme-muted">
                        {move || format!(".{}", export_format.get())}
                    </span>
                </div>
            </SettingsRow>

            <SettingsRow label="Dialect" hint="Current exporter behavior stays unchanged in this redesign pass.">
                <div class="flex flex-wrap gap-1.5">
                    <DisabledOption label="MySQL" selected=true />
                    <DisabledOption label="PostgreSQL" selected=false />
                    <DisabledOption label="SQLite" selected=false />
                </div>
            </SettingsRow>

            <SettingsRow label="Options" hint="Unsupported export toggles are visible but disabled until the exporter flow is expanded.">
                <div class="grid gap-2 sm:grid-cols-2">
                    <DisabledPreference label="Pretty print" enabled=true />
                    <DisabledPreference label="Include positions" enabled=true />
                    <DisabledPreference label="Include DROP statements" enabled=false />
                    <DisabledPreference label="Confirm SQL diff before apply" enabled=false />
                </div>
            </SettingsRow>

            <SettingsRow label="Summary" hint="Export uses the current in-memory schema graph.">
                <div class="rounded-lg border border-theme-primary bg-theme-secondary p-3">
                    <div class="eyebrow">"Schema size"</div>
                    <div class="mt-1 font-mono text-sm text-theme-primary">{schema_stats}</div>
                    <p class="mt-2 text-[11.5px] leading-snug text-theme-muted">
                        {move || match export_format.get() {
                            "json" => "JSON includes tables, columns, relationships, and canvas positions.",
                            "csv" => "CSV exports separate tabular sections for spreadsheet review.",
                            _ => "SQL exports CREATE TABLE statements with the current MySQL dialect path.",
                        }}
                    </p>
                </div>
            </SettingsRow>

            <div class="mt-5 flex items-center justify-end gap-2">
                {move || {
                    if let Some(g) = graph {
                        view! {
                            <button
                                type="button"
                                class="btn-primary btn-lg"
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
                                <Icon name=icons::ARROW_DOWN_TO_LINE class="w-4 h-4" />
                                "Download"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button type="button" class="btn-primary btn-lg" disabled=true>
                                <Icon name=icons::ARROW_DOWN_TO_LINE class="w-4 h-4" />
                                "No schema loaded"
                            </button>
                        }.into_any()
                    }
                }}
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
    active_tab: ReadSignal<&'static str>,
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
                            "{}//{}/editor/{}?room={}",
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
            <Show when=move || active_tab.get() == "diagram">
            // Diagram info section with editable name
            <div class="space-y-3">
                <div class="eyebrow">"Identity"</div>
                <div class="rounded-xl border border-theme-primary bg-theme-secondary p-4">
                    <div class="flex items-center gap-3">
                        <div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border border-theme-primary bg-theme-surface">
                            <Icon name=icons::DATABASE class="w-5 h-5 text-theme-accent"/>
                        </div>
                        <div class="flex-1 min-w-0">
                            {move || if is_editing_name.get() {
                                view! {
                                    <div class="flex flex-col gap-2">
                                        <input
                                            type="text"
                                            class="input-theme input-lg font-medium"
                                            autocomplete="off"
                                            spellcheck="false"
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
                                                type="button"
                                                class="btn btn-sm"
                                                on:click=move |_| {
                                                    is_editing_name.set(false);
                                                    name_input.set(original_name.get());
                                                }
                                                disabled=move || renaming.get()
                                            >
                                                "Cancel"
                                            </button>
                                            <button
                                                type="button"
                                                class="btn-primary btn-sm disabled:opacity-50"
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
                                        <span class="badge-default text-[10.5px]">
                                            {if is_demo { "demo" } else { "saved" }}
                                        </span>
                                        {if !is_demo && has_diagram_id {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="btn-icon btn-sm opacity-0 transition-opacity group-hover:opacity-100"
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
                                    <p class="mt-1 text-xs text-theme-muted">
                                        {if is_demo { "Demo diagrams are read-only examples." } else { "Renames save to your account." }}
                                    </p>
                                }.into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>
            </Show>

            <Show when=move || active_tab.get() == "collaboration">
            // LiveShare section
            <div class="space-y-3">
                <div class="eyebrow">"LiveShare"</div>

                // Connection status
                <div class="flex items-center gap-3 rounded-xl border border-theme-primary bg-theme-secondary p-3 theme-transition">
                    {move || {
                        let state = connection_state.get();
                        match state {
                            ConnectionState::Connected => view! {
                                <div class="h-2.5 w-2.5 rounded-full bg-theme-accent animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Connected"</span>
                            }.into_any(),
                            ConnectionState::Connecting => view! {
                                <div class="h-2.5 w-2.5 rounded-full bg-theme-warning animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Connecting..."</span>
                            }.into_any(),
                            ConnectionState::Reconnecting => view! {
                                <div class="h-2.5 w-2.5 rounded-full bg-theme-warning animate-pulse"></div>
                                <span class="text-sm text-theme-secondary">"Reconnecting..."</span>
                            }.into_any(),
                            ConnectionState::Error => view! {
                                <div class="h-2.5 w-2.5 rounded-full bg-theme-error"></div>
                                <span class="text-sm text-theme-secondary">"Error"</span>
                            }.into_any(),
                            ConnectionState::Disconnected => view! {
                                <div class="h-2.5 w-2.5 rounded-full bg-theme-tertiary"></div>
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
                                <div class="space-y-3 rounded-xl border border-theme-accent bg-theme-accent-light p-4">
                                    // Room info
                                    {move || room_info.get().map(|info| view! {
                                        <p class="text-sm font-medium text-theme-primary">{info.name}</p>
                                    })}

                                    // Room link
                                    <div class="flex items-center gap-2">
                                        <input
                                            type="text"
                                            readonly
                                            class="input-theme input-sm flex-1 font-mono"
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
                                                                    format!("{}//{}/editor/{}?room={}", protocol, host, diagram_id_val, id)
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
                                        type="button"
                                        class="btn-icon"
                                        on:click=move |ev| copy_link.with_value(|f| f(ev))
                                        title="Copy room link"
                                    >
                                        <Icon name=icons::DOCUMENT_DUPLICATE class="w-4 h-4" />
                                    </button>
                                </div>

                                // Disconnect button
                                <button
                                    type="button"
                                    class="btn-danger btn-sm w-full"
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
            </Show>

            <Show when=move || active_tab.get() == "danger">
            // Danger zone section
            {if !is_demo && diagram_id.is_some() {
                view! {
                    <div class="space-y-3">
                        <div class="eyebrow text-theme-error">"Danger zone"</div>
                        {move || if connection_state.get() == ConnectionState::Connected {
                            view! {
                                <div class="mb-3 flex items-center gap-3 rounded-xl border border-theme-primary bg-theme-secondary p-4">
                                    <div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border border-theme-primary bg-theme-surface">
                                        <Icon name=icons::USERS class="w-5 h-5 text-theme-secondary"/>
                                    </div>
                                    <div class="min-w-0 flex-1">
                                        <p class="text-sm font-medium text-theme-primary">"Disconnect from LiveShare room"</p>
                                        <p class="mt-1 text-xs text-theme-muted">"Stop receiving collaborator updates. Your local copy stays open."</p>
                                    </div>
                                    <button type="button" class="btn btn-sm" on:click=move |_| disconnect.run(())>
                                        "Disconnect"
                                    </button>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }}
                        <div class="rounded-xl border border-theme-error bg-theme-error p-4">
                            <div class="flex items-start gap-3">
                                <div class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border border-theme-error bg-theme-surface">
                                    <Icon name=icons::TRASH class="w-5 h-5 text-theme-error"/>
                                </div>
                                <div class="flex-1">
                                    <p class="text-sm font-medium text-theme-primary">"Delete Diagram"</p>
                                    <p class="text-xs text-theme-muted mt-1">
                                        "Permanently delete this diagram and all its data. This action cannot be undone."
                                    </p>

                                    // Error message
                                    {move || delete_error.get().map(|e| view! {
                                        <div class="mt-3 rounded-lg border border-theme-error bg-theme-error p-2">
                                            <p class="text-xs text-theme-error">{e}</p>
                                        </div>
                                    })}

                                    {move || if show_delete_confirm.get() {
                                        view! {
                                            <div class="mt-3 flex items-center gap-2">
                                                <button
                                                    type="button"
                                                    class="btn btn-sm"
                                                    on:click=move |_| show_delete_confirm.set(false)
                                                    disabled=move || deleting.get()
                                                >
                                                    "Cancel"
                                                </button>
                                                <button
                                                    type="button"
                                                    class="btn-danger btn-sm disabled:opacity-50"
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
                                                type="button"
                                                class="btn-danger btn-sm mt-3"
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
            </Show>
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

    let current_tab = Memo::new(move |_| active_tab.get());
    let set_diagram = Callback::new(move |_: ()| set_active_tab.set("diagram"));
    let set_collaboration = Callback::new(move |_: ()| set_active_tab.set("collaboration"));
    let set_canvas = Callback::new(move |_: ()| set_active_tab.set("canvas"));
    let set_export = Callback::new(move |_: ()| set_active_tab.set("export"));
    let set_danger = Callback::new(move |_: ()| set_active_tab.set("danger"));

    let active_title = move || match active_tab.get() {
        "collaboration" => "Collaboration",
        "canvas" => "Canvas",
        "export" => "Export",
        "danger" => "Danger zone",
        _ => "Diagram",
    };
    let active_subtitle = move || match active_tab.get() {
        "collaboration" => "Join, create, and share LiveShare rooms.",
        "canvas" => "How the editor looks and behaves during long sessions.",
        "export" => "Generate SQL, JSON, or CSV from the current schema graph.",
        "danger" => "Destructive actions that cannot be undone.",
        _ => "Rename the saved diagram and review its state.",
    };

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

                // Command-center modal content
                <div
                    class="dialog settings-command-dialog relative theme-transition"
                >
                    <nav class="settings-rail">
                        <div class="flex items-center gap-2 px-2 pb-3 pt-1">
                            <Icon name=icons::SETTINGS class="w-3.5 h-3.5 text-theme-accent" />
                            <span class="text-sm font-semibold text-theme-primary">"Settings"</span>
                        </div>

                        <SettingsTabButton tab_value="diagram" current_tab=current_tab label="Diagram" icon_name=icons::DATABASE on_click=set_diagram />
                        <SettingsTabButton tab_value="collaboration" current_tab=current_tab label="Collaboration" icon_name=icons::USERS on_click=set_collaboration />
                        <SettingsTabButton tab_value="canvas" current_tab=current_tab label="Canvas" icon_name=icons::LAYOUT on_click=set_canvas />
                        <SettingsTabButton tab_value="export" current_tab=current_tab label="Export" icon_name=icons::ARROW_DOWN_TO_LINE on_click=set_export />
                        <SettingsTabButton tab_value="danger" current_tab=current_tab label="Danger zone" icon_name=icons::WARNING danger=true on_click=set_danger />

                        <div class="flex-1"></div>
                        <div class="border-t border-theme-primary px-2 pt-3 text-[11px] leading-snug text-theme-muted">
                            "Press " <span class="kbd-key">"Esc"</span> " to close"
                        </div>
                    </nav>

                    <div class="flex min-h-0 flex-col bg-theme-surface">
                        <div class="flex items-start justify-between gap-4 px-6 pb-3 pt-4">
                            <div>
                                <div class="dialog-title">{active_title}</div>
                                <div class="dialog-sub">{active_subtitle}</div>
                            </div>
                            <button
                                type="button"
                                class="btn-icon"
                                on:click=close_modal
                                title="Close"
                            >
                                <Icon name=icons::X class="icon-standalone"/>
                            </button>
                        </div>

                        <div class="scroll flex-1 overflow-y-auto px-6 pb-4">
                            {
                                let diagram_name_clone = diagram_name.clone();
                                let diagram_id_clone = diagram_id.clone();
                                view! {
                                    <Show when=move || matches!(active_tab.get(), "diagram" | "collaboration" | "danger")>
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
                                            active_tab=active_tab
                                        />
                                    </Show>
                                }
                            }

                            <Show when=move || active_tab.get() == "canvas">
                                <CanvasTab/>
                            </Show>

                            <Show when=move || active_tab.get() == "export">
                                <ExportTab graph=graph/>
                            </Show>
                        </div>

                        <div class="flex items-center justify-between border-t border-theme-primary px-6 py-3">
                            <span class="text-[11.5px] text-theme-muted">
                                "Changes apply immediately unless the control is marked disabled."
                            </span>
                            <button type="button" class="btn-primary" on:click=close_modal>
                                "Done"
                            </button>
                        </div>
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
            type="button"
            class="fixed bottom-4 right-4 z-40 flex h-11 w-11 items-center justify-center rounded-xl border border-theme-primary bg-theme-surface text-theme-secondary shadow-theme-lg transition-colors theme-transition hover:border-theme-accent hover:text-theme-accent"
            on:click=move |_| is_open.set(true)
            title="Settings"
        >
            <Icon name=icons::SETTINGS class="w-5 h-5" />
        </button>
    }
}
