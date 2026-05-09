//! Register form component
//!
//! A modal/page component for user registration with email, username, and password.

use leptos::prelude::*;
use leptos::task::spawn_local;

use super::context::{register, use_auth_context};
use crate::ui::icon::{Icon, icons};

/// Register form component
#[component]
pub fn RegisterForm(
    /// Callback when registration is successful
    #[prop(optional, into)]
    on_success: Option<Callback<()>>,
    /// Callback to switch to login form
    #[prop(optional, into)]
    on_login_click: Option<Callback<()>>,
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
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let show_password = RwSignal::new(false);

    // Form validation
    let email_error = RwSignal::new(None::<String>);
    let username_error = RwSignal::new(None::<String>);
    let password_error = RwSignal::new(None::<String>);
    let confirm_error = RwSignal::new(None::<String>);

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

    // Validate username
    let validate_username = move || {
        let value = username.get();
        if value.is_empty() {
            username_error.set(Some("Username is required".to_string()));
            false
        } else if value.len() < 3 {
            username_error.set(Some("Username must be at least 3 characters".to_string()));
            false
        } else if value.len() > 30 {
            username_error.set(Some("Username must be less than 30 characters".to_string()));
            false
        } else if !value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            username_error.set(Some(
                "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
            ));
            false
        } else {
            username_error.set(None);
            true
        }
    };

    // Validate password
    let validate_password = move || {
        let value = password.get();
        if value.is_empty() {
            password_error.set(Some("Password is required".to_string()));
            false
        } else if value.len() < 8 {
            password_error.set(Some("Password must be at least 8 characters".to_string()));
            false
        } else if !value.chars().any(|c| c.is_uppercase()) {
            password_error.set(Some(
                "Password must contain at least one uppercase letter".to_string(),
            ));
            false
        } else if !value.chars().any(|c| c.is_lowercase()) {
            password_error.set(Some(
                "Password must contain at least one lowercase letter".to_string(),
            ));
            false
        } else if !value.chars().any(|c| c.is_numeric()) {
            password_error.set(Some("Password must contain at least one digit".to_string()));
            false
        } else {
            password_error.set(None);
            true
        }
    };

    // Validate confirm password
    let validate_confirm = move || {
        let pass = password.get();
        let confirm = confirm_password.get();
        if confirm.is_empty() {
            confirm_error.set(Some("Please confirm your password".to_string()));
            false
        } else if pass != confirm {
            confirm_error.set(Some("Passwords do not match".to_string()));
            false
        } else {
            confirm_error.set(None);
            true
        }
    };

    // Password strength indicator
    let password_strength = move || {
        let pass = password.get();
        if pass.is_empty() {
            return (0, "");
        }

        let mut score = 0;

        // Length checks
        if pass.len() >= 8 {
            score += 1;
        }
        if pass.len() >= 12 {
            score += 1;
        }

        // Character variety
        if pass.chars().any(|c| c.is_uppercase()) {
            score += 1;
        }
        if pass.chars().any(|c| c.is_lowercase()) {
            score += 1;
        }
        if pass.chars().any(|c| c.is_numeric()) {
            score += 1;
        }
        if pass.chars().any(|c| !c.is_alphanumeric()) {
            score += 1;
        }

        match score {
            0..=2 => (1, "Weak"),
            3..=4 => (2, "Medium"),
            _ => (3, "Strong"),
        }
    };

    // Handle form submission
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        // Clear previous errors
        auth.clear_error();

        // Validate all fields
        let email_valid = validate_email();
        let username_valid = validate_username();
        let password_valid = validate_password();
        let confirm_valid = validate_confirm();

        if !email_valid || !username_valid || !password_valid || !confirm_valid {
            return;
        }

        let email_val = email.get();
        let username_val = username.get();
        let password_val = password.get();
        let on_success = on_success;

        spawn_local(async move {
            match register(&email_val, &username_val, &password_val).await {
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
                    "Create your account"
                </h1>
                <p class="mt-1.5 text-[13px] text-theme-muted">
                    "Start free. No credit card required."
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
                    aria-describedby="register-email-error"
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
                            <p id="register-email-error" class="mt-1 text-xs text-theme-error">{error}</p>
                        }
                    })
                }}
            </div>

            // Username field
            <div>
                <label for="username" class="field-label">
                    "Username"
                </label>
                <input
                    type="text"
                    id="username"
                    name="username"
                    autocomplete="username"
                    spellcheck="false"
                    placeholder="Choose a username"
                    class="input input-lg"
                    class:border-theme-error=move || username_error.get().is_some()
                    aria-invalid=move || username_error.get().is_some()
                    aria-describedby="username-error"
                    prop:value=move || username.get()
                    on:input=move |ev| {
                        username.set(event_target_value(&ev));
                        username_error.set(None);
                    }
                    on:blur=move |_| { validate_username(); }
                />
                {move || {
                    username_error.get().map(|error| {
                        view! {
                            <p id="username-error" class="mt-1 text-xs text-theme-error">{error}</p>
                        }
                    })
                }}
            </div>

            // Password field
            <div>
                <label for="password" class="field-label">
                    "Password"
                </label>
                <div class="relative">
                    <input
                        type=move || if show_password.get() { "text" } else { "password" }
                        id="password"
                        name="password"
                        autocomplete="new-password"
                        placeholder="Create a strong password"
                        class="input input-lg pr-10"
                        class:border-theme-error=move || password_error.get().is_some()
                        aria-invalid=move || password_error.get().is_some()
                        aria-describedby="register-password-error"
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
                // Password strength indicator
                {move || {
                    let (strength, label) = password_strength();
                    if !password.get().is_empty() {
                        let color_class = match strength {
                            1 => "bg-theme-error",
                            2 => "bg-theme-warning",
                            _ => "bg-theme-success",
                        };
                        let text_class = match strength {
                            1 => "text-theme-error",
                            2 => "text-theme-warning",
                            _ => "text-theme-success",
                        };
                        Some(view! {
                            <div class="mt-2">
                                <div class="flex gap-1 mb-1">
                                    <div class={format!("h-1 flex-1 rounded {}", if strength >= 1 { color_class } else { "bg-theme-tertiary" })}></div>
                                    <div class={format!("h-1 flex-1 rounded {}", if strength >= 2 { color_class } else { "bg-theme-tertiary" })}></div>
                                    <div class={format!("h-1 flex-1 rounded {}", if strength >= 3 { color_class } else { "bg-theme-tertiary" })}></div>
                                </div>
                                <p class={format!("text-xs {}", text_class)}>{label}</p>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
                {move || {
                    password_error.get().map(|error| {
                        view! {
                            <p id="register-password-error" class="mt-1 text-xs text-theme-error">{error}</p>
                        }
                    })
                }}
            </div>

            // Confirm password field
            <div>
                <label for="confirm-password" class="field-label">
                    "Confirm password"
                </label>
                <input
                    type=move || if show_password.get() { "text" } else { "password" }
                    id="confirm-password"
                    name="confirm-password"
                    autocomplete="new-password"
                    placeholder="Confirm your password"
                    class="input input-lg"
                    class:border-theme-error=move || confirm_error.get().is_some()
                    aria-invalid=move || confirm_error.get().is_some()
                    aria-describedby="confirm-password-error"
                    prop:value=move || confirm_password.get()
                    on:input=move |ev| {
                        confirm_password.set(event_target_value(&ev));
                        confirm_error.set(None);
                    }
                    on:blur=move |_| { validate_confirm(); }
                />
                {move || {
                    confirm_error.get().map(|error| {
                        view! {
                            <p id="confirm-password-error" class="mt-1 text-xs text-theme-error">{error}</p>
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
                                "Creating account..."
                            </span>
                        }.into_any()
                    } else {
                        view! { <span>"Create account"</span> }.into_any()
                    }
                }}
            </button>

            // Login link
            <div class="text-center text-xs text-theme-muted">
                "Have an account? "
                <button
                    type="button"
                    class="btn-link"
                    on:click=move |_| {
                        if let Some(callback) = on_login_click.as_ref() {
                            callback.run(());
                        }
                    }
                >
                    "Sign in"
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
                <div class="surface-elev relative max-h-[90vh] w-full max-w-md overflow-y-auto p-6 shadow-theme-xl">
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
