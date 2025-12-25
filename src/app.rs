use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::ui::{
    ActivityTracker, DashboardPage, EditorPage, LandingPage, LoginPage, NotFoundPage, ProfilePage,
    RegisterPage, provide_auth_context, provide_liveshare_context, provide_theme_context,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Provide theme context for dark/light mode management
    let _theme_ctx = provide_theme_context();

    // Provide auth context for authentication state management
    let _auth_ctx = provide_auth_context();

    // Provide LiveShare context for real-time collaboration
    let _liveshare_ctx = provide_liveshare_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/archischema.css"/>

        // sets the document title
        <Title text="Archischema - Database Schema Editor"/>

        // Activity tracker for monitoring user idle status
        <ActivityTracker />

        // Router for page navigation
        // For SSR + hydration, we need to provide the initial URL
        <Router>
            <main class="w-full min-h-screen bg-theme-primary theme-transition">
                <Routes fallback=|| view! { <NotFoundPage /> }>
                    // Landing page (home)
                    <Route path=path!("/") view=LandingPage />

                    // Authentication pages
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/register") view=RegisterPage />

                    // Dashboard (requires auth)
                    <Route path=path!("/dashboard") view=DashboardPage />

                    // Profile page (requires auth)
                    <Route path=path!("/profile") view=ProfilePage />

                    // Editor pages
                    <Route path=path!("/editor/:id") view=EditorPage />
                </Routes>
            </main>
        </Router>
    }
}
