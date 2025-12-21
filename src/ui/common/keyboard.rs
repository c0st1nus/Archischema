//! Reusable keyboard shortcuts hint components

use crate::ui::Icon;
use leptos::prelude::*;

/// Single keyboard shortcut hint item
#[derive(Clone, Debug)]
pub struct KeyboardHint {
    /// The key name (e.g., "Enter", "Esc", "Ctrl+S")
    pub key: String,
    /// Description of what the key does (e.g., "to save", "to cancel")
    pub action: String,
}

impl KeyboardHint {
    /// Create a new keyboard hint
    pub fn new(key: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            action: action.into(),
        }
    }
}

/// Keyboard shortcuts hints component
/// Displays a list of keyboard shortcuts with styled key indicators
#[component]
pub fn KeyboardHints(
    /// List of keyboard hints to display
    hints: Vec<KeyboardHint>,
) -> impl IntoView {
    view! {
        <div class="kbd-hint">
            {hints
                .into_iter()
                .map(|hint| {
                    view! {
                        <div class="flex items-center">
                            <kbd class="kbd">{hint.key}</kbd>
                            <span class="ml-1">{hint.action}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Common keyboard hints for save/cancel actions
#[component]
pub fn SaveCancelHints() -> impl IntoView {
    view! {
        <KeyboardHints hints=vec![
            KeyboardHint::new("Enter", "to save"),
            KeyboardHint::new("Esc", "to cancel"),
        ]/>
    }
}

/// Common keyboard hints for submit/cancel actions
#[component]
pub fn SubmitCancelHints() -> impl IntoView {
    view! {
        <KeyboardHints hints=vec![
            KeyboardHint::new("Enter", "to submit"),
            KeyboardHint::new("Esc", "to cancel"),
        ]/>
    }
}

/// Common keyboard hints for create/cancel actions
#[component]
pub fn CreateCancelHints() -> impl IntoView {
    view! {
        <KeyboardHints hints=vec![
            KeyboardHint::new("Enter", "to create"),
            KeyboardHint::new("Esc", "to cancel"),
        ]/>
    }
}

/// Single keyboard key display component
#[component]
pub fn Kbd(
    /// The key name to display
    key: String,
) -> impl IntoView {
    view! {
        <kbd class="kbd">{key}</kbd>
    }
}

/// Keyboard shortcut with icon
#[component]
pub fn KeyboardHintWithIcon(
    /// The key name
    key: String,
    /// Description
    action: String,
    /// Icon name
    icon: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center text-xs text-theme-muted">
            <Icon name=icon class="icon-text" />
            <kbd class="kbd ml-1">{key}</kbd>
            <span class="ml-1">{action}</span>
        </div>
    }
}
