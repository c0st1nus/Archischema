//! Small design-system primitives used by the redesigned UI.

use crate::ui::icon::{Icon, icons};
use leptos::prelude::*;

#[component]
pub fn BrandMark(
    #[prop(default = "w-5 h-5")] icon_class: &'static str,
    #[prop(default = false)] with_label: bool,
    #[prop(default = false)] boxed: bool,
    #[prop(default = None)] version: Option<&'static str>,
) -> impl IntoView {
    view! {
        <span class="brand-row">
            {if boxed {
                view! {
                    <span class="brand-icon-box">
                        <Icon name=icons::SQUARES_2X2 class=icon_class />
                    </span>
                }.into_any()
            } else {
                view! {
                    <Icon name=icons::SQUARES_2X2 class="brand-icon" />
                }.into_any()
            }}
            {with_label.then(|| view! {
                <span>"ArchiSchema"</span>
            })}
            {version.map(|version| view! {
                <span class="chip h-[18px] text-[10.5px]">{version}</span>
            })}
        </span>
    }
}

#[component]
pub fn Sep(
    #[prop(default = false)] vertical: bool,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    let base = if vertical { "hairline-v" } else { "hairline-h" };
    let full_class = if class.is_empty() {
        base.to_string()
    } else {
        format!("{} {}", base, class)
    };

    view! { <div class=full_class></div> }
}

#[component]
pub fn Field(
    children: Children,
    #[prop(default = String::new())] label: String,
    #[prop(default = None)] hint: Option<String>,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    view! {
        <label class=class>
            {(!label.is_empty()).then(|| view! { <span class="field-label">{label}</span> })}
            {children()}
            {hint.map(|hint| view! { <span class="field-help">{hint}</span> })}
        </label>
    }
}

#[component]
pub fn KbdKey(children: Children, #[prop(default = String::new())] class: String) -> impl IntoView {
    let full_class = if class.is_empty() {
        "kbd-key".to_string()
    } else {
        format!("kbd-key {}", class)
    };

    view! { <span class=full_class>{children()}</span> }
}

#[derive(Clone, PartialEq)]
pub struct SegmentedOption {
    pub value: String,
    pub label: String,
    pub icon: Option<&'static str>,
}

impl SegmentedOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
        }
    }

    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }
}

#[component]
pub fn SegmentedControl(
    options: Vec<SegmentedOption>,
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(default = false)] full_width: bool,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    let width_class = if full_width { " w-full" } else { "" };
    let full_class = if class.is_empty() {
        format!("segmented{}", width_class)
    } else {
        format!("segmented{} {}", width_class, class)
    };

    view! {
        <div class=full_class>
            {options.into_iter().map(|option| {
                let option_value = option.value.clone();
                let active_for_attr = option.value.clone();
                let active_for_class = option.value.clone();
                let icon = option.icon;
                view! {
                    <button
                        type="button"
                        class=if full_width { "flex-1" } else { "" }
                        attr:data-active=move || if value.get() == active_for_attr { "true" } else { "false" }
                        aria-pressed=move || value.get() == active_for_class
                        on:click=move |_| on_change.run(option_value.clone())
                    >
                        {icon.map(|icon| view! { <Icon name=icon class="h-3 w-3" /> })}
                        <span>{option.label}</span>
                    </button>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn Surface(
    children: Children,
    #[prop(default = String::new())] class: String,
    #[prop(default = false)] elevated: bool,
) -> impl IntoView {
    let base = if elevated { "surface-elev" } else { "surface" };
    let full_class = if class.is_empty() {
        base.to_string()
    } else {
        format!("{} {}", base, class)
    };

    view! { <div class=full_class>{children()}</div> }
}

#[component]
pub fn Chip(children: Children, #[prop(default = String::new())] class: String) -> impl IntoView {
    let full_class = if class.is_empty() {
        "chip".to_string()
    } else {
        format!("chip {}", class)
    };

    view! { <span class=full_class>{children()}</span> }
}

#[component]
pub fn AvatarInitials(
    name: String,
    #[prop(optional)] color: Option<String>,
    #[prop(default = String::new())] class: String,
    #[prop(default = false)] ring: bool,
) -> impl IntoView {
    let initials = name
        .split_whitespace()
        .take(2)
        .filter_map(|part| part.chars().next())
        .collect::<String>()
        .to_uppercase();
    let initials = if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    };
    let color = color.unwrap_or_else(|| "oklch(0.55 0.13 285)".to_string());
    let full_class = format!("av {} {}", if ring { "av-ring" } else { "" }, class);

    view! {
        <span class=full_class style=format!("background: {}; color: {};", color, color)>
            <span class="text-white">{initials}</span>
        </span>
    }
}

#[component]
pub fn StatTile(
    label: String,
    value: String,
    #[prop(optional)] icon: Option<&'static str>,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    view! {
        <Surface class=format!("p-3 {}", class)>
            <div class="mb-1 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-theme-muted">
                {icon.map(|icon_name| view! { <Icon name=icon_name class="h-3 w-3" /> })}
                <span>{label}</span>
            </div>
            <div class="font-mono text-xl font-semibold tracking-[-0.02em] text-theme-primary">{value}</div>
        </Surface>
    }
}

#[component]
pub fn EmptyState(
    title: String,
    description: String,
    children: Children,
    #[prop(optional)] icon: Option<&'static str>,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    view! {
        <div class=format!("mx-auto flex max-w-md flex-col items-center justify-center text-center {}", class)>
            <div class="mb-4 flex h-14 w-14 items-center justify-center rounded-xl border border-theme bg-theme-accent-light text-theme-accent shadow-theme-sm">
                <Icon name=icon.unwrap_or(icons::DATABASE) class="h-7 w-7" />
            </div>
            <h2 class="text-xl font-semibold tracking-[-0.02em] text-theme-primary">{title}</h2>
            <p class="mt-2 text-sm leading-relaxed text-theme-tertiary">{description}</p>
            <div class="mt-5 flex flex-wrap items-center justify-center gap-2">{children()}</div>
        </div>
    }
}

#[component]
pub fn NeutralVisualPlaceholder(
    #[prop(default = String::new())] label: String,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    view! {
        <div class=format!("neutral-visual {}", class)>
            {(!label.is_empty()).then(|| view! {
                <div class="absolute left-3 top-3 z-10 rounded-full border border-theme bg-theme-surface/80 px-2 py-1 font-mono text-[10px] text-theme-muted backdrop-blur">
                    {label}
                </div>
            })}
        </div>
    }
}

#[component]
pub fn AuthVisualPane() -> impl IntoView {
    view! {
        <div class="auth-visual-pane hidden lg:block">
            <NeutralVisualPlaceholder
                label="Model workspace".to_string()
                class="absolute left-[60px] top-[80px] h-[150px] w-[260px] rotate-[-3deg] shadow-theme-lg".to_string()
            />
            <NeutralVisualPlaceholder
                label="Source sync".to_string()
                class="absolute right-[50px] top-[220px] h-[150px] w-[260px] rotate-[2deg] shadow-theme-lg".to_string()
            />
            <NeutralVisualPlaceholder
                label="Team review".to_string()
                class="absolute bottom-[80px] left-[100px] h-[150px] w-[260px] rotate-[-1.5deg] shadow-theme-lg".to_string()
            />

            <div class="absolute bottom-[30px] left-10 right-10 rounded-xl border border-theme bg-theme-secondary/80 p-[18px] backdrop-blur">
                <p class="text-[13px] leading-[1.55] text-theme-secondary">
                    "We dropped Lucidchart and Miro for our schema work. ArchiSchema is the only tool that feels like a database IDE and a whiteboard at the same time."
                </p>
                <div class="mt-2.5 flex items-center gap-2">
                    <AvatarInitials name="Marcus Reyes".to_string() color="oklch(0.55 0.13 285)".to_string() class="av-lg".to_string() />
                    <div>
                        <div class="text-[12.5px] font-semibold text-theme-primary">"Marcus Reyes"</div>
                        <div class="text-[11.5px] text-theme-muted">"Staff engineer - Plinth"</div>
                    </div>
                </div>
            </div>
        </div>
    }
}
