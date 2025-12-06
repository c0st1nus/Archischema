//! Theme context module for managing dark/light/automatic theme
//!
//! Provides:
//! - ThemeMode enum (Auto, Dark, Light)
//! - ThemeContext for reactive theme state
//! - System theme detection via prefers-color-scheme
//! - LocalStorage persistence

use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use leptos::web_sys;

/// Theme mode options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Auto,
    Dark,
    Light,
}

impl ThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Auto => "auto",
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dark" => ThemeMode::Dark,
            "light" => ThemeMode::Light,
            _ => ThemeMode::Auto,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeMode::Auto => "Automatic",
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
        }
    }
}

/// Theme context for managing theme state
#[derive(Clone, Copy)]
pub struct ThemeContext {
    /// Current theme mode setting
    pub mode: RwSignal<ThemeMode>,
    /// Whether the current effective theme is dark (considering auto mode)
    pub is_dark: Memo<bool>,
    /// System prefers dark mode
    pub system_prefers_dark: RwSignal<bool>,
}

impl ThemeContext {
    /// Set the theme mode and persist to localStorage
    pub fn set_mode(&self, mode: ThemeMode) {
        self.mode.set(mode);
        self.persist_theme(mode);
        self.apply_theme_class();
    }

    /// Persist theme to localStorage
    fn persist_theme(&self, mode: ThemeMode) {
        #[cfg(not(feature = "ssr"))]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("diagramix-theme", mode.as_str());
                }
            }
        }
        #[cfg(feature = "ssr")]
        {
            let _ = mode;
        }
    }

    /// Apply the dark class to the document element
    pub fn apply_theme_class(&self) {
        #[cfg(not(feature = "ssr"))]
        {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(html) = document.document_element() {
                        let class_list = html.class_list();
                        if self.is_dark.get_untracked() {
                            let _ = class_list.add_1("dark");
                        } else {
                            let _ = class_list.remove_1("dark");
                        }
                    }
                }
            }
        }
    }
}

/// Load theme from localStorage
fn load_persisted_theme() -> ThemeMode {
    #[cfg(not(feature = "ssr"))]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(value)) = storage.get_item("diagramix-theme") {
                    return ThemeMode::from_str(&value);
                }
            }
        }
    }
    ThemeMode::Auto
}

/// Detect system color scheme preference
fn detect_system_prefers_dark() -> bool {
    #[cfg(not(feature = "ssr"))]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(media_query)) = window.match_media("(prefers-color-scheme: dark)") {
                return media_query.matches();
            }
        }
    }
    false
}

/// Provide theme context to the application
pub fn provide_theme_context() -> ThemeContext {
    let initial_mode = load_persisted_theme();
    let initial_system_dark = detect_system_prefers_dark();

    let mode = RwSignal::new(initial_mode);
    let system_prefers_dark = RwSignal::new(initial_system_dark);

    // Compute effective dark mode
    let is_dark = Memo::new(move |_| {
        let current_mode = mode.get();
        match current_mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::Auto => system_prefers_dark.get(),
        }
    });

    let ctx = ThemeContext {
        mode,
        is_dark,
        system_prefers_dark,
    };

    // Listen for system theme changes
    #[cfg(not(feature = "ssr"))]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        Effect::new(move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(media_query)) = window.match_media("(prefers-color-scheme: dark)") {
                    let system_dark_signal = system_prefers_dark;
                    let handler = Closure::<dyn Fn(web_sys::MediaQueryListEvent)>::new(
                        move |e: web_sys::MediaQueryListEvent| {
                            system_dark_signal.set(e.matches());
                        },
                    );

                    let _ = media_query.add_event_listener_with_callback(
                        "change",
                        handler.as_ref().unchecked_ref(),
                    );

                    // Keep the closure alive
                    handler.forget();
                }
            }
        });
    }

    // Apply theme class initially and on changes
    #[cfg(not(feature = "ssr"))]
    {
        let ctx_clone = ctx;
        Effect::new(move |_| {
            // Subscribe to is_dark changes
            let _ = ctx_clone.is_dark.get();
            ctx_clone.apply_theme_class();
        });
    }

    // Provide context
    provide_context(ctx);

    ctx
}

/// Use theme context from anywhere in the component tree
pub fn use_theme_context() -> ThemeContext {
    use_context::<ThemeContext>().expect("ThemeContext should be provided")
}
