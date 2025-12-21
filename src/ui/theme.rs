//! Theme context module for managing dark/light/automatic theme
//!
//! Provides:
//! - ThemeMode enum (Auto, Dark, Light)
//! - ThemeContext for reactive theme state
//! - System theme detection via prefers-color-scheme
//! - LocalStorage persistence

use std::str::FromStr;

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

    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeMode::Auto => "Automatic",
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
        }
    }
}

impl FromStr for ThemeMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(ThemeMode::Dark),
            "light" => Ok(ThemeMode::Light),
            "auto" => Ok(ThemeMode::Auto),
            _ => Ok(ThemeMode::Auto),
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
    /// Toggle between light and dark mode
    pub fn toggle(&self) {
        let new_mode = if self.is_dark.get() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        self.set_mode(new_mode);
    }

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
                    let _ = storage.set_item("archischema-theme", mode.as_str());
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
                if let Ok(Some(value)) = storage.get_item("archischema-theme") {
                    return value.parse().unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ThemeMode Tests
    // ========================================================================

    #[test]
    fn test_theme_mode_default() {
        let mode = ThemeMode::default();
        assert_eq!(mode, ThemeMode::Auto);
    }

    #[test]
    fn test_theme_mode_as_str() {
        assert_eq!(ThemeMode::Auto.as_str(), "auto");
        assert_eq!(ThemeMode::Dark.as_str(), "dark");
        assert_eq!(ThemeMode::Light.as_str(), "light");
    }

    #[test]
    fn test_theme_mode_from_str() {
        assert_eq!("auto".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("dark".parse::<ThemeMode>().unwrap(), ThemeMode::Dark);
        assert_eq!("light".parse::<ThemeMode>().unwrap(), ThemeMode::Light);
    }

    #[test]
    fn test_theme_mode_from_str_unknown() {
        // Unknown values should default to Auto
        assert_eq!("unknown".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("DARK".parse::<ThemeMode>().unwrap(), ThemeMode::Auto); // Case sensitive
    }

    #[test]
    fn test_theme_mode_display_name() {
        assert_eq!(ThemeMode::Auto.display_name(), "Automatic");
        assert_eq!(ThemeMode::Dark.display_name(), "Dark");
        assert_eq!(ThemeMode::Light.display_name(), "Light");
    }

    #[test]
    fn test_theme_mode_clone() {
        let mode = ThemeMode::Dark;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_theme_mode_copy() {
        let mode = ThemeMode::Light;
        let copied: ThemeMode = mode; // Copy trait
        assert_eq!(mode, copied);
    }

    #[test]
    fn test_theme_mode_debug() {
        let mode = ThemeMode::Auto;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("Auto"));
    }

    #[test]
    fn test_theme_mode_equality() {
        assert_eq!(ThemeMode::Auto, ThemeMode::Auto);
        assert_eq!(ThemeMode::Dark, ThemeMode::Dark);
        assert_eq!(ThemeMode::Light, ThemeMode::Light);

        assert_ne!(ThemeMode::Auto, ThemeMode::Dark);
        assert_ne!(ThemeMode::Dark, ThemeMode::Light);
        assert_ne!(ThemeMode::Light, ThemeMode::Auto);
    }

    #[test]
    fn test_theme_mode_roundtrip() {
        // Test that from_str(as_str()) returns the same mode
        for mode in [ThemeMode::Auto, ThemeMode::Dark, ThemeMode::Light] {
            let str_repr = mode.as_str();
            let parsed: ThemeMode = str_repr.parse().unwrap();
            assert_eq!(mode, parsed);
        }
    }

    // ========================================================================
    // Helper Function Tests (SSR fallbacks)
    // ========================================================================

    #[test]
    fn test_load_persisted_theme_ssr() {
        // In SSR mode (which is the default for tests), this should return Auto
        let theme = load_persisted_theme();
        assert_eq!(theme, ThemeMode::Auto);
    }

    #[test]
    fn test_detect_system_prefers_dark_ssr() {
        // In SSR mode, this should return false
        let prefers_dark = detect_system_prefers_dark();
        assert!(!prefers_dark);
    }

    // ========================================================================
    // ThemeMode Edge Cases
    // ========================================================================

    #[test]
    fn test_theme_mode_from_str_whitespace() {
        // Strings with whitespace should not match
        assert_eq!(" dark".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("dark ".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!(" dark ".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
    }

    #[test]
    fn test_theme_mode_from_str_mixed_case() {
        // Case sensitivity check
        assert_eq!("Dark".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("LIGHT".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
        assert_eq!("AUTO".parse::<ThemeMode>().unwrap(), ThemeMode::Auto);
    }

    // ========================================================================
    // All Theme Modes Iteration Test
    // ========================================================================

    #[test]
    fn test_all_theme_modes_have_unique_str_repr() {
        let modes = [ThemeMode::Auto, ThemeMode::Dark, ThemeMode::Light];
        let strs: Vec<&str> = modes.iter().map(|m| m.as_str()).collect();

        // Check all strings are unique
        for (i, s1) in strs.iter().enumerate() {
            for (j, s2) in strs.iter().enumerate() {
                if i != j {
                    assert_ne!(s1, s2, "Theme mode strings should be unique");
                }
            }
        }
    }

    #[test]
    fn test_all_theme_modes_have_unique_display_names() {
        let modes = [ThemeMode::Auto, ThemeMode::Dark, ThemeMode::Light];
        let names: Vec<&str> = modes.iter().map(|m| m.display_name()).collect();

        // Check all display names are unique
        for (i, n1) in names.iter().enumerate() {
            for (j, n2) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(n1, n2, "Theme mode display names should be unique");
                }
            }
        }
    }
}
