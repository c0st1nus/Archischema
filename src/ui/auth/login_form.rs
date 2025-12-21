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
        let on_success = on_success.clone();

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
        <form on:submit=on_submit class="space-y-6">
            // Header
            <div class="text-center">
                <h2 class="text-2xl font-bold text-theme-primary">
                    "Welcome Back"
                </h2>
                <p class="mt-2 text-sm text-theme-secondary">
                    "Sign in to your account to continue"
                </p>
            </div>

            // Global error message
            {move || {
                auth.error.get().map(|error| {
                    view! {
                        <div class="p-3 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 rounded-lg">
                            <p class="text-sm text-red-700 dark:text-red-300">{error}</p>
                        </div>
                    }
                })
            }}

            // Email field
            <div>
                <label for="email" class="block text-sm font-medium text-theme-primary mb-1">
                    "Email"
                </label>
                <input
                    type="email"
                    id="email"
                    name="email"
                    autocomplete="email"
                    placeholder="you@example.com"
                    class="w-full px-3 py-2 bg-theme-secondary border border-theme rounded-lg
                           text-theme-primary placeholder-theme-tertiary
                           focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent
                           transition-colors"
                    class:border-red-500=move || email_error.get().is_some()
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
                            <p class="mt-1 text-sm text-red-500">{error}</p>
                        }
                    })
                }}
            </div>

            // Password field
            <div>
                <label for="password" class="block text-sm font-medium text-theme-primary mb-1">
                    "Password"
                </label>
                <div class="relative">
                    <input
                        type=move || if show_password.get() { "text" } else { "password" }
                        id="password"
                        name="password"
                        autocomplete="current-password"
                        placeholder="Enter your password"
                        class="w-full px-3 py-2 pr-10 bg-theme-secondary border border-theme rounded-lg
                               text-theme-primary placeholder-theme-tertiary
                               focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent
                               transition-colors"
                        class:border-red-500=move || password_error.get().is_some()
                        prop:value=move || password.get()
                        on:input=move |ev| {
                            password.set(event_target_value(&ev));
                            password_error.set(None);
                        }
                        on:blur=move |_| { validate_password(); }
                    />
                    <button
                        type="button"
                        class="absolute inset-y-0 right-0 pr-3 flex items-center text-theme-tertiary hover:text-theme-secondary"
                        on:click=move |_| show_password.update(|v| *v = !*v)
                    >
                        {move || {
                            if show_password.get() {
                                view! {
                                    <Icon name=icons::EYE_CLOSED class="h-5 w-5" />
                                }.into_any()
                            } else {
                                view! {
                                    <Icon name=icons::EYE class="h-5 w-5" />
                                }.into_any()
                            }
                        }}
                    </button>
                </div>
                {move || {
                    password_error.get().map(|error| {
                        view! {
                            <p class="mt-1 text-sm text-red-500">{error}</p>
                        }
                    })
                }}
            </div>

            // Submit button
            <button
                type="submit"
                class="w-full py-2.5 px-4 bg-accent-primary hover:bg-accent-primary-hover
                       text-white font-medium rounded-lg
                       focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-accent-primary
                       disabled:opacity-50 disabled:cursor-not-allowed
                       transition-colors"
                disabled=move || auth.loading.get()
            >
                {move || {
                    if auth.loading.get() {
                        view! {
                            <span class="flex items-center justify-center">
                                <Icon name=icons::LOADER class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" />
                                "Signing in..."
                            </span>
                        }.into_any()
                    } else {
                        view! { <span class="block">"Sign In"</span> }.into_any()
                    }
                }}
            </button>

            // Register link
            <div class="text-center text-sm text-theme-secondary">
                "Don't have an account? "
                <button
                    type="button"
                    class="text-accent-primary hover:text-accent-primary-hover font-medium"
                    on:click=move |_| {
                        if let Some(callback) = on_register_click.as_ref() {
                            callback.run(());
                        }
                    }
                >
                    "Sign up"
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
                <div class="relative w-full max-w-md bg-theme-primary rounded-xl shadow-xl p-6 border border-theme">
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
            <div class="w-full max-w-md mx-auto bg-theme-primary rounded-xl shadow-lg p-6 border border-theme">
                {form_content}
            </div>
        }.into_any()
    }
}
