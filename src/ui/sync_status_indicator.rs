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

    let get_status_colors = move || {
        let (container, dot) = match connection_state.get() {
            ConnectionState::Disconnected => ("bg-gray-500/10 border-gray-500/30", "bg-gray-400"),
            ConnectionState::Connecting => {
                ("bg-yellow-500/10 border-yellow-500/30", "bg-yellow-400")
            }
            ConnectionState::Connected => match sync_status.get() {
                SyncStatus::Idle => ("bg-blue-500/10 border-blue-500/30", "bg-blue-400"),
                SyncStatus::Syncing => (
                    "bg-blue-500/10 border-blue-500/30",
                    "bg-blue-400 animate-pulse",
                ),
                SyncStatus::Synced => ("bg-green-500/10 border-green-500/30", "bg-green-400"),
                SyncStatus::Error => ("bg-red-500/10 border-red-500/30", "bg-red-400"),
                SyncStatus::Throttled => ("bg-orange-500/10 border-orange-500/30", "bg-orange-400"),
            },
            ConnectionState::Reconnecting => (
                "bg-orange-500/10 border-orange-500/30",
                "bg-orange-400 animate-pulse",
            ),
            ConnectionState::Error => ("bg-red-500/10 border-red-500/30", "bg-red-400"),
        };
        ("text-theme-primary", container, dot)
    };

    view! {
        <Show when=move || is_visible.get()>
            <div class={move || {
                let (_, container, _) = get_status_colors();
                format!(
                    "inline-flex items-center gap-2 px-3 py-1.5 rounded-lg border backdrop-blur-sm transition-all duration-300 {}",
                    container
                )
            }}>
                <div class={move || {
                    let (_, _, dot) = get_status_colors();
                    format!("w-2 h-2 rounded-full {}", dot)
                }}></div>
                <span class="text-xs font-medium text-theme-primary whitespace-nowrap">
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
        <div class="inline-flex items-center gap-2">
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
                                    class={format!("w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold text-white {}", opacity)}
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
                <div class="bg-orange-500/10 border border-orange-500/30 rounded-lg px-4 py-2 flex items-center gap-3 backdrop-blur-sm shadow-lg">
                    <Icon name=icons::ALERT_CIRCLE class="icon-text text-orange-400" />
                    <span class="text-sm text-orange-400">{message}</span>
                    <button
                        class="btn-xs bg-orange-500/20 hover:bg-orange-500/30 text-orange-400"
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
                "inline-flex items-center gap-2 px-2 py-1 rounded text-xs text-blue-400 animate-pulse"
            } else {
                "inline-flex items-center gap-2 px-2 py-1 rounded text-xs text-green-400"
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
