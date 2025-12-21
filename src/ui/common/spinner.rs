use leptos::prelude::*;

/// Spinner style variants
#[derive(Clone, Copy, PartialEq)]
pub enum SpinnerStyle {
    /// Classic circular spinner
    Circle,
    /// Dots animation
    Dots,
    /// Pulsing circle
    Pulse,
    /// Bar loader
    Bar,
    /// Ring/border spinner
    Ring,
}

impl SpinnerStyle {
    fn class(&self) -> &'static str {
        match self {
            SpinnerStyle::Circle => "spinner-circle",
            SpinnerStyle::Dots => "spinner-dots",
            SpinnerStyle::Pulse => "spinner-pulse",
            SpinnerStyle::Bar => "spinner-bar",
            SpinnerStyle::Ring => "spinner-ring",
        }
    }
}

/// Spinner size options
#[derive(Clone, Copy, PartialEq)]
pub enum SpinnerSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl SpinnerSize {
    fn class(&self) -> &'static str {
        match self {
            SpinnerSize::Small => "spinner-sm",
            SpinnerSize::Medium => "spinner-md",
            SpinnerSize::Large => "spinner-lg",
            SpinnerSize::ExtraLarge => "spinner-xl",
        }
    }
}

/// Loading spinner component
#[component]
pub fn Spinner(
    /// Spinner style
    #[prop(default = SpinnerStyle::Circle)]
    style: SpinnerStyle,
    /// Spinner size
    #[prop(default = SpinnerSize::Medium)]
    size: SpinnerSize,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
    /// Optional label text
    #[prop(default = String::new())]
    label: String,
    /// Whether to center the spinner
    #[prop(default = false)]
    centered: bool,
) -> impl IntoView {
    let base_classes = format!("spinner {} {}", style.class(), size.class());

    let full_classes = if class.is_empty() {
        base_classes
    } else {
        format!("{} {}", base_classes, class)
    };

    let container_class = if centered {
        "spinner-container spinner-centered"
    } else {
        "spinner-container"
    };

    view! {
        <div class=container_class>
            <div class=full_classes role="status" aria-live="polite">
                {match style {
                    SpinnerStyle::Circle => view! {
                        <div class="spinner-circle-inner"></div>
                    }.into_any(),
                    SpinnerStyle::Dots => view! {
                        <div class="spinner-dots-container">
                            <div class="spinner-dot"></div>
                            <div class="spinner-dot"></div>
                            <div class="spinner-dot"></div>
                        </div>
                    }.into_any(),
                    SpinnerStyle::Pulse => view! {
                        <div class="spinner-pulse-inner"></div>
                    }.into_any(),
                    SpinnerStyle::Bar => view! {
                        <div class="spinner-bar-track">
                            <div class="spinner-bar-fill"></div>
                        </div>
                    }.into_any(),
                    SpinnerStyle::Ring => view! {
                        <div class="spinner-ring-inner"></div>
                    }.into_any(),
                }}
                <span class="sr-only">"Loading..."</span>
            </div>
            {(!label.is_empty()).then(|| view! {
                <div class="spinner-label">{label.clone()}</div>
            })}
        </div>
    }
}

/// Simple loading spinner with default settings
#[component]
pub fn LoadingSpinner(
    /// Optional loading message
    #[prop(default = String::new())]
    message: String,
) -> impl IntoView {
    view! {
        <Spinner
            style=SpinnerStyle::Circle
            size=SpinnerSize::Medium
            label=message
            centered=true
        />
    }
}

/// Inline spinner for buttons or text
#[component]
pub fn InlineSpinner(
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <Spinner
            style=SpinnerStyle::Circle
            size=SpinnerSize::Small
            class=format!("spinner-inline {}", class)
        />
    }
}

/// Full-page loading overlay
#[component]
pub fn LoadingOverlay(
    /// Whether overlay is visible
    visible: ReadSignal<bool>,
    /// Loading message
    #[prop(default = "Loading...".to_string())]
    message: String,
    /// Spinner style
    #[prop(default = SpinnerStyle::Circle)]
    style: SpinnerStyle,
    /// Background opacity (0.0 to 1.0)
    #[prop(default = 0.8)]
    opacity: f32,
) -> impl IntoView {
    view! {
        <Show when=move || visible.get()>
            <div
                class="loading-overlay"
                style=format!("background-color: rgba(0, 0, 0, {})", opacity)
            >
                <div class="loading-overlay-content">
                    <Spinner
                        style=style
                        size=SpinnerSize::Large
                        label=message.clone()
                    />
                </div>
            </div>
        </Show>
    }
}

/// Skeleton loader for content placeholders
#[component]
pub fn Skeleton(
    /// Width of the skeleton
    #[prop(default = "100%".to_string())]
    width: String,
    /// Height of the skeleton
    #[prop(default = "1rem".to_string())]
    height: String,
    /// Whether skeleton is circular
    #[prop(default = false)]
    circle: bool,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let shape_class = if circle {
        "skeleton-circle"
    } else {
        "skeleton-rect"
    };

    let full_classes = if class.is_empty() {
        format!("skeleton {}", shape_class)
    } else {
        format!("skeleton {} {}", shape_class, class)
    };

    let style = format!("width: {}; height: {}", width, height);

    view! {
        <div class=full_classes style=style aria-busy="true">
            <div class="skeleton-shimmer"></div>
        </div>
    }
}

/// Group of skeleton loaders for complex layouts
#[component]
pub fn SkeletonGroup(
    /// Number of skeleton lines
    #[prop(default = 3)]
    lines: usize,
    /// Spacing between lines
    #[prop(default = "0.5rem".to_string())]
    spacing: String,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <div class=format!("skeleton-group {}", class) style=format!("gap: {}", spacing)>
            {(0..lines).map(|i| {
                let width = if i == lines - 1 {
                    "70%".to_string()
                } else {
                    "100%".to_string()
                };

                view! {
                    <Skeleton width=width height="1rem".to_string() />
                }
            }).collect_view()}
        </div>
    }
}

/// Loading button state
#[component]
pub fn LoadingButton(
    /// Whether button is in loading state
    loading: ReadSignal<bool>,
    /// Button text when not loading
    text: String,
    /// Button text when loading
    #[prop(default = "Loading...".to_string())]
    loading_text: String,
    /// Click handler (disabled when loading)
    on_click: Callback<()>,
    /// Whether button is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let is_disabled = move || disabled || loading.get();

    let full_classes = if class.is_empty() {
        "btn-base btn-primary".to_string()
    } else {
        format!("btn-base btn-primary {}", class)
    };

    view! {
        <button
            class=full_classes
            on:click=move |_| {
                if !is_disabled() {
                    on_click.run(())
                }
            }
            disabled=is_disabled
        >
            <Show
                when=move || loading.get()
                fallback=move || view! { <span>{text.clone()}</span> }
            >
                <div class="flex items-center gap-2">
                    <InlineSpinner />
                    <span>{loading_text.clone()}</span>
                </div>
            </Show>
        </button>
    }
}

/// Suspense-like loading wrapper
#[component]
pub fn LoadingWrapper(
    /// Whether content is loading
    loading: ReadSignal<bool>,
    /// Content to show when loaded
    children: ChildrenFn,
    /// Loading message
    #[prop(default = "Loading...".to_string())]
    message: String,
) -> impl IntoView {
    move || {
        if loading.get() {
            view! {
                <LoadingSpinner message=message.clone() />
            }
            .into_any()
        } else {
            children().into_any()
        }
    }
}
