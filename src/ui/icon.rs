use leptos::prelude::*;

#[component]
pub fn Icon(
    /// Имя иконки (без расширения .svg)
    name: &'static str,
    /// CSS классы для стилизации
    #[prop(default = "w-5 h-5")]
    class: &'static str,
) -> impl IntoView {
    let icon_path = format!("/icons/{}.svg", name);

    view! {
        <img
            src=icon_path
            class=class
            alt=name
            draggable=false
        />
    }
}

/// Предопределенные иконки для удобства использования
#[allow(dead_code)]
pub mod icons {
    pub const TABLE: &str = "table";
    pub const SEARCH: &str = "search";
    pub const CHEVRON_RIGHT: &str = "chevron-right";
    pub const CHEVRON_DOWN: &str = "chevron-down";
    pub const CHEVRON_LEFT: &str = "chevron-left";
    pub const PLUS: &str = "plus";
    pub const KEY: &str = "key";
    pub const EDIT: &str = "edit";
    pub const TRASH: &str = "trash";
    pub const CHECK: &str = "check";
    pub const X: &str = "x";
    pub const LIGHTNING: &str = "lightning";
    pub const MENU: &str = "menu";
    pub const EXPAND: &str = "expand";
    pub const COLLAPSE: &str = "collapse";
    pub const ALERT_CIRCLE: &str = "alert-circle";
    pub const LOADER: &str = "loader";
}
