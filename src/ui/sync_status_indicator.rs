//! Synchronization status indicator component for LiveShare
//!
//! Displays the current synchronization status with animated indicators
//! and provides visual feedback on connection and sync state.

use crate::ui::liveshare_client::{ConnectionState, SyncStatus, use_liveshare_context};
use crate::ui::{Icon, icons};
use leptos::prelude::*;

/// Calculate elapsed seconds from a timestamp in milliseconds
#[cfg(target_arch = "wasm32")]
fn elapsed_secs(timestamp_ms: f64) -> u64 {
    let now = js_sys::Date::now();
    let elapsed_ms = now - timestamp_ms;
    (elapsed_ms / 1000.0) as u64
}

/// Calculate elapsed seconds from a timestamp in milliseconds
#[cfg(not(target_arch = "wasm32"))]
fn elapsed_secs(timestamp_ms: f64) -> u64 {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    let elapsed_ms = now - timestamp_ms;
    (elapsed_ms / 1000.0) as u64
}

/// Sync status badge component
/// Shows current sync state with visual indicators
#[component]
pub fn SyncStatusBadge() -> impl IntoView {
    let ctx = use_liveshare_context();
    let sync_status = ctx.sync_status;
    let connection_state = ctx.connection_state;
    let pending_updates = ctx.pending_updates;

    let (is_visible, _set_is_visible) = signal(true);

    let status_text = move || match connection_state.get() {
        ConnectionState::Disconnected => "Disconnected".to_string(),
        ConnectionState::Connecting => "Connecting...".to_string(),
        ConnectionState::Connected => match sync_status.get() {
            SyncStatus::Idle => "Ready".to_string(),
            SyncStatus::Syncing => {
                let pending = pending_updates.get();
                if pending > 0 {
                    format!("Syncing... ({} updates)", pending)
                } else {
                    "Syncing...".to_string()
                }
            }
            SyncStatus::Synced => "Synced".to_string(),
            SyncStatus::Error => "Sync error".to_string(),
            SyncStatus::Throttled => "Throttled".to_string(),
        },
        ConnectionState::Reconnecting => "Reconnecting...".to_string(),
        ConnectionState::Error => "Connection error".to_string(),
    };

    let get_status_styles = move || match connection_state.get() {
        ConnectionState::Disconnected => (
            "color: var(--muted-foreground); border-color: color-mix(in oklab, var(--muted-foreground) 28%, transparent); background: color-mix(in oklab, var(--muted-foreground) 10%, transparent);",
            "background: var(--muted-foreground);",
            "",
        ),
        ConnectionState::Connecting => (
            "color: var(--warning); border-color: color-mix(in oklab, var(--warning) 35%, transparent); background: var(--warning-soft);",
            "background: var(--warning);",
            "animate-pulse",
        ),
        ConnectionState::Connected => match sync_status.get() {
            SyncStatus::Idle => (
                "color: var(--info); border-color: color-mix(in oklab, var(--info) 35%, transparent); background: var(--info-soft);",
                "background: var(--info);",
                "",
            ),
            SyncStatus::Syncing => (
                "color: var(--info); border-color: color-mix(in oklab, var(--info) 35%, transparent); background: var(--info-soft);",
                "background: var(--info);",
                "animate-pulse",
            ),
            SyncStatus::Synced => (
                "color: var(--success); border-color: var(--success-border); background: var(--success-soft);",
                "background: var(--success);",
                "",
            ),
            SyncStatus::Error => (
                "color: var(--destructive); border-color: var(--error-border); background: var(--destructive-soft);",
                "background: var(--destructive);",
                "",
            ),
            SyncStatus::Throttled => (
                "color: var(--warning); border-color: var(--warning-border); background: var(--warning-soft);",
                "background: var(--warning);",
                "",
            ),
        },
        ConnectionState::Reconnecting => (
            "color: var(--warning); border-color: var(--warning-border); background: var(--warning-soft);",
            "background: var(--warning);",
            "animate-pulse",
        ),
        ConnectionState::Error => (
            "color: var(--destructive); border-color: var(--error-border); background: var(--destructive-soft);",
            "background: var(--destructive);",
            "",
        ),
    };

    view! {
        <Show when=move || is_visible.get()>
            <div
                class="inline-flex items-center gap-2 rounded-lg border px-3 py-1.5 backdrop-blur-sm theme-transition"
                style=move || get_status_styles().0
            >
                <div class={move || {
                    let (_, _, pulse) = get_status_styles();
                    format!("h-2 w-2 rounded-full {}", pulse)
                }} style=move || get_status_styles().1></div>
                <span class="whitespace-nowrap text-xs font-medium">
                    {status_text}
                </span>
            </div>
        </Show>
    }
}

/// User presence indicator showing active users in the room
#[component]
pub fn UserPresenceIndicator() -> impl IntoView {
    let ctx = use_liveshare_context();
    let room_info = ctx.room_info;
    let remote_users = ctx.remote_users;

    let user_count = move || room_info.get().map(|info| info.user_count).unwrap_or(0);

    let active_users = move || {
        remote_users
            .get()
            .into_iter()
            .filter(|u| u.is_active)
            .collect::<Vec<_>>()
    };

    let idle_users = move || {
        remote_users
            .get()
            .into_iter()
            .filter(|u| !u.is_active)
            .collect::<Vec<_>>()
    };

    view! {
        <div class="inline-flex items-center gap-2 rounded-lg border border-theme-primary bg-theme-secondary px-2.5 py-1.5">
            <Icon name=icons::USER_PLUS class="icon-text text-theme-secondary" />
            <span class="text-xs font-medium text-theme-primary">
                {move || format!("{} user{}", user_count(), if user_count() != 1 { "s" } else { "" })}
            </span>
            <Show when=move || !active_users().is_empty() || !idle_users().is_empty()>
                <div class="flex items-center gap-1">
                    {move || {
                        let mut all_users = active_users();
                        all_users.extend(idle_users());
                        all_users.into_iter().take(3).map(|user| {
                            let initials = user.username.chars()
                                .take(2)
                                .collect::<String>()
                                .to_uppercase();
                            let opacity = if user.is_active { "opacity-100" } else { "opacity-50" };
                            view! {
                                <div
                                    class={format!("flex h-5 w-5 items-center justify-center rounded-full text-xs font-bold text-white {}", opacity)}
                                    style=move || format!("background-color: {}", user.color)
                                    title={if user.is_active { format!("{} (active)", user.username) } else { format!("{} (idle)", user.username) }}
                                >
                                    {initials}
                                </div>
                            }
                        }).collect_view()
                    }}
                    <Show when=move || { user_count() > 3 }>
                        <span class="text-xs text-theme-secondary">
                            {move || format!("+{}", user_count() - 3)}
                        </span>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// Connection status bar showing recovery information
#[component]
pub fn ConnectionStatusBar() -> impl IntoView {
    let ctx = use_liveshare_context();
    let connection_state = ctx.connection_state;
    let connection_lost_since = ctx.connection_lost_since;

    let should_show = move || {
        matches!(
            connection_state.get(),
            ConnectionState::Error | ConnectionState::Reconnecting
        )
    };

    let lost_duration = move || {
        if let Some(lost_time) = connection_lost_since.get() {
            let elapsed = elapsed_secs(lost_time);
            if elapsed < 60 {
                format!("{}s", elapsed)
            } else {
                format!("{}m", elapsed / 60)
            }
        } else {
            String::new()
        }
    };

    let message = move || match connection_state.get() {
        ConnectionState::Error => {
            format!("Connection lost. Reconnecting... ({})", lost_duration())
        }
        ConnectionState::Reconnecting => {
            format!("Reconnecting... ({})", lost_duration())
        }
        _ => String::new(),
    };

    let retry = move |_| {
        ctx.connect(ctx.room_id.get().unwrap_or_default(), None);
    };

    view! {
        <Show when=should_show>
            <div class="fixed top-20 left-1/2 transform -translate-x-1/2 z-50">
                <div class="flex items-center gap-3 rounded-lg border border-theme-warning bg-theme-warning px-4 py-2 shadow-theme-lg backdrop-blur-sm">
                    <Icon name=icons::ALERT_CIRCLE class="icon-text text-theme-warning" />
                    <span class="text-sm text-theme-warning">{message}</span>
                    <button
                        type="button"
                        class="btn-warning btn-sm"
                        on:click=retry
                    >
                        "Retry Now"
                    </button>
                </div>
            </div>
        </Show>
    }
}

/// Snapshot save progress indicator
#[component]
pub fn SnapshotSaveIndicator() -> impl IntoView {
    let ctx = use_liveshare_context();
    let snapshot_saving = ctx.snapshot_saving;
    let last_snapshot_time = ctx.last_snapshot_time;

    let status_text = move || {
        if snapshot_saving.get() {
            "Saving snapshot...".to_string()
        } else if let Some(last_time) = last_snapshot_time.get() {
            let elapsed = elapsed_secs(last_time);
            if elapsed < 60 {
                format!("Snapshot saved {}s ago", elapsed)
            } else {
                format!("Snapshot saved {}m ago", elapsed / 60)
            }
        } else {
            "Snapshot ready".to_string()
        }
    };

    view! {
        <div class={move || {
            if snapshot_saving.get() {
                "inline-flex items-center gap-2 rounded-lg px-2 py-1 text-xs text-theme-accent animate-pulse"
            } else {
                "inline-flex items-center gap-2 rounded-lg px-2 py-1 text-xs text-theme-success"
            }
        }}>
            <Icon name={
                if snapshot_saving.get() {
                    icons::LOADER
                } else {
                    icons::CHECK
                }
            } class="icon-text" />
            <span class="whitespace-nowrap">{status_text}</span>
        </div>
    }
}
