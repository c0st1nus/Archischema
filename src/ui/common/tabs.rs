use leptos::prelude::*;

/// Tab item definition
#[derive(Clone, PartialEq)]
pub struct TabItem {
    /// Unique identifier for the tab
    pub id: String,
    /// Display label for the tab
    pub label: String,
    /// Optional icon name
    pub icon: Option<&'static str>,
    /// Whether tab is disabled
    pub disabled: bool,
}

impl TabItem {
    /// Create a new tab item
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    /// Add an icon to the tab
    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark tab as disabled
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Tabs component for organizing content into switchable panels
#[component]
pub fn Tabs(
    /// List of tab items
    tabs: Vec<TabItem>,
    /// Currently active tab ID
    active_tab: ReadSignal<String>,
    /// Callback when tab is changed
    on_change: Callback<String>,
    /// Additional CSS classes for the container
    #[prop(default = String::new())]
    class: String,
    /// Whether tabs should take full width
    #[prop(default = false)]
    full_width: bool,
) -> impl IntoView {
    let container_class = if class.is_empty() {
        "tabs-container".to_string()
    } else {
        format!("tabs-container {}", class)
    };

    let tabs_class = if full_width {
        "tabs-list tabs-full-width"
    } else {
        "tabs-list"
    };

    view! {
        <div class=container_class>
            <div class=tabs_class role="tablist">
                {tabs.into_iter().map(|tab| {
                    let tab_id = tab.id.clone();
                    let is_active = Signal::derive(move || active_tab.get() == tab_id);

                    let tab_class = move || {
                        let mut classes = vec!["tab-item"];
                        if is_active.get() {
                            classes.push("tab-active");
                        }
                        if tab.disabled {
                            classes.push("tab-disabled");
                        }
                        classes.join(" ")
                    };

                    let tab_id_for_click = tab.id.clone();
                    let on_click = move |_| {
                        if !tab.disabled {
                            on_change.run(tab_id_for_click.clone());
                        }
                    };

                    view! {
                        <button
                            class=tab_class
                            on:click=on_click
                            disabled=tab.disabled
                            role="tab"
                            aria-selected=move || is_active.get()
                            aria-controls=format!("panel-{}", tab.id)
                        >
                            {tab.icon.map(|icon| {
                                view! {
                                    <span class="tab-icon">{icon}</span>
                                }
                            })}
                            <span class="tab-label">{tab.label}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Tab panel content component
#[component]
pub fn TabPanel(
    /// Tab ID this panel belongs to
    tab_id: String,
    /// Currently active tab ID
    active_tab: ReadSignal<String>,
    /// Panel content
    children: Children,
    /// Additional CSS classes
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    let tab_id_for_check = tab_id.clone();
    let is_active = Signal::derive(move || active_tab.get() == tab_id_for_check);

    let panel_class = if class.is_empty() {
        "tab-panel".to_string()
    } else {
        format!("tab-panel {}", class)
    };

    view! {
        <div
            class=panel_class
            role="tabpanel"
            id=format!("panel-{}", tab_id)
            style:display=move || if is_active.get() { "block" } else { "none" }
            aria-hidden=move || !is_active.get()
        >
            {children()}
        </div>
    }
}

/// Complete tabs with panels component
#[component]
pub fn TabsWithPanels(
    /// List of tab items
    tabs: Vec<TabItem>,
    /// Currently active tab ID
    active_tab: ReadSignal<String>,
    /// Callback when tab is changed
    on_change: Callback<String>,
    /// Panel content
    children: Children,
    /// Additional CSS classes for the container
    #[prop(default = String::new())]
    class: String,
) -> impl IntoView {
    view! {
        <div class=format!("tabs-with-panels {}", class)>
            <Tabs
                tabs=tabs
                active_tab=active_tab
                on_change=on_change
            />
            <div class="tab-panels">
                {children()}
            </div>
        </div>
    }
}
