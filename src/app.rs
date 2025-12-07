use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

use crate::core::SchemaGraph;
use crate::ui::{provide_liveshare_context, provide_theme_context, SchemaCanvas};

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

    // Provide LiveShare context for real-time collaboration
    let _liveshare_ctx = provide_liveshare_context();

    // Создаем пустой граф для визуализации (пользователь увидит Empty State)
    let graph = RwSignal::new(SchemaGraph::new());

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/archischema.css"/>

        // sets the document title
        <Title text="Diagramix - Database Schema Editor"/>

        // main application content
        <div class="w-full h-screen bg-theme-primary theme-transition">
            <SchemaCanvas graph=graph />
        </div>
    }
}
