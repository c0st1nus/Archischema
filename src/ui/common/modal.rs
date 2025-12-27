use crate::ui::icon::{Icon, icons};
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use leptos::wasm_bindgen::JsCast;

/// Base modal component with consistent structure
#[component]
pub fn BaseModal(
    /// Modal title
    title: String,
    /// Optional subtitle/description
    #[prop(optional)]
    subtitle: Option<String>,
    /// Whether modal is open
    is_open: Signal<bool>,
    /// Callback to close modal
    on_close: Callback<()>,
    /// Modal content
    children: Children,
    /// Maximum width class (default: max-w-2xl)
    #[prop(default = "max-w-2xl")]
    max_width: &'static str,
    /// Whether clicking backdrop closes modal
    #[prop(default = true)]
    close_on_backdrop: bool,
    /// Whether to show close button in header
    #[prop(default = true)]
    show_close_button: bool,
) -> impl IntoView {
    // Close on Escape key
    #[cfg(not(feature = "ssr"))]
    {
        use leptos::ev::keydown;

        let handle_keydown = window_event_listener(keydown, move |ev| {
            if ev.key() == "Escape" && is_open.with_untracked(|v| *v) {
                on_close.run(());
            }
        });

        on_cleanup(move || drop(handle_keydown));
    }

    view! {
        <div
            class=move || {
                if is_open.get() {
                    "fixed inset-0 z-50 flex items-center justify-center backdrop-theme transition-all duration-300"
                } else {
                    "fixed inset-0 z-50 flex items-center justify-center backdrop-theme opacity-0 pointer-events-none transition-all duration-300"
                }
            }
            on:click=move |e| {
                if close_on_backdrop {
                    #[cfg(not(feature = "ssr"))]
                    {
                        if let Some(target) = e.target() {
                            if let Some(element) = target.dyn_ref::<web_sys::Element>() {
                                if element.class_list().contains("backdrop-theme") {
                                    on_close.run(());
                                }
                            }
                        }
                    }
                    #[cfg(feature = "ssr")]
                    {
                        let _ = e;
                    }
                }
            }
        >
            <div class=format!("w-full {} card theme-transition", max_width)>
                // Header
                <div class="card-header">
                    <div>
                        <h3 class="title-lg">{title}</h3>
                        {subtitle.map(|s| view! { <p class="subtitle">{s}</p> })}
                    </div>
                    {show_close_button.then(|| view! {
                        <button
                            class="btn-icon"
                            on:click=move |_| on_close.run(())
                            title="Close"
                            aria-label="Close modal"
                        >
                            <Icon name=icons::X class="icon-standalone"/>
                        </button>
                    })}
                </div>

                // Content
                <div class="p-6">
                    {children()}
                </div>
            </div>
        </div>
    }
}

/// Confirmation dialog modal
#[component]
pub fn ConfirmDialog(
    /// Dialog title
    title: String,
    /// Dialog message
    message: String,
    /// Whether dialog is open
    is_open: Signal<bool>,
    /// Callback when confirmed
    on_confirm: Callback<()>,
    /// Callback when cancelled
    on_cancel: Callback<()>,
    /// Confirm button text
    #[prop(default = "Confirm".to_string())]
    confirm_text: String,
    /// Cancel button text
    #[prop(default = "Cancel".to_string())]
    cancel_text: String,
    /// Whether confirm action is destructive (uses danger button)
    #[prop(default = false)]
    is_destructive: bool,
) -> impl IntoView {
    view! {
        <BaseModal
            title=title
            is_open=is_open
            on_close=Callback::new(move |_| on_cancel.run(()))
            max_width="max-w-md"
        >
            <div class="space-y-4">
                <p class="text-theme-secondary">{message}</p>

                <div class="flex items-center justify-end gap-2 divider-top pt-4">
                    <button
                        class="btn-secondary"
                        on:click=move |_| on_cancel.run(())
                    >
                        {cancel_text.clone()}
                    </button>
                    <button
                        class=if is_destructive { "btn-danger" } else { "btn-primary" }
                        on:click=move |_| {
                            on_confirm.run(());
                            on_cancel.run(());
                        }
                    >
                        {confirm_text.clone()}
                    </button>
                </div>
            </div>
        </BaseModal>
    }
}

/// Simple alert/info modal
#[component]
pub fn AlertDialog(
    /// Dialog title
    title: String,
    /// Dialog message
    message: String,
    /// Whether dialog is open
    is_open: Signal<bool>,
    /// Callback when closed
    on_close: Callback<()>,
    /// Close button text
    #[prop(default = "OK".to_string())]
    button_text: String,
) -> impl IntoView {
    view! {
        <BaseModal
            title=title
            is_open=is_open
            on_close=on_close
            max_width="max-w-md"
        >
            <div class="space-y-4">
                <p class="text-theme-secondary">{message}</p>

                <div class="flex justify-end divider-top pt-4">
                    <button
                        class="btn-primary"
                        on:click=move |_| on_close.run(())
                    >
                        {button_text.clone()}
                    </button>
                </div>
            </div>
        </BaseModal>
    }
}
