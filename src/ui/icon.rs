use leptos::prelude::*;

#[component]
pub fn Icon(
    /// Icon name from the local Lucide-style set.
    name: &'static str,
    /// CSS classes for size/color.
    #[prop(default = "w-5 h-5")]
    class: &'static str,
) -> impl IntoView {
    let full_class = format!("lucide-icon {}", class);

    view! {
        <svg
            class=full_class
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            {icon_paths(name)}
        </svg>
    }
}

fn icon_paths(name: &'static str) -> AnyView {
    match name {
        icons::DATABASE | icons::DATABASE_BACKUP | icons::DATABASE_ZAP => view! {
            <ellipse cx="12" cy="5" rx="8" ry="3" />
            <path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5" />
            <path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
        }.into_any(),
        icons::SEARCH => view! {
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
        }.into_any(),
        icons::PLUS => view! { <path d="M12 5v14M5 12h14" /> }.into_any(),
        icons::MENU => view! { <path d="M4 6h16M4 12h16M4 18h16" /> }.into_any(),
        icons::X | icons::ERROR => view! { <path d="M18 6 6 18M6 6l12 12" /> }.into_any(),
        icons::CHECK | icons::USER_CHECK => view! { <path d="M20 6 9 17l-5-5" /> }.into_any(),
        icons::CHEVRON_DOWN => view! { <path d="m6 9 6 6 6-6" /> }.into_any(),
        icons::CHEVRON_RIGHT => view! { <path d="m9 6 6 6-6 6" /> }.into_any(),
        icons::CHEVRON_LEFT | icons::ARROW_LEFT => view! { <path d="m15 6-6 6 6 6" /> }.into_any(),
        icons::SETTINGS => view! {
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            <circle cx="12" cy="12" r="3" />
        }.into_any(),
        icons::SPARKLES => view! {
            <path d="m12 3 1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z" />
            <path d="M5 3v4" />
            <path d="M3 5h4" />
            <path d="M19 17v4" />
            <path d="M17 19h4" />
        }.into_any(),
        icons::TABLE => view! {
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <path d="M3 9h18M9 21V9" />
        }.into_any(),
        icons::CODE => view! { <path d="m16 18 6-6-6-6M8 6l-6 6 6 6" /> }.into_any(),
        icons::EYE => view! {
            <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7" />
            <circle cx="12" cy="12" r="3" />
        }.into_any(),
        icons::EYE_CLOSED => view! {
            <path d="m2 2 20 20" />
            <path d="M10.6 10.6A2 2 0 0 0 13.4 13.4" />
            <path d="M7.1 7.1C3.7 8.9 2 12 2 12s3.5 7 10 7c1.9 0 3.5-.6 4.9-1.4" />
            <path d="M14.1 5.3C13.4 5.1 12.7 5 12 5c-6.5 0-10 7-10 7s.9 1.8 2.7 3.5" />
        }.into_any(),
        icons::FOLDER => view! { <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /> }.into_any(),
        icons::FOLDER_PLUS => view! {
            <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
            <path d="M12 11v6M9 14h6" />
        }.into_any(),
        icons::FILE | icons::DOCUMENT_TEXT | icons::JSON => view! { <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9zM14 3v6h6" /> }.into_any(),
        icons::DOCUMENT_DUPLICATE => view! {
            <rect x="9" y="9" width="13" height="13" rx="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        }.into_any(),
        icons::USER | icons::USER_PLUS | icons::USER_MINUS => view! {
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
            {if name == icons::USER_PLUS {
                view! { <path d="M19 8v6M16 11h6" /> }.into_any()
            } else if name == icons::USER_MINUS {
                view! { <path d="M16 11h6" /> }.into_any()
            } else {
                ().into_any()
            }}
        }.into_any(),
        icons::USERS => view! {
            <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        }.into_any(),
        icons::SUN => view! {
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
        }.into_any(),
        icons::MOON => view! { <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" /> }.into_any(),
        icons::KEY => view! { <path d="M21 2 13 10M16 7l3 3M11 12a4 4 0 1 1-7 0 4 4 0 0 1 7 0z" /> }.into_any(),
        icons::EDIT => view! { <path d="M12 20h9M16.5 3.5a2.12 2.12 0 1 1 3 3L7 19l-4 1 1-4z" /> }.into_any(),
        icons::TRASH => view! { <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" /> }.into_any(),
        icons::SEND => view! { <path d="m22 2-7 20-4-9-9-4z" /> }.into_any(),
        icons::ARROW_DOWN_TO_LINE => view! { <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" /> }.into_any(),
        icons::CAMERA => view! {
            <path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3z" />
            <circle cx="12" cy="13" r="3" />
        }.into_any(),
        icons::CLOCK => view! {
            <circle cx="12" cy="12" r="9" />
            <path d="M12 8v4l3 3" />
        }.into_any(),
        icons::ELLIPSIS | icons::ELLIPSIS_VERTICAL => {
            if name == icons::ELLIPSIS_VERTICAL {
                view! { <path d="M12 5h.01M12 12h.01M12 19h.01" /> }.into_any()
            } else {
                view! { <path d="M5 12h.01M12 12h.01M19 12h.01" /> }.into_any()
            }
        }
        icons::WARNING => view! {
            <path d="m10.29 3.86 1.71-3 1.71 3L21 18.86 12 21 3 18.86z" transform="translate(0,-1)" />
            <path d="M12 9v4M12 17h.01" />
        }.into_any(),
        icons::ALERT_CIRCLE | icons::INFORMATION_CIRCLE => view! {
            <circle cx="12" cy="12" r="9" />
            {if name == icons::INFORMATION_CIRCLE {
                view! { <path d="M12 16v-4M12 8h.01" /> }.into_any()
            } else {
                view! { <path d="M12 8v4M12 16h.01" /> }.into_any()
            }}
        }.into_any(),
        icons::EXTERNAL_LINK => view! { <path d="M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" /> }.into_any(),
        icons::GLOBE => view! {
            <circle cx="12" cy="12" r="9" />
            <path d="M3 12h18M12 3a13.5 13.5 0 0 1 0 18M12 3a13.5 13.5 0 0 0 0 18" />
        }.into_any(),
        icons::GIT_BRANCH => view! {
            <path d="M6 3v12" />
            <circle cx="6" cy="18" r="3" />
            <circle cx="18" cy="6" r="3" />
            <path d="M18 9a9 9 0 0 1-9 9" />
        }.into_any(),
        icons::LOCK => view! {
            <rect x="4" y="10" width="16" height="11" rx="2" />
            <path d="M8 10V7a4 4 0 0 1 8 0v3" />
        }.into_any(),
        icons::GITHUB => view! { <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.4 5.4 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4M9 18c-4.51 2-5-2-7-2" /> }.into_any(),
        icons::LOGOUT => view! { <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9" /> }.into_any(),
        icons::SQUARES_2X2 => view! {
            <rect x="3" y="3" width="8" height="8" rx="1.5" />
            <rect x="13" y="3" width="8" height="8" rx="1.5" />
            <rect x="3" y="13" width="8" height="8" rx="1.5" />
            <path d="M14 17h7M17.5 14v6" />
        }.into_any(),
        icons::GRID => view! {
            <rect x="3" y="3" width="7" height="7" rx="1.5" />
            <rect x="14" y="3" width="7" height="7" rx="1.5" />
            <rect x="3" y="14" width="7" height="7" rx="1.5" />
            <rect x="14" y="14" width="7" height="7" rx="1.5" />
        }.into_any(),
        icons::LAYOUT => view! {
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M9 4v16M3 10h18" />
        }.into_any(),
        icons::FILTER => view! {
            <path d="M3 5h18l-7 8v5l-4 2v-7z" />
        }.into_any(),
        icons::STAR => view! {
            <path d="m12 3 2.7 5.47 6.03.88-4.36 4.25 1.03 6-5.4-2.84-5.4 2.84 1.03-6-4.36-4.25 6.03-.88z" />
        }.into_any(),
        icons::PANEL_LEFT_CLOSE | icons::PANEL_LEFT_OPEN => view! {
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <path d="M9 3v18" />
            {if name == icons::PANEL_LEFT_CLOSE {
                view! { <path d="m16 15-3-3 3-3" /> }.into_any()
            } else {
                view! { <path d="m14 9 3 3-3 3" /> }.into_any()
            }}
        }.into_any(),
        icons::EXPAND => view! { <path d="M3 9V5a2 2 0 0 1 2-2h4M21 9V5a2 2 0 0 0-2-2h-4M3 15v4a2 2 0 0 0 2 2h4M21 15v4a2 2 0 0 1-2 2h-4" /> }.into_any(),
        icons::COLLAPSE => view! { <path d="M8 3v3a2 2 0 0 1-2 2H3M21 8h-3a2 2 0 0 1-2-2V3M3 16h3a2 2 0 0 1 2 2v3M16 21v-3a2 2 0 0 1 2-2h3" /> }.into_any(),
        icons::SAVE => view! { <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2zM17 21v-8H7v8M7 3v5h8" /> }.into_any(),
        icons::LOADER => view! { <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" /> }.into_any(),
        icons::SORT | icons::SORT_UP | icons::SORT_DOWN => view! { <path d="m3 16 4 4 4-4M7 20V4M21 8l-4-4-4 4M17 4v16" /> }.into_any(),
        icons::GRIP_HORIZONTAL => view! { <path d="M5 9h14M5 15h14" /> }.into_any(),
        icons::GRIP_VERTICAL => view! { <path d="M9 5v14M15 5v14" /> }.into_any(),
        icons::LIGHTNING => view! { <path d="M13 2 3 14h8l-1 8 10-12h-8z" /> }.into_any(),
        icons::DICES => view! {
            <rect x="3" y="3" width="8" height="8" rx="2" />
            <rect x="13" y="13" width="8" height="8" rx="2" />
            <path d="M7 7h.01M17 17h.01M17 13h.01M21 17h.01M17 21h.01" />
        }.into_any(),
        icons::BOT => view! {
            <rect x="3" y="8" width="18" height="12" rx="2" />
            <path d="M12 8V4M8 4h8M8 14h.01M16 14h.01M9 18h6" />
        }.into_any(),
        icons::SIGNAL_ZERO | icons::SIGNAL_LOW | icons::SIGNAL_MEDIUM | icons::SIGNAL_HIGH | icons::SIGNAL_BRILLIANT => view! {
            <path d="M5 12.55a11 11 0 0 1 14 0M2 8.82a15 15 0 0 1 20 0M8.5 16.43a6 6 0 0 1 7 0M12 20h.01" />
        }.into_any(),
        _ => view! { <path d="M13 10V3L4 14h7v7l9-11h-7z" /> }.into_any(),
    }
}

/// Icon names kept stable for the existing UI code.
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
    pub const USERS: &str = "users";
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
    pub const FOLDER_PLUS: &str = "folder-plus";
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
    pub const FILTER: &str = "filter";
    pub const GIT_BRANCH: &str = "git-branch";
    pub const GLOBE: &str = "globe";
    pub const GRID: &str = "grid";
    pub const LAYOUT: &str = "layout";
    pub const LOCK: &str = "lock";
    pub const STAR: &str = "star";
    pub const EYE: &str = "eye";
    pub const EYE_CLOSED: &str = "eye-closed";
    pub const GITHUB: &str = "github";
    pub const INFORMATION_CIRCLE: &str = "information-circle";
    pub const LOGOUT: &str = "logout";
    pub const SQUARES_2X2: &str = "squares-2x2";
    pub const ARROW_LEFT: &str = "arrow-left";
    pub const PANEL_LEFT_CLOSE: &str = "panel-left-close";
    pub const PANEL_LEFT_OPEN: &str = "panel-left-open";
    pub const SAVE: &str = "save";
}
