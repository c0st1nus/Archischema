//! Login form component
//!
//! A modal/page component for user authentication with email and password.

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::context::{login, use_auth_context};
use crate::ui::icon::{Icon, icons};

/// Login form component
#[component]
pub fn LoginForm(
    /// Callback when login is successful
    #[prop(optional, into)]
    on_success: Option<Callback<()>>,
    /// Callback to switch to register form
    #[prop(optional, into)]
    on_register_click: Option<Callback<()>>,
    /// Whether to show as a modal or inline form
    #[prop(default = false)]
    modal: bool,
    /// Callback to close modal (if modal=true)
    #[prop(optional, into)]
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    let auth = use_auth_context();

    // Form state
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let show_password = RwSignal::new(false);

    // Form validation
    let email_error = RwSignal::new(None::<String>);
    let password_error = RwSignal::new(None::<String>);

    // Validate email
    let validate_email = move || {
        let value = email.get();
        if value.is_empty() {
            email_error.set(Some("Email is required".to_string()));
            false
        } else if !value.contains('@') || !value.contains('.') {
            email_error.set(Some("Please enter a valid email".to_string()));
            false
        } else {
            email_error.set(None);
            true
        }
    };

    // Validate password
    let validate_password = move || {
        let value = password.get();
        if value.is_empty() {
            password_error.set(Some("Password is required".to_string()));
            false
        } else {
            password_error.set(None);
            true
        }
    };

    // Handle form submission
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        // Clear previous errors
        auth.clear_error();

        // Validate all fields
        let email_valid = validate_email();
        let password_valid = validate_password();

        if !email_valid || !password_valid {
            return;
        }

        let email_val = email.get();
        let password_val = password.get();
        let on_success = on_success;

        spawn_local(async move {
            match login(&email_val, &password_val).await {
                Ok(_) => {
                    if let Some(callback) = on_success {
                        callback.run(());
                    }
                }
                Err(_) => {
                    // Error is already set in auth context
                }
            }
        });
    };

    let form_content = view! {
        <form on:submit=on_submit class="space-y-[18px]" aria-busy=move || auth.loading.get()>
            <div class="mt-3">
                <h1 class="m-0 text-[26px] font-semibold tracking-[-0.02em] text-theme-primary">
                    "Welcome back"
                </h1>
                <p class="mt-1.5 text-[13px] text-theme-muted">
                    "Sign in to keep modeling."
                </p>
            </div>

            // Global error message
            {move || {
                auth.error.get().map(|error| {
                    view! {
                        <div class="rounded-lg border border-theme-error bg-theme-error p-3">
                            <p class="text-sm text-theme-error">{error}</p>
                        </div>
                    }
                })
            }}

            // Email field
            <div>
                <label for="email" class="field-label">
                    "Email"
                </label>
                <input
                    type="email"
                    id="email"
                    name="email"
                    autocomplete="email"
                    spellcheck="false"
                    placeholder="you@example.com"
                    class="input input-lg"
                    class:border-theme-error=move || email_error.get().is_some()
                    aria-invalid=move || email_error.get().is_some()
                    aria-describedby="email-error"
                    prop:value=move || email.get()
                    on:input=move |ev| {
                        email.set(event_target_value(&ev));
                        email_error.set(None);
                    }
                    on:blur=move |_| { validate_email(); }
                />
                {move || {
                    email_error.get().map(|error| {
                        view! {
                            <p id="email-error" class="mt-1 text-xs text-theme-error">{error}</p>
                        }
                    })
                }}
            </div>

            // Password field
            <div>
                <div class="mb-1 flex items-center justify-between">
                    <label for="password" class="field-label mb-0">
                        "Password"
                    </label>
                    <span class="text-[11.5px] text-theme-muted">"Password reset soon"</span>
                </div>
                <div class="relative">
                    <input
                        type=move || if show_password.get() { "text" } else { "password" }
                        id="password"
                        name="password"
                        autocomplete="current-password"
                        placeholder="Enter your password"
                        class="input input-lg pr-10"
                        class:border-theme-error=move || password_error.get().is_some()
                        aria-invalid=move || password_error.get().is_some()
                        aria-describedby="password-error"
                        prop:value=move || password.get()
                        on:input=move |ev| {
                            password.set(event_target_value(&ev));
                            password_error.set(None);
                        }
                        on:blur=move |_| { validate_password(); }
                    />
                    <button
                        type="button"
                        class="btn-icon absolute right-1 top-1/2 -translate-y-1/2"
                        on:click=move |_| show_password.update(|v| *v = !*v)
                        aria-label=move || if show_password.get() { "Hide password" } else { "Show password" }
                    >
                        {move || {
                            if show_password.get() {
                                view! {
                                    <Icon name=icons::EYE_CLOSED class="h-4 w-4" />
                                }.into_any()
                            } else {
                                view! {
                                    <Icon name=icons::EYE class="h-4 w-4" />
                                }.into_any()
                            }
                        }}
                    </button>
                </div>
                {move || {
                    password_error.get().map(|error| {
                        view! {
                            <p id="password-error" class="mt-1 text-xs text-theme-error">{error}</p>
                        }
                    })
                }}
            </div>

            // Submit button
            <button
                type="submit"
                class="btn btn-primary btn-lg w-full"
                disabled=move || auth.loading.get()
            >
                {move || {
                    if auth.loading.get() {
                        view! {
                            <span class="flex items-center justify-center gap-2">
                                <Icon name=icons::LOADER class="icon-spin h-4 w-4" />
                                "Signing in..."
                            </span>
                        }.into_any()
                    } else {
                        view! { <span>"Sign in to ArchiSchema"</span> }.into_any()
                    }
                }}
            </button>

            // Register link
            <div class="text-center text-xs text-theme-muted">
                "New here? "
                <button
                    type="button"
                    class="btn-link"
                    on:click=move |_| {
                        if let Some(callback) = on_register_click.as_ref() {
                            callback.run(());
                        }
                    }
                >
                    "Create an account"
                </button>
            </div>
        </form>
    };

    if modal {
        view! {
            <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
                // Backdrop
                <div
                    class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                    on:click=move |_| {
                        if let Some(callback) = on_close.as_ref() {
                            callback.run(());
                        }
                    }
                ></div>

                // Modal content
                <div class="surface-elev relative w-full max-w-md p-6 shadow-theme-xl">
                    // Close button
                    <button
                        type="button"
                        class="absolute top-4 right-4 text-theme-tertiary hover:text-theme-secondary"
                        on:click=move |_| {
                            if let Some(callback) = on_close.as_ref() {
                                callback.run(());
                            }
                        }
                    >
                        <Icon name=icons::X class="h-5 w-5" />
                    </button>

                    {form_content}
                </div>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="w-full">
                {form_content}
            </div>
        }
        .into_any()
    }
}
