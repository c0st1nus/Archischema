//! Login page component
//!
//! A standalone page for user login, redirects to dashboard on success.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::ui::auth::{AuthState, LoginForm, use_auth_context};
use crate::ui::theme::{ThemeMode, use_theme_context};

/// Login page component
#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();

    // Redirect if already authenticated
    Effect::new(move |_| {
        if matches!(auth.state.get(), AuthState::Authenticated(_)) {
            let navigate = use_navigate();
            navigate("/dashboard", Default::default());
        }
    });

    // Handle successful login
    let on_success = move |_| {
        let navigate = use_navigate();
        navigate("/dashboard", Default::default());
    };

    // Switch to register page
    let on_register_click = move |_| {
        let navigate = use_navigate();
        navigate("/register", Default::default());
    };

    view! {
        <div class="min-h-screen bg-theme-primary flex flex-col">
            // Header
            <header class="border-b border-theme">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
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

                        // Theme toggle
                        <button
                            class="p-2 rounded-lg hover:bg-theme-secondary transition-colors text-theme-secondary"
                            on:click=move |_| theme.toggle()
                            title="Toggle theme"
                        >
                            {move || {
                                if theme.mode.get() == ThemeMode::Dark {
                                    view! {
                                        <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                                  d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                                        </svg>
                                    }
                                } else {
                                    view! {
                                        <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                                  d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                                        </svg>
                                    }
                                }
                            }}
                        </button>
                    </div>
                </div>
            </header>

            // Main content
            <main class="flex-1 flex items-center justify-center p-4">
                <div class="w-full max-w-md">
                    <LoginForm
                        on_success=Callback::new(on_success)
                        on_register_click=Callback::new(on_register_click)
                    />
                </div>
            </main>

            // Footer
            <footer class="py-4 border-t border-theme">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <p class="text-center text-sm text-theme-tertiary">
                        "© 2025 Archischema. All rights reserved."
                    </p>
                </div>
            </footer>
        </div>
    }
}
