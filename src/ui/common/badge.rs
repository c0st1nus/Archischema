use leptos::prelude::*;

/// Badge variant types for different use cases
#[derive(Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    /// Default neutral badge
    Default,
    /// Primary color badge
    Primary,
    /// Success/positive badge (green)
    Success,
    /// Warning badge (yellow/orange)
    Warning,
    /// Danger/error badge (red)
    Danger,
    /// Info badge (blue)
    Info,
    /// Outline variant
    Outline,
}

impl BadgeVariant {
    fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Default => "badge-default",
            BadgeVariant::Primary => "badge-primary",
            BadgeVariant::Success => "badge-success",
            BadgeVariant::Warning => "badge-warning",
            BadgeVariant::Danger => "badge-danger",
            BadgeVariant::Info => "badge-info",
            BadgeVariant::Outline => "badge-outline",
        }
    }
}

/// Badge size options
#[derive(Clone, Copy, PartialEq)]
pub enum BadgeSize {
    Small,
    Medium,
    Large,
}

impl BadgeSize {
    fn class(&self) -> &'static str {
        match self {
            BadgeSize::Small => "badge-sm",
            BadgeSize::Medium => "badge-md",
            BadgeSize::Large => "badge-lg",
        }
    }
}

/// Badge component for displaying labels, counts, and status indicators
#[component]
pub fn Badge(
    /// Badge content (text or number)
    children: Children,
    /// Visual variant
    #[prop(default = BadgeVariant::Default)]
    variant: BadgeVariant,
    /// Size of the badge
    #[prop(default = BadgeSize::Medium)]
    size: BadgeSize,
    /// Optional icon to show before text
    #[prop(optional)]
    icon: Option<&'static str>,
    /// Whether badge is rounded/pill-shaped
    #[prop(default = false)]
    rounded: bool,
    /// Optional click handler (makes badge interactive)
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
    /// Optional title/tooltip
    #[prop(optional)]
    title: Option<String>,
) -> impl IntoView {
    let base_classes = format!("badge {} {}", variant.class(), size.class());

    let shape_class = if rounded { " badge-rounded" } else { "" };

    let full_classes = if class.is_empty() {
        format!("{}{}", base_classes, shape_class)
    } else {
        format!("{}{} {}", base_classes, shape_class, class)
    };

    let is_interactive = on_click.is_some();
    let interactive_class = if is_interactive {
        format!("{} badge-interactive", full_classes)
    } else {
        full_classes
    };

    let content_icon = icon;
    let content = move || {
        view! {
            {content_icon.map(|icon_name| view! {
                <span class="badge-icon">{icon_name}</span>
            })}
            <span class="badge-content">
                {children()}
            </span>
        }
    };

    if let Some(callback) = on_click {
        let interactive_class = interactive_class.clone();
        view! {
            <button
                class=interactive_class
                on:click=move |_| callback.run(())
                title=title
            >
                {content()}
            </button>
        }
        .into_any()
    } else {
        view! {
            <span
                class=interactive_class
                title=title
            >
                {content()}
            </span>
        }
        .into_any()
    }
}

/// Count badge for displaying numbers
#[component]
pub fn CountBadge(
    /// The count to display
    count: ReadSignal<usize>,
    /// Maximum count to display (shows "99+" if exceeded)
    #[prop(default = 99)]
    max: usize,
    /// Visual variant
    #[prop(default = BadgeVariant::Primary)]
    variant: BadgeVariant,
    /// Whether to hide badge when count is 0
    #[prop(default = true)]
    hide_zero: bool,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let display_text = move || {
        let c = count.get();
        if c > max {
            format!("{}+", max)
        } else {
            c.to_string()
        }
    };

    let should_show = move || !hide_zero || count.get() > 0;

    view! {
        <Show when=should_show>
            <Badge variant=variant size=BadgeSize::Small class=class.clone()>
                {display_text}
            </Badge>
        </Show>
    }
}

/// Status badge for boolean states
#[component]
pub fn StatusBadge(
    /// Whether status is active/true
    active: ReadSignal<bool>,
    /// Text to show when active
    #[prop(default = "Active".to_string())]
    active_text: String,
    /// Text to show when inactive
    #[prop(default = "Inactive".to_string())]
    inactive_text: String,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let variant = move || {
        if active.get() {
            BadgeVariant::Success
        } else {
            BadgeVariant::Default
        }
    };

    let text = move || {
        if active.get() {
            active_text.clone()
        } else {
            inactive_text.clone()
        }
    };

    view! {
        <Badge variant=variant() size=BadgeSize::Small class=class>
            {text}
        </Badge>
    }
}

/// Dot badge - small colored indicator
#[component]
pub fn DotBadge(
    /// Visual variant
    #[prop(default = BadgeVariant::Primary)]
    variant: BadgeVariant,
    /// Optional label text
    #[prop(optional)]
    label: Option<String>,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let full_classes = if class.is_empty() {
        format!("badge-dot {}", variant.class())
    } else {
        format!("badge-dot {} {}", variant.class(), class)
    };

    view! {
        <span class=full_classes>
            <span class="badge-dot-indicator"></span>
            {label.map(|text| view! {
                <span class="badge-dot-label">{text}</span>
            })}
        </span>
    }
}

/// Badge group container for multiple badges
#[component]
pub fn BadgeGroup(
    /// Badge elements
    children: Children,
    /// Spacing between badges
    #[prop(default = "gap-2")]
    spacing: &'static str,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let full_classes = if class.is_empty() {
        format!("badge-group flex items-center flex-wrap {}", spacing)
    } else {
        format!(
            "badge-group flex items-center flex-wrap {} {}",
            spacing, class
        )
    };

    view! {
        <div class=full_classes>
            {children()}
        </div>
    }
}

/// Removable badge with close button
#[component]
pub fn RemovableBadge(
    /// Badge content
    children: Children,
    /// Callback when badge is removed
    on_remove: Callback<()>,
    /// Visual variant
    #[prop(default = BadgeVariant::Default)]
    variant: BadgeVariant,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <Badge variant=variant class=format!("badge-removable {}", class)>
            <span class="badge-removable-content">
                {children()}
            </span>
            <button
                class="badge-remove-btn"
                on:click=move |_| on_remove.run(())
                aria-label="Remove"
                title="Remove"
            >
                "×"
            </button>
        </Badge>
    }
}

/// Badge for displaying keyboard shortcuts
#[component]
pub fn KeyboardBadge(
    /// Key or key combination to display
    keys: String,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let full_classes = if class.is_empty() {
        "badge-keyboard".to_string()
    } else {
        format!("badge-keyboard {}", class)
    };

    view! {
        <kbd class=full_classes>
            {keys}
        </kbd>
    }
}
