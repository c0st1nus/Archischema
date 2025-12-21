use crate::ui::icon::Icon;
use leptos::prelude::*;

/// Button variant types
#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Success,
    Warning,
    Ghost,
    Link,
    Icon,
}

/// Button size options
#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "btn-primary",
            ButtonVariant::Secondary => "btn-secondary",
            ButtonVariant::Danger => "btn-danger",
            ButtonVariant::Success => "btn-success",
            ButtonVariant::Warning => "btn-warning",
            ButtonVariant::Ghost => "btn-ghost",
            ButtonVariant::Link => "btn-link",
            ButtonVariant::Icon => "btn-icon",
        }
    }
}

impl ButtonSize {
    fn class(&self) -> &'static str {
        match self {
            ButtonSize::Small => "btn-sm",
            ButtonSize::Medium => "",
            ButtonSize::Large => "btn-lg",
        }
    }
}

/// Type-safe button component with variants and sizes
#[component]
pub fn Button(
    /// Button variant style
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    /// Button size
    #[prop(default = ButtonSize::Medium)]
    size: ButtonSize,
    /// Click handler
    on_click: Callback<()>,
    /// Whether button is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Whether button is in loading state
    #[prop(default = false)]
    loading: bool,
    /// Optional title/tooltip
    #[prop(optional)]
    title: Option<String>,
    /// Button content (text or elements)
    children: Children,
    /// Optional icon name to show before text
    #[prop(optional)]
    icon: Option<&'static str>,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let base_classes = format!("btn-base {} {}", variant.class(), size.class());
    let full_classes = if class.is_empty() {
        base_classes
    } else {
        format!("{} {}", base_classes, class)
    };

    let is_disabled = disabled || loading;

    view! {
        <button
            class=full_classes
            on:click=move |_| {
                if !loading {
                    on_click.run(())
                }
            }
            disabled=is_disabled
            title=title
        >
            {move || if loading {
                view! {
                    <span class="btn-spinner">
                        <Icon name="spinner" class="icon-spin"/>
                    </span>
                }.into_any()
            } else if let Some(icon_name) = icon {
                view! {
                    <Icon name=icon_name class="icon-btn"/>
                }.into_any()
            } else {
                ().into_any()
            }}
            {children()}
        </button>
    }
}

/// Icon-only button component
#[component]
pub fn IconButton(
    /// Icon name to display
    icon: &'static str,
    /// Click handler
    on_click: Callback<()>,
    /// Whether button is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Optional title/tooltip (recommended for accessibility)
    #[prop(optional)]
    title: Option<String>,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let full_classes = if class.is_empty() {
        "btn-icon".to_string()
    } else {
        format!("btn-icon {}", class)
    };

    let aria_label = title.clone();

    view! {
        <button
            class=full_classes
            on:click=move |_| on_click.run(())
            disabled=disabled
            title=title
            aria-label=aria_label
        >
            <Icon name=icon class="icon-standalone"/>
        </button>
    }
}

/// Button group container for multiple buttons
#[component]
pub fn ButtonGroup(
    /// Button elements
    children: Children,
    /// Spacing between buttons
    #[prop(default = "space-x-2")]
    spacing: &'static str,
) -> impl IntoView {
    view! {
        <div class=format!("flex items-center {}", spacing)>
            {children()}
        </div>
    }
}

/// Submit/Cancel button pair with keyboard hints
#[component]
pub fn SubmitCancelButtons(
    /// Submit button text
    #[prop(default = "Save".to_string())]
    submit_text: String,
    /// Cancel button text
    #[prop(default = "Cancel".to_string())]
    cancel_text: String,
    /// Submit click handler
    on_submit: Callback<()>,
    /// Cancel click handler
    on_cancel: Callback<()>,
    /// Whether submit is disabled
    #[prop(default = false)]
    submit_disabled: bool,
    /// Whether to show keyboard hints
    #[prop(default = true)]
    show_hints: bool,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between">
            <Button
                variant=ButtonVariant::Secondary
                on_click=on_cancel
            >
                {cancel_text}
            </Button>
            <div class="flex items-center gap-3">
                {show_hints.then(|| view! {
                    <div class="text-xs text-theme-muted hidden sm:flex items-center gap-2">
                        <div class="flex items-center gap-1">
                            <kbd class="kbd">"Enter"</kbd>
                            <span>"to save"</span>
                        </div>
                        <div class="flex items-center gap-1">
                            <kbd class="kbd">"Esc"</kbd>
                            <span>"to cancel"</span>
                        </div>
                    </div>
                })}
                <Button
                    variant=ButtonVariant::Primary
                    on_click=on_submit
                    disabled=submit_disabled
                    loading=false
                >
                    {submit_text}
                </Button>
            </div>
        </div>
    }
}
