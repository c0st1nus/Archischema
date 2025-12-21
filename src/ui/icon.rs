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
    let full_class = format!("block {}", class);

    view! {
        <img
            src=icon_path
            class=full_class
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
    pub const MENU: &str = "bars-3";
    pub const EXPAND: &str = "expand";
    pub const COLLAPSE: &str = "collapse";
    pub const ALERT_CIRCLE: &str = "alert-circle";
    pub const LOADER: &str = "loader";
    pub const DICES: &str = "dices";
    pub const SETTINGS: &str = "settings";
    pub const GRIP_HORIZONTAL: &str = "grip-horizontal";
    pub const GRIP_VERTICAL: &str = "grip-vertical";
    pub const USER: &str = "user";
    pub const USER_PLUS: &str = "user-plus";
    pub const USER_CHECK: &str = "user-check";
    pub const USER_MINUS: &str = "user-minus";
    pub const SIGNAL_ZERO: &str = "signal-zero";
    pub const SIGNAL_LOW: &str = "signal-low";
    pub const SIGNAL_MEDIUM: &str = "signal-medium";
    pub const SIGNAL_HIGH: &str = "signal-high";
    pub const SIGNAL_BRILLIANT: &str = "signal-brilliant";
    pub const DATABASE: &str = "database";
    pub const DATABASE_BACKUP: &str = "database-backup";
    pub const DATABASE_ZAP: &str = "database-zap";
    pub const JSON: &str = "json";
    pub const FILE: &str = "file";
    pub const BOT: &str = "bot";
    pub const SEND: &str = "send";
    pub const SPARKLES: &str = "sparkles";
    pub const SUN: &str = "sun";
    pub const MOON: &str = "moon";
    pub const FOLDER: &str = "folder";
    pub const SORT_UP: &str = "sort-up";
    pub const SORT_DOWN: &str = "sort-down";
    pub const SORT: &str = "sort";
    pub const ARROW_DOWN_TO_LINE: &str = "arrow-down-to-line";
    pub const CAMERA: &str = "camera";
    pub const CLOCK: &str = "clock";
    pub const CODE: &str = "code";
    pub const DOCUMENT_DUPLICATE: &str = "document-duplicate";
    pub const DOCUMENT_TEXT: &str = "document-text";
    pub const ELLIPSIS: &str = "ellipsis";
    pub const ELLIPSIS_VERTICAL: &str = "ellipsis-vertical";
    pub const ERROR: &str = "error";
    pub const WARNING: &str = "warning";
    pub const EXTERNAL_LINK: &str = "external-link";
    pub const EYE: &str = "eye";
    pub const EYE_CLOSED: &str = "eye-closed";
    pub const GITHUB: &str = "github";
    pub const INFORMATION_CIRCLE: &str = "information-circle";
    pub const LOGOUT: &str = "logout";
    pub const SQUARES_2X2: &str = "squares-2x2";
    pub const ARROW_LEFT: &str = "arrow-left";
    pub const PANEL_LEFT_CLOSE: &str = "panel-left-close";
    pub const PANEL_LEFT_OPEN: &str = "panel-left-open";
}
