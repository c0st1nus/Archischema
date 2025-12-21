use crate::ui::icon::{Icon, icons};
use leptos::prelude::*;

/// Generic form field component with label and input
#[component]
pub fn FormField(
    /// Field label text
    label: String,
    /// Whether field is required (shows red asterisk)
    #[prop(default = false)]
    required: bool,
    /// Input type (text, password, email, etc.)
    #[prop(default = "text")]
    input_type: &'static str,
    /// Placeholder text
    #[prop(default = String::new())]
    placeholder: String,
    /// Current value signal
    value: Signal<String>,
    /// Input event callback
    on_input: Callback<String>,
    /// Whether field is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Optional error message to display
    #[prop(optional)]
    error: Option<Signal<Option<String>>>,
) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <label class="label">
                {label}
                {required.then(|| view! { <span class="text-red-500 ml-0.5">"*"</span> })}
            </label>
            <input
                type=input_type
                class="input-base"
                class:border-red-500=move || error.as_ref().and_then(|e| e.get()).is_some()
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| on_input.run(event_target_value(&ev))
                disabled=disabled
            />
            {move || {
                error.as_ref().and_then(|e| e.get()).map(|err| view! {
                    <div class="flex items-center text-sm text-theme-error">
                        <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                        <span>{err}</span>
                    </div>
                })
            }}
        </div>
    }
}

/// Text area form field component
#[component]
pub fn TextAreaField(
    /// Field label text
    label: String,
    /// Whether field is required (shows red asterisk)
    #[prop(default = false)]
    required: bool,
    /// Placeholder text
    #[prop(default = String::new())]
    placeholder: String,
    /// Current value signal
    value: Signal<String>,
    /// Input event callback
    on_input: Callback<String>,
    /// Number of rows
    #[prop(default = 3)]
    rows: u32,
    /// Whether field is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Optional error message to display
    #[prop(optional)]
    error: Option<Signal<Option<String>>>,
) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <label class="label">
                {label}
                {required.then(|| view! { <span class="text-red-500 ml-0.5">"*"</span> })}
            </label>
            <textarea
                class="input-base resize-none"
                class:border-red-500=move || error.as_ref().and_then(|e| e.get()).is_some()
                placeholder=placeholder
                rows=rows
                prop:value=move || value.get()
                on:input=move |ev| on_input.run(event_target_value(&ev))
                disabled=disabled
            />
            {move || {
                error.as_ref().and_then(|e| e.get()).map(|err| view! {
                    <div class="flex items-center text-sm text-theme-error">
                        <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                        <span>{err}</span>
                    </div>
                })
            }}
        </div>
    }
}

/// Select/dropdown form field component
#[component]
pub fn SelectField(
    /// Field label text
    label: String,
    /// Whether field is required (shows red asterisk)
    #[prop(default = false)]
    required: bool,
    /// Current value signal
    value: Signal<String>,
    /// Change event callback
    on_change: Callback<String>,
    /// Options as (value, display_text) pairs
    options: Vec<(String, String)>,
    /// Whether field is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Optional error message to display
    #[prop(optional)]
    error: Option<Signal<Option<String>>>,
) -> impl IntoView {
    view! {
        <div class="space-y-1.5">
            <label class="label">
                {label}
                {required.then(|| view! { <span class="text-red-500 ml-0.5">"*"</span> })}
            </label>
            <select
                class="select-base"
                class:border-red-500=move || error.as_ref().and_then(|e| e.get()).is_some()
                prop:value=move || value.get()
                on:change=move |ev| {
                    let val = event_target_value(&ev);
                    on_change.run(val);
                }
                disabled=disabled
            >
                {options.into_iter().map(|(val, text)| {
                    view! {
                        <option value=val.clone()>{text}</option>
                    }
                }).collect_view()}
            </select>
            {move || {
                error.as_ref().and_then(|e| e.get()).map(|err| view! {
                    <div class="flex items-center text-sm text-theme-error">
                        <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                        <span>{err}</span>
                    </div>
                })
            }}
        </div>
    }
}

/// Checkbox form field component
#[component]
pub fn CheckboxField(
    /// Field label text
    label: String,
    /// Current checked state
    checked: Signal<bool>,
    /// Change event callback
    on_change: Callback<bool>,
    /// Whether field is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Optional description text below checkbox
    #[prop(optional)]
    description: Option<String>,
) -> impl IntoView {
    view! {
        <div class="flex items-start gap-3">
            <input
                type="checkbox"
                class="mt-1 w-4 h-4 rounded border-theme-primary text-theme-accent focus:ring-2 focus:ring-theme-accent"
                prop:checked=move || checked.get()
                on:change=move |ev| on_change.run(event_target_checked(&ev))
                disabled=disabled
            />
            <div class="flex-1">
                <label class="label cursor-pointer">{label}</label>
                {description.map(|desc| view! {
                    <p class="text-sm text-theme-muted mt-0.5">{desc}</p>
                })}
            </div>
        </div>
    }
}
