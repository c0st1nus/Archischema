//! Not found page component
//!
//! A 404 error page displayed when a route is not found.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::ui::icon::{Icon, icons};

/// Not found (404) page component
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-theme-primary flex flex-col items-center justify-center p-4">
            <div class="text-center">
                // 404 icon
                <div class="w-24 h-24 mx-auto mb-6 bg-theme-secondary rounded-full flex items-center justify-center">
                    <Icon name=icons::DOCUMENT_TEXT class="w-12 h-12 text-theme-tertiary" />
                </div>

                // Error code
                <h1 class="text-6xl font-bold text-theme-primary mb-4">"404"</h1>

                // Title
                <h2 class="text-2xl font-semibold text-theme-primary mb-2">
                    "Page Not Found"
                </h2>

                // Description
                <p class="text-theme-secondary mb-8 max-w-md mx-auto">
                    "The page you're looking for doesn't exist or has been moved."
                </p>

                // Actions
                <div class="flex flex-col sm:flex-row items-center justify-center gap-4">
                    <A
                        href="/"
                        attr:class="px-6 py-3 bg-accent-primary hover:bg-accent-primary-hover text-white font-medium rounded-lg transition-colors"
                    >
                        "Go Home"
                    </A>
                    <A
                        href="/dashboard"
                        attr:class="px-6 py-3 border border-theme text-theme-primary hover:bg-theme-secondary font-medium rounded-lg transition-colors"
                    >
                        "My Diagrams"
                    </A>
                </div>
            </div>

            // Footer
            <div class="absolute bottom-8 text-center">
                <p class="text-sm text-theme-tertiary">
                    "© 2025 Archischema"
                </p>
            </div>
        </div>
    }
}
