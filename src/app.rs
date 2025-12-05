use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};

use crate::core::SchemaGraph;
use crate::ui::{SchemaCanvas, provide_liveshare_context};

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

    // Provide LiveShare context for real-time collaboration
    let _liveshare_ctx = provide_liveshare_context();

    // Создаем пустой граф для визуализации (пользователь увидит Empty State)
    let graph = RwSignal::new(SchemaGraph::new());

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/diagramix.css"/>

        // sets the document title
        <Title text="Diagramix - Database Schema Editor"/>

        // main application content
        <div class="w-full h-screen">
            <SchemaCanvas graph=graph />
        </div>
    }
}
