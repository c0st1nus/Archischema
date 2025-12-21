//! Reusable message components for displaying errors, warnings, success messages, etc.

use crate::ui::{Icon, icons};
use leptos::prelude::*;

/// Error message component
/// Displays an error message with an alert icon
#[component]
pub fn ErrorMessage(
    /// Error signal - shows message when Some, hidden when None
    #[prop(into)]
    error: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || error.get().is_some()>
            <div class="error-message">
                <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                <span>{move || error.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}

/// Success message component
/// Displays a success message with a check icon
#[component]
pub fn SuccessMessage(
    /// Success message signal - shows when Some, hidden when None
    #[prop(into)]
    message: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || message.get().is_some()>
            <div class="success-message">
                <Icon name=icons::CHECK class="icon-text"/>
                <span>{move || message.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}

/// Warning message component
/// Displays a warning message with a warning icon
#[component]
pub fn WarningMessage(
    /// Warning message signal - shows when Some, hidden when None
    #[prop(into)]
    message: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || message.get().is_some()>
            <div class="warning-message">
                <Icon name=icons::WARNING class="icon-text"/>
                <span>{move || message.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}

/// Info message component
/// Displays an info message with an information icon
#[component]
pub fn InfoMessage(
    /// Info message signal - shows when Some, hidden when None
    #[prop(into)]
    message: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || message.get().is_some()>
            <div class="flex items-center text-sm text-blue-500 dark:text-blue-400">
                <Icon name=icons::INFORMATION_CIRCLE class="icon-text"/>
                <span>{move || message.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}

/// Static error message (always visible)
#[component]
pub fn ErrorMessageStatic(
    /// Error message text
    message: String,
) -> impl IntoView {
    view! {
        <div class="error-message">
            <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
            <span>{message}</span>
        </div>
    }
}

/// Static success message (always visible)
#[component]
pub fn SuccessMessageStatic(
    /// Success message text
    message: String,
) -> impl IntoView {
    view! {
        <div class="success-message">
            <Icon name=icons::CHECK class="icon-text"/>
            <span>{message}</span>
        </div>
    }
}

/// Static warning message (always visible)
#[component]
pub fn WarningMessageStatic(
    /// Warning message text
    message: String,
) -> impl IntoView {
    view! {
        <div class="warning-message">
            <Icon name=icons::WARNING class="icon-text"/>
            <span>{message}</span>
        </div>
    }
}
