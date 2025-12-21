use leptos::prelude::*;

/// Tooltip position relative to the target element
#[derive(Clone, Copy, PartialEq)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
    TopStart,
    TopEnd,
    BottomStart,
    BottomEnd,
}

impl TooltipPosition {
    fn class(&self) -> &'static str {
        match self {
            TooltipPosition::Top => "tooltip-top",
            TooltipPosition::Bottom => "tooltip-bottom",
            TooltipPosition::Left => "tooltip-left",
            TooltipPosition::Right => "tooltip-right",
            TooltipPosition::TopStart => "tooltip-top-start",
            TooltipPosition::TopEnd => "tooltip-top-end",
            TooltipPosition::BottomStart => "tooltip-bottom-start",
            TooltipPosition::BottomEnd => "tooltip-bottom-end",
        }
    }
}

/// Tooltip trigger behavior
#[derive(Clone, Copy, PartialEq)]
pub enum TooltipTrigger {
    /// Show on hover
    Hover,
    /// Show on click
    Click,
    /// Show on hover or focus (for accessibility)
    HoverFocus,
}

/// Tooltip component that shows helpful text on hover
#[component]
pub fn Tooltip(
    /// The content to show in the tooltip
    text: String,
    /// The element that triggers the tooltip
    children: Children,
    /// Position of the tooltip relative to the trigger
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
    /// Trigger behavior
    #[prop(default = TooltipTrigger::Hover)]
    trigger: TooltipTrigger,
    /// Delay before showing tooltip (ms)
    #[prop(default = 200)]
    delay: u32,
    /// Additional CSS classes for the tooltip
    #[prop(default = String::new())]
    class: String,
    /// Whether tooltip is disabled
    #[prop(default = false)]
    disabled: bool,
) -> impl IntoView {
    let (is_visible, set_is_visible) = signal(false);
    let (should_show, set_should_show) = signal(false);

    // Handle delayed show
    Effect::new(move |_| {
        if should_show.get() && !disabled {
            set_timeout(
                move || set_is_visible.set(true),
                std::time::Duration::from_millis(delay as u64),
            );
        } else {
            set_is_visible.set(false);
        }
    });

    let on_mouse_enter = move |_| {
        if matches!(trigger, TooltipTrigger::Hover | TooltipTrigger::HoverFocus) {
            set_should_show.set(true);
        }
    };

    let on_mouse_leave = move |_| {
        if matches!(trigger, TooltipTrigger::Hover | TooltipTrigger::HoverFocus) {
            set_should_show.set(false);
        }
    };

    let on_focus = move |_| {
        if matches!(trigger, TooltipTrigger::HoverFocus) {
            set_should_show.set(true);
        }
    };

    let on_blur = move |_| {
        if matches!(trigger, TooltipTrigger::HoverFocus) {
            set_should_show.set(false);
        }
    };

    let on_click = move |_| {
        if matches!(trigger, TooltipTrigger::Click) {
            set_should_show.update(|s| *s = !*s);
        }
    };

    let tooltip_class = if class.is_empty() {
        format!("tooltip {}", position.class())
    } else {
        format!("tooltip {} {}", position.class(), class)
    };

    view! {
        <div
            class="tooltip-container"
            on:mouseenter=on_mouse_enter
            on:mouseleave=on_mouse_leave
            on:focus=on_focus
            on:blur=on_blur
            on:click=on_click
        >
            {children()}
            <Show when=move || is_visible.get()>
                <div
                    class=tooltip_class.clone()
                    role="tooltip"
                    aria-live="polite"
                >
                    <div class="tooltip-content">
                        {text.clone()}
                    </div>
                    <div class="tooltip-arrow"></div>
                </div>
            </Show>
        </div>
    }
}

/// Simple tooltip with just text and default settings
#[component]
pub fn SimpleTooltip(
    /// Tooltip text
    text: String,
    /// Element to wrap with tooltip
    children: Children,
) -> impl IntoView {
    view! {
        <Tooltip text=text>
            {children()}
        </Tooltip>
    }
}

/// Tooltip wrapper for icon buttons
#[component]
pub fn TooltipIcon(
    /// Tooltip text
    text: String,
    /// Icon content
    icon: &'static str,
    /// Click handler for the icon
    #[prop(optional)]
    on_click: Option<Callback<()>>,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
) -> impl IntoView {
    view! {
        <Tooltip text=text position=position>
            <button
                class="tooltip-icon-btn"
                on:click=move |_| {
                    if let Some(callback) = on_click {
                        callback.run(());
                    }
                }
            >
                {icon}
            </button>
        </Tooltip>
    }
}

/// Info tooltip with an info icon trigger
#[component]
pub fn InfoTooltip(
    /// Tooltip text
    text: String,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <Tooltip text=text position=position class=class>
            <span class="info-tooltip-icon" title="More information">
                "ℹ️"
            </span>
        </Tooltip>
    }
}

/// Tooltip that wraps text with an underline indicator
#[component]
pub fn TextTooltip(
    /// The text to display with underline
    label: String,
    /// Tooltip content
    tooltip_text: String,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
) -> impl IntoView {
    view! {
        <Tooltip text=tooltip_text position=position>
            <span class="text-with-tooltip">
                {label}
            </span>
        </Tooltip>
    }
}
