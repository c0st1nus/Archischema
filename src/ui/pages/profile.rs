//! User profile page component
//!
//! A page for viewing and editing user profile information,
//! account settings, and preferences.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::ui::auth::{AuthState, User, logout, use_auth_context};
use crate::ui::icon::{Icon, icons};
use crate::ui::theme::{ThemeMode, use_theme_context};

/// Profile page component
#[component]
pub fn ProfilePage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();

    // Editing states
    let editing_username = RwSignal::new(false);
    let editing_email = RwSignal::new(false);
    let new_username = RwSignal::new(String::new());
    let new_email = RwSignal::new(String::new());
    let save_error = RwSignal::new(Option::<String>::None);
    let save_success = RwSignal::new(Option::<String>::None);

    // Password change states
    let show_password_modal = RwSignal::new(false);
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let password_error = RwSignal::new(Option::<String>::None);
    let password_loading = RwSignal::new(false);

    // Handle logout
    let handle_logout = move |_| {
        spawn_local(async move {
            logout().await;
            let navigate = use_navigate();
            navigate("/", Default::default());
        });
    };

    // Handle password change
    let handle_password_change = move |_| {
        let current = current_password.get();
        let new_pass = new_password.get();
        let confirm = confirm_password.get();

        // Validation
        if current.is_empty() || new_pass.is_empty() || confirm.is_empty() {
            password_error.set(Some("All fields are required".to_string()));
            return;
        }

        if new_pass != confirm {
            password_error.set(Some("Passwords do not match".to_string()));
            return;
        }

        if new_pass.len() < 8 {
            password_error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }

        password_loading.set(true);
        password_error.set(None);

        spawn_local(async move {
            // Make API call to change password
            #[cfg(not(feature = "ssr"))]
            {
                use gloo_net::http::Request;
                use serde::Serialize;

                #[derive(Serialize)]
                struct ChangePasswordRequest {
                    current_password: String,
                    new_password: String,
                }

                let request = ChangePasswordRequest {
                    current_password: current,
                    new_password: new_pass,
                };

                match Request::post("/api/auth/password")
                    .header("Content-Type", "application/json")
                    .json(&request)
                {
                    Ok(req) => match req.send().await {
                        Ok(response) => {
                            if response.ok() {
                                save_success.set(Some("Password changed successfully".to_string()));
                                show_password_modal.set(false);
                                current_password.set(String::new());
                                new_password.set(String::new());
                                confirm_password.set(String::new());
                            } else {
                                password_error.set(Some("Failed to change password. Please check your current password.".to_string()));
                            }
                        }
                        Err(_) => {
                            password_error
                                .set(Some("Network error. Please try again.".to_string()));
                        }
                    },
                    Err(_) => {
                        password_error.set(Some("Failed to send request".to_string()));
                    }
                }
            }
            password_loading.set(false);
        });
    };

    view! {
        <div class="min-h-screen bg-theme-primary">
            // Header
            <ProfileHeader theme=theme />

            // Main content
            <main class="max-w-4xl mx-auto px-4 py-8">
                {move || {
                    match auth.state.get() {
                        AuthState::Loading => {
                            view! {
                                <div class="flex items-center justify-center py-20">
                                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-accent-primary"></div>
                                </div>
                            }.into_any()
                        }
                        AuthState::Unauthenticated => {
                            view! {
                                <div class="max-w-md mx-auto py-20">
                                    <div class="p-6 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700 rounded-lg">
                                        <div class="flex items-start gap-4">
                                            <div class="flex-shrink-0">
                                                <Icon name=icons::WARNING class="h-6 w-6 text-yellow-500" />
                                            </div>
                                            <div class="flex-1">
                                                <h3 class="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                                                    "Authentication required"
                                                </h3>
                                                <p class="mt-1 text-sm text-yellow-700 dark:text-yellow-300">
                                                    "You need to be logged in to view your profile. Please log in to continue."
                                                </p>
                                                <div class="mt-4">
                                                    <A
                                                        href="/login"
                                                        attr:class="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary-hover rounded-lg transition-colors"
                                                    >
                                                        <Icon name=icons::LOGOUT class="w-4 h-4" />
                                                        "Go to Login"
                                                    </A>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        AuthState::Authenticated(user) => {
                            // Initialize edit fields with current values
                            if new_username.get().is_empty() {
                                new_username.set(user.username.clone());
                            }
                            if new_email.get().is_empty() {
                                new_email.set(user.email.clone());
                            }

                            // Clone user for different closures
                            let user_for_profile = user.clone();
                            let user_for_username_display = user.username.clone();
                            let user_for_username_cancel = user.username.clone();
                            let user_for_email_display = user.email.clone();
                            let user_for_email_cancel = user.email.clone();

                            view! {
                                <div class="space-y-8">
                                    // Success/Error messages
                                    {move || save_success.get().map(|msg| view! {
                                        <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4">
                                            <div class="flex items-center gap-2">
                                                <Icon name=icons::CHECK class="h-5 w-5 text-green-500" />
                                                <p class="text-sm text-green-700 dark:text-green-300">{msg}</p>
                                            </div>
                                        </div>
                                    })}

                                    {move || save_error.get().map(|msg| view! {
                                        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
                                            <div class="flex items-center gap-2">
                                                <Icon name=icons::X class="h-5 w-5 text-red-500" />
                                                <p class="text-sm text-red-700 dark:text-red-300">{msg}</p>
                                            </div>
                                        </div>
                                    })}

                                    // Profile Card
                                    <ProfileCard user=user_for_profile />

                                    // Account Settings Section
                                    <section class="bg-theme-secondary/30 rounded-xl p-6 border border-theme">
                                        <h2 class="text-lg font-semibold text-theme-primary mb-6">"Account Settings"</h2>

                                        <div class="space-y-6">
                                            // Username field
                                            <div class="flex items-center justify-between">
                                                <div class="flex-1">
                                                    <label class="block text-sm font-medium text-theme-secondary mb-1">"Username"</label>
                                                    {
                                                        let username_for_cancel = user_for_username_cancel.clone();
                                                        let username_for_display = user_for_username_display.clone();
                                                        move || {
                                                            let username_cancel = username_for_cancel.clone();
                                                            let username_display = username_for_display.clone();
                                                            if editing_username.get() {
                                                                view! {
                                                                    <div class="flex items-center gap-2">
                                                                        <input
                                                                            type="text"
                                                                            class="flex-1 px-3 py-2 bg-theme-primary border border-theme rounded-lg
                                                                                   text-theme-primary focus:outline-none focus:ring-2 focus:ring-accent-primary"
                                                                            prop:value=move || new_username.get()
                                                                            on:input=move |ev| {
                                                                                new_username.set(event_target_value(&ev));
                                                                            }
                                                                        />
                                                                        <button
                                                                            class="px-3 py-2 text-sm font-medium text-white bg-accent-primary
                                                                                   hover:bg-accent-primary-hover rounded-lg transition-colors"
                                                                            on:click=move |_| {
                                                                                // TODO: Save username via API
                                                                                editing_username.set(false);
                                                                                save_success.set(Some("Username updated".to_string()));
                                                                            }
                                                                        >
                                                                            "Save"
                                                                        </button>
                                                                        <button
                                                                            class="px-3 py-2 text-sm font-medium text-theme-secondary
                                                                                   hover:text-theme-primary transition-colors"
                                                                            on:click={
                                                                                let username = username_cancel.clone();
                                                                                move |_| {
                                                                                    editing_username.set(false);
                                                                                    new_username.set(username.clone());
                                                                                }
                                                                            }
                                                                        >
                                                                            "Cancel"
                                                                        </button>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <div class="flex items-center justify-between">
                                                                        <p class="text-theme-primary">{username_display}</p>
                                                                        <button
                                                                            class="text-sm text-accent-primary hover:text-accent-primary-hover transition-colors"
                                                                            on:click=move |_| editing_username.set(true)
                                                                        >
                                                                            "Edit"
                                                                        </button>
                                                                    </div>
                                                                }.into_any()
                                                            }
                                                        }
                                                    }
                                                </div>
                                            </div>

                                            // Email field
                                            <div class="flex items-center justify-between">
                                                <div class="flex-1">
                                                    <label class="block text-sm font-medium text-theme-secondary mb-1">"Email"</label>
                                                    {
                                                        let email_for_cancel = user_for_email_cancel.clone();
                                                        let email_for_display = user_for_email_display.clone();
                                                        move || {
                                                            let email_cancel = email_for_cancel.clone();
                                                            let email_display = email_for_display.clone();
                                                            if editing_email.get() {
                                                                view! {
                                                                    <div class="flex items-center gap-2">
                                                                        <input
                                                                            type="email"
                                                                            class="flex-1 px-3 py-2 bg-theme-primary border border-theme rounded-lg
                                                                                   text-theme-primary focus:outline-none focus:ring-2 focus:ring-accent-primary"
                                                                            prop:value=move || new_email.get()
                                                                            on:input=move |ev| {
                                                                                new_email.set(event_target_value(&ev));
                                                                            }
                                                                        />
                                                                        <button
                                                                            class="px-3 py-2 text-sm font-medium text-white bg-accent-primary
                                                                                   hover:bg-accent-primary-hover rounded-lg transition-colors"
                                                                            on:click=move |_| {
                                                                                // TODO: Save email via API
                                                                                editing_email.set(false);
                                                                                save_success.set(Some("Email updated".to_string()));
                                                                            }
                                                                        >
                                                                            "Save"
                                                                        </button>
                                                                        <button
                                                                            class="px-3 py-2 text-sm font-medium text-theme-secondary
                                                                                   hover:text-theme-primary transition-colors"
                                                                            on:click={
                                                                                let email = email_cancel.clone();
                                                                                move |_| {
                                                                                    editing_email.set(false);
                                                                                    new_email.set(email.clone());
                                                                                }
                                                                            }
                                                                        >
                                                                            "Cancel"
                                                                        </button>
                                                                    </div>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <div class="flex items-center justify-between">
                                                                        <p class="text-theme-primary">{email_display}</p>
                                                                        <button
                                                                            class="text-sm text-accent-primary hover:text-accent-primary-hover transition-colors"
                                                                            on:click=move |_| editing_email.set(true)
                                                                        >
                                                                            "Edit"
                                                                        </button>
                                                                    </div>
                                                                }.into_any()
                                                            }
                                                        }
                                                    }
                                                </div>
                                            </div>

                                            // Password
                                            <div>
                                                <label class="block text-sm font-medium text-theme-secondary mb-1">"Password"</label>
                                                <div class="flex items-center justify-between">
                                                    <p class="text-theme-primary">"••••••••"</p>
                                                    <button
                                                        class="text-sm text-accent-primary hover:text-accent-primary-hover transition-colors"
                                                        on:click=move |_| show_password_modal.set(true)
                                                    >
                                                        "Change"
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    </section>

                                    // Preferences Section
                                    <section class="bg-theme-secondary/30 rounded-xl p-6 border border-theme">
                                        <h2 class="text-lg font-semibold text-theme-primary mb-6">"Preferences"</h2>

                                        <div class="space-y-6">
                                            // Theme preference
                                            <div class="flex items-center justify-between">
                                                <div>
                                                    <p class="text-sm font-medium text-theme-primary">"Theme"</p>
                                                    <p class="text-xs text-theme-tertiary">"Choose your preferred color scheme"</p>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <button
                                                        class="px-3 py-1.5 text-sm font-medium rounded-lg transition-colors"
                                                        class=("bg-accent-primary text-white", move || theme.mode.get() == ThemeMode::Light)
                                                        class=("bg-theme-secondary text-theme-secondary hover:text-theme-primary", move || theme.mode.get() != ThemeMode::Light)
                                                        on:click=move |_| theme.set_mode(ThemeMode::Light)
                                                    >
                                                        "Light"
                                                    </button>
                                                    <button
                                                        class="px-3 py-1.5 text-sm font-medium rounded-lg transition-colors"
                                                        class=("bg-accent-primary text-white", move || theme.mode.get() == ThemeMode::Dark)
                                                        class=("bg-theme-secondary text-theme-secondary hover:text-theme-primary", move || theme.mode.get() != ThemeMode::Dark)
                                                        on:click=move |_| theme.set_mode(ThemeMode::Dark)
                                                    >
                                                        "Dark"
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    </section>

                                    // Danger Zone
                                    <section class="bg-red-50 dark:bg-red-900/10 rounded-xl p-6 border border-red-200 dark:border-red-800">
                                        <h2 class="text-lg font-semibold text-red-600 dark:text-red-400 mb-4">"Danger Zone"</h2>

                                        <div class="space-y-4">
                                            <div class="flex items-center justify-between">
                                                <div>
                                                    <p class="text-sm font-medium text-theme-primary">"Sign Out"</p>
                                                    <p class="text-xs text-theme-tertiary">"Sign out of your account on this device"</p>
                                                </div>
                                                <button
                                                    class="px-4 py-2 text-sm font-medium text-red-600 border border-red-300
                                                           hover:bg-red-100 dark:hover:bg-red-900/30 rounded-lg transition-colors"
                                                    on:click=handle_logout
                                                >
                                                    "Sign Out"
                                                </button>
                                            </div>

                                            <div class="border-t border-red-200 dark:border-red-800 pt-4">
                                                <div class="flex items-center justify-between">
                                                    <div>
                                                        <p class="text-sm font-medium text-theme-primary">"Delete Account"</p>
                                                        <p class="text-xs text-theme-tertiary">"Permanently delete your account and all data"</p>
                                                    </div>
                                                    <button
                                                        class="px-4 py-2 text-sm font-medium text-white bg-red-600
                                                               hover:bg-red-700 rounded-lg transition-colors"
                                                        disabled=true
                                                        title="Account deletion is not yet available"
                                                    >
                                                        "Delete Account"
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    </section>
                                </div>

                                // Password Change Modal
                                {move || {
                                    if show_password_modal.get() {
                                        Some(view! {
                                            <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                                                <div class="bg-theme-primary rounded-xl shadow-xl max-w-md w-full p-6 border border-theme">
                                                    <div class="flex items-center justify-between mb-6">
                                                        <h3 class="text-lg font-semibold text-theme-primary">"Change Password"</h3>
                                                        <button
                                                            class="p-1 text-theme-tertiary hover:text-theme-primary transition-colors"
                                                            on:click=move |_| {
                                                                show_password_modal.set(false);
                                                                password_error.set(None);
                                                            }
                                                        >
                                                            <Icon name=icons::X class="h-5 w-5" />
                                                        </button>
                                                    </div>

                                                    {move || password_error.get().map(|msg| view! {
                                                        <div class="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                                                            <p class="text-sm text-red-600 dark:text-red-400">{msg}</p>
                                                        </div>
                                                    })}

                                                    <div class="space-y-4">
                                                        <div>
                                                            <label class="block text-sm font-medium text-theme-secondary mb-1">"Current Password"</label>
                                                            <input
                                                                type="password"
                                                                class="w-full px-3 py-2 bg-theme-primary border border-theme rounded-lg
                                                                       text-theme-primary focus:outline-none focus:ring-2 focus:ring-accent-primary"
                                                                prop:value=move || current_password.get()
                                                                on:input=move |ev| current_password.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="block text-sm font-medium text-theme-secondary mb-1">"New Password"</label>
                                                            <input
                                                                type="password"
                                                                class="w-full px-3 py-2 bg-theme-primary border border-theme rounded-lg
                                                                       text-theme-primary focus:outline-none focus:ring-2 focus:ring-accent-primary"
                                                                prop:value=move || new_password.get()
                                                                on:input=move |ev| new_password.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="block text-sm font-medium text-theme-secondary mb-1">"Confirm New Password"</label>
                                                            <input
                                                                type="password"
                                                                class="w-full px-3 py-2 bg-theme-primary border border-theme rounded-lg
                                                                       text-theme-primary focus:outline-none focus:ring-2 focus:ring-accent-primary"
                                                                prop:value=move || confirm_password.get()
                                                                on:input=move |ev| confirm_password.set(event_target_value(&ev))
                                                            />
                                                        </div>
                                                    </div>

                                                    <div class="flex items-center justify-end gap-3 mt-6">
                                                        <button
                                                            class="px-4 py-2 text-sm font-medium text-theme-secondary
                                                                   hover:text-theme-primary transition-colors"
                                                            on:click=move |_| {
                                                                show_password_modal.set(false);
                                                                password_error.set(None);
                                                            }
                                                        >
                                                            "Cancel"
                                                        </button>
                                                        <button
                                                            class="px-4 py-2 text-sm font-medium text-white bg-accent-primary
                                                                   hover:bg-accent-primary-hover rounded-lg transition-colors
                                                                   disabled:opacity-50 disabled:cursor-not-allowed"
                                                            disabled=move || password_loading.get()
                                                            on:click=handle_password_change
                                                        >
                                                            {move || if password_loading.get() { "Saving..." } else { "Change Password" }}
                                                        </button>
                                                    </div>
                                                </div>
                                            </div>
                                        })
                                    } else {
                                        None
                                    }
                                }}
                            }.into_any()
                        }
                    }
                }}
            </main>
        </div>
    }
}

/// Profile header component
#[component]
fn ProfileHeader(theme: crate::ui::theme::ThemeContext) -> impl IntoView {
    view! {
        <header class="border-b border-theme bg-theme-primary">
            <div class="max-w-4xl mx-auto px-4">
                <div class="flex items-center justify-between h-16">
                    // Logo
                    <A href="/" attr:class="flex items-center gap-3 hover:opacity-80 transition-opacity">
                        <div class="w-8 h-8 bg-accent-primary rounded-lg flex items-center justify-center">
                            <svg class="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                      d="M4 7v10c0 2 1 3 3 3h10c2 0 3-1 3-3V7c0-2-1-3-3-3H7C5 4 4 5 4 7z" />
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                      d="M9 12h6M12 9v6" />
                            </svg>
                        </div>
                        <span class="text-xl font-bold text-theme-primary">"Archischema"</span>
                    </A>

                    <div class="flex items-center gap-4">
                        // Back to Dashboard
                        <A
                            href="/dashboard"
                            attr:class="text-sm font-medium text-theme-secondary hover:text-theme-primary transition-colors"
                        >
                            "← Dashboard"
                        </A>

                        // Theme toggle
                        <button
                            class="p-2 rounded-lg hover:bg-theme-secondary transition-colors text-theme-secondary"
                            on:click=move |_| theme.toggle()
                            title="Toggle theme"
                        >
                            {move || {
                                if theme.mode.get() == ThemeMode::Dark {
                                    view! {
                                        <Icon name=icons::SUN class="w-5 h-5" />
                                    }
                                } else {
                                    view! {
                                        <Icon name=icons::MOON class="w-5 h-5" />
                                    }
                                }
                            }}
                        </button>
                    </div>
                </div>
            </div>
        </header>
    }
}

/// Profile card showing user avatar and basic info
#[component]
fn ProfileCard(user: User) -> impl IntoView {
    // Generate avatar color from username
    let hash = user
        .username
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
    let colors = [
        "bg-blue-500",
        "bg-green-500",
        "bg-yellow-500",
        "bg-red-500",
        "bg-purple-500",
        "bg-pink-500",
        "bg-indigo-500",
        "bg-teal-500",
    ];
    let color = colors[(hash as usize) % colors.len()];

    let initials = user
        .username
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    view! {
        <section class="bg-theme-secondary/30 rounded-xl p-6 border border-theme">
            <div class="flex items-center gap-6">
                // Large avatar
                {if let Some(avatar_url) = &user.avatar_url {
                    view! {
                        <img
                            src=avatar_url.clone()
                            alt=format!("{}'s avatar", user.username)
                            class="w-24 h-24 rounded-full object-cover ring-4 ring-theme"
                        />
                    }.into_any()
                } else {
                    view! {
                        <div class=format!("w-24 h-24 rounded-full flex items-center justify-center text-white font-bold text-3xl ring-4 ring-theme {}", color)>
                            {initials}
                        </div>
                    }.into_any()
                }}

                <div class="flex-1">
                    <h1 class="text-2xl font-bold text-theme-primary">{user.username.clone()}</h1>
                    <p class="text-theme-secondary">{user.email.clone()}</p>
                    <p class="text-sm text-theme-tertiary mt-1">
                        "Member since joining Archischema"
                    </p>
                </div>

                // Edit avatar button (placeholder)
                <button
                    class="p-2 text-theme-tertiary hover:text-theme-primary hover:bg-theme-secondary
                           rounded-lg transition-colors"
                    title="Change avatar (coming soon)"
                    disabled=true
                >
                    <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                              d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                              d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                </button>
            </div>
        </section>
    }
}
