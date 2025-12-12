//! Notification component for canvas display
//!
//! Provides toast-style notifications for displaying errors, warnings,
//! success messages, and info on the canvas.

use crate::core::{CanvasNotification, NotificationType};
use leptos::prelude::*;
use std::collections::VecDeque;

/// Maximum number of notifications to show at once
const MAX_NOTIFICATIONS: usize = 5;

/// Notification item with unique ID for tracking
#[derive(Clone, Debug)]
pub struct NotificationItem {
    pub id: u64,
    pub notification: CanvasNotification,
}

/// Notifications container component
/// Place this at the canvas level to show notifications
#[component]
pub fn NotificationsContainer(
    /// Signal containing the list of notifications
    notifications: RwSignal<VecDeque<NotificationItem>>,
) -> impl IntoView {
    view! {
        <div class="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
            {move || {
                notifications.get().into_iter().map(|item| {
                    let id = item.id;
                    let notification = item.notification.clone();
                    let notifications_signal = notifications;

                    view! {
                        <NotificationToastSimple
                            notification=notification
                            id=id
                            notifications=notifications_signal
                        />
                    }
                }).collect_view()
            }}
        </div>
    }
}

/// Single notification toast component (simplified version)
#[component]
fn NotificationToastSimple(
    notification: CanvasNotification,
    id: u64,
    notifications: RwSignal<VecDeque<NotificationItem>>,
) -> impl IntoView {
    let (is_visible, _set_is_visible) = signal(true);
    let (is_exiting, _set_is_exiting) = signal(false);

    // Auto-dismiss if specified
    if let Some(_ms) = notification.auto_dismiss_ms {
        #[cfg(not(feature = "ssr"))]
        {
            use gloo_timers::future::TimeoutFuture;
            use wasm_bindgen_futures::spawn_local;

            spawn_local(async move {
                TimeoutFuture::new(_ms).await;
                _set_is_exiting.set(true);
                // Wait for exit animation
                TimeoutFuture::new(300).await;
                _set_is_visible.set(false);
                // Remove from list
                notifications.update(|n| {
                    n.retain(|i| i.id != id);
                });
            });
        }
    }

    let (bg_class, border_class, icon_class) = match notification.notification_type {
        NotificationType::Success => ("bg-green-500/10", "border-green-500/30", "text-green-400"),
        NotificationType::Error => ("bg-red-500/10", "border-red-500/30", "text-red-400"),
        NotificationType::Warning => (
            "bg-yellow-500/10",
            "border-yellow-500/30",
            "text-yellow-400",
        ),
        NotificationType::Info => ("bg-blue-500/10", "border-blue-500/30", "text-blue-400"),
    };

    let icon_path = match notification.notification_type {
        NotificationType::Success => "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z",
        NotificationType::Error => "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
        NotificationType::Warning => {
            "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        }
        NotificationType::Info => "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    };

    let title = notification.title.clone();
    let message = notification.message.clone();
    let container_class = format!(
        "flex items-start gap-3 p-4 rounded-lg border backdrop-blur-sm shadow-lg transition-all duration-300 {} {}",
        bg_class, border_class
    );

    view! {
        <Show when=move || is_visible.get()>
            <div
                class=container_class.clone()
                style=move || if is_exiting.get() { "opacity: 0; transform: translateX(1rem);" } else { "opacity: 1; transform: translateX(0);" }
            >
                <div class=icon_class>
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=icon_path />
                    </svg>
                </div>
                <div class="flex-1 min-w-0">
                    <h4 class="text-sm font-medium text-theme-primary">{title.clone()}</h4>
                    <p class="text-xs text-theme-secondary mt-0.5">{message.clone()}</p>
                </div>
                <button
                    class="text-theme-muted hover:text-theme-primary transition-colors"
                    on:click=move |_| {
                        notifications.update(|n| {
                            n.retain(|i| i.id != id);
                        });
                    }
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
        </Show>
    }
}

/// Hook to manage notifications
pub struct NotificationManager {
    notifications: RwSignal<VecDeque<NotificationItem>>,
    next_id: RwSignal<u64>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: RwSignal::new(VecDeque::new()),
            next_id: RwSignal::new(0),
        }
    }

    /// Get the notifications signal for the container
    pub fn notifications(&self) -> RwSignal<VecDeque<NotificationItem>> {
        self.notifications
    }

    /// Add a notification
    pub fn notify(&self, notification: CanvasNotification) {
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        self.notifications.update(|n| {
            n.push_back(NotificationItem { id, notification });

            // Remove oldest if we exceed max
            while n.len() > MAX_NOTIFICATIONS {
                n.pop_front();
            }
        });
    }

    /// Add a success notification
    pub fn success(&self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(CanvasNotification::success(title, message));
    }

    /// Add an error notification
    pub fn error(&self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(CanvasNotification::error(title, message));
    }

    /// Add a warning notification
    pub fn warning(&self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(CanvasNotification::warning(title, message));
    }

    /// Add an info notification
    pub fn info(&self, title: impl Into<String>, message: impl Into<String>) {
        self.notify(CanvasNotification::info(title, message));
    }

    /// Clear all notifications
    pub fn clear(&self) {
        self.notifications.set(VecDeque::new());
    }

    /// Create a callback for use with other components
    pub fn callback(&self) -> Callback<CanvasNotification> {
        let notifications = self.notifications;
        let next_id = self.next_id;

        Callback::new(move |notification: CanvasNotification| {
            let id = next_id.get();
            next_id.set(id + 1);

            notifications.update(|n| {
                n.push_back(NotificationItem { id, notification });

                while n.len() > MAX_NOTIFICATIONS {
                    n.pop_front();
                }
            });
        })
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        Self {
            notifications: self.notifications,
            next_id: self.next_id,
        }
    }
}

impl Copy for NotificationManager {}
