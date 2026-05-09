//! Register page component
//!
//! A standalone page for user registration, redirects to dashboard on success.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::ui::auth::{AuthState, RegisterForm, use_auth_context};
use crate::ui::common::{AuthVisualPane, BrandMark};
use crate::ui::icon::{Icon, icons};
use crate::ui::theme::{ThemeMode, use_theme_context};

/// Register page component
#[component]
pub fn RegisterPage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();

    // Redirect if already authenticated
    Effect::new(move |_| {
        if matches!(auth.state.get(), AuthState::Authenticated(_)) {
            let navigate = use_navigate();
            navigate("/dashboard", Default::default());
        }
    });

    // Handle successful registration
    let on_success = move |_| {
        let navigate = use_navigate();
        navigate("/dashboard", Default::default());
    };

    // Switch to login page
    let on_login_click = move |_| {
        let navigate = use_navigate();
        navigate("/login", Default::default());
    };

    view! {
        <div class="auth-shell">
            <main class="auth-form-pane relative">
                <div class="flex items-center justify-between">
                    <A href="/" attr:class="brand-row hover:opacity-85 theme-transition">
                        <BrandMark with_label=true />
                    </A>
                    <button
                        type="button"
                        class="btn-icon"
                        on:click=move |_| theme.toggle()
                        aria-label="Toggle theme"
                    >
                        {move || if theme.mode.get() == ThemeMode::Dark {
                            view! { <Icon name=icons::SUN class="h-4 w-4" /> }.into_any()
                        } else {
                            view! { <Icon name=icons::MOON class="h-4 w-4" /> }.into_any()
                        }}
                    </button>
                </div>

                <RegisterForm
                    on_success=Callback::new(on_success)
                    on_login_click=Callback::new(on_login_click)
                />
            </main>

            <AuthVisualPane />
        </div>
    }
}
