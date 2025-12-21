use leptos::prelude::*;
use leptos::web_sys::MouseEvent;

/// Dropdown menu item definition
#[derive(Clone, PartialEq)]
pub struct DropdownItem {
    /// Unique identifier for the item
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional icon name
    pub icon: Option<&'static str>,
    /// Whether item is disabled
    pub disabled: bool,
    /// Whether to show a separator after this item
    pub separator: bool,
    /// Optional variant for styling (e.g., "danger")
    pub variant: Option<DropdownItemVariant>,
}

/// Dropdown item styling variants
#[derive(Clone, Copy, PartialEq)]
pub enum DropdownItemVariant {
    Normal,
    Danger,
    Success,
}

impl DropdownItemVariant {
    fn class(&self) -> &'static str {
        match self {
            DropdownItemVariant::Normal => "dropdown-item-normal",
            DropdownItemVariant::Danger => "dropdown-item-danger",
            DropdownItemVariant::Success => "dropdown-item-success",
        }
    }
}

impl DropdownItem {
    /// Create a new dropdown item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            disabled: false,
            separator: false,
            variant: None,
        }
    }

    /// Add an icon to the item
    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark item as disabled
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Add a separator after this item
    pub fn with_separator(mut self) -> Self {
        self.separator = true;
        self
    }

    /// Set the variant styling
    pub fn with_variant(mut self, variant: DropdownItemVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Mark item as danger (shorthand)
    pub fn danger(mut self) -> Self {
        self.variant = Some(DropdownItemVariant::Danger);
        self
    }
}

/// Dropdown menu alignment
#[derive(Clone, Copy, PartialEq)]
pub enum DropdownAlign {
    Left,
    Right,
}

/// Dropdown component with menu items
#[component]
pub fn Dropdown(
    /// List of dropdown items
    items: Vec<DropdownItem>,
    /// Callback when an item is selected
    on_select: Callback<String>,
    /// Trigger button content
    trigger: Children,
    /// Menu alignment
    #[prop(default = DropdownAlign::Left)]
    align: DropdownAlign,
    /// Additional CSS classes for the container
    #[prop(default = String::new())]
    class: String,
    /// Whether dropdown is disabled
    #[prop(default = false)]
    disabled: bool,
) -> impl IntoView {
    let items = StoredValue::new(items);
    let (is_open, set_is_open) = signal(false);

    let toggle = move |_: MouseEvent| {
        if !disabled {
            set_is_open.update(|open| *open = !*open);
        }
    };

    let close = move || set_is_open.set(false);

    let handle_select = move |item_id: String| {
        on_select.run(item_id);
        close();
    };

    let align_class = match align {
        DropdownAlign::Left => "dropdown-align-left",
        DropdownAlign::Right => "dropdown-align-right",
    };

    let container_class = if class.is_empty() {
        format!("dropdown-container {}", align_class)
    } else {
        format!("dropdown-container {} {}", align_class, class)
    };

    // Close dropdown when clicking outside
    let close_on_outside = move |_| {
        if is_open.get() {
            close();
        }
    };

    view! {
        <div class=container_class>
            <button
                class="dropdown-trigger"
                on:click=toggle
                disabled=disabled
                aria-haspopup="true"
                aria-expanded=move || is_open.get()
            >
                {trigger()}
            </button>

            <Show when=move || is_open.get()>
                <div class="dropdown-backdrop" on:click=close_on_outside></div>
                <div class="dropdown-menu" role="menu">
                    {items.get_value().into_iter().map(|item| {
                        let item_id = item.id.clone();
                        let variant_class = item.variant
                            .map(|v| v.class())
                            .unwrap_or("dropdown-item-normal");

                        let item_class = if item.disabled {
                            format!("{} dropdown-item-disabled", variant_class)
                        } else {
                            variant_class.to_string()
                        };

                        let on_click = move |_: MouseEvent| {
                            if !item.disabled {
                                handle_select(item_id.clone());
                            }
                        };

                        view! {
                            <div>
                                <button
                                    class=format!("dropdown-item {}", item_class)
                                    on:click=on_click
                                    disabled=item.disabled
                                    role="menuitem"
                                >
                                    {item.icon.map(|icon| {
                                        view! {
                                            <span class="dropdown-item-icon">{icon}</span>
                                        }
                                    })}
                                    <span class="dropdown-item-label">{item.label}</span>
                                </button>
                                {item.separator.then(|| {
                                    view! {
                                        <div class="dropdown-separator"></div>
                                    }
                                })}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

/// Simple dropdown with text trigger
#[component]
pub fn SimpleDropdown(
    /// Trigger button text
    trigger_text: String,
    /// List of dropdown items
    items: Vec<DropdownItem>,
    /// Callback when an item is selected
    on_select: Callback<String>,
    /// Menu alignment
    #[prop(default = DropdownAlign::Left)]
    align: DropdownAlign,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <Dropdown
            items=items
            on_select=on_select
            align=align
            class=class
            trigger=Box::new(move || view! { <span>{trigger_text.clone()}</span> }.into_any())
        />
    }
}

/// Icon-based dropdown button
#[component]
pub fn IconDropdown(
    /// Icon name for the trigger
    icon: &'static str,
    /// List of dropdown items
    items: Vec<DropdownItem>,
    /// Callback when an item is selected
    on_select: Callback<String>,
    /// Menu alignment
    #[prop(default = DropdownAlign::Right)]
    align: DropdownAlign,
    /// Optional tooltip
    #[prop(optional)]
    title: Option<String>,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <Dropdown
            items=items
            on_select=on_select
            align=align
            class=class
            trigger=Box::new(move || view! { <span class="dropdown-icon" title=title.clone()>{icon}</span> }.into_any())
        />
    }
}
