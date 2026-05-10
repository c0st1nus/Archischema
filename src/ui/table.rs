use crate::core::{Column, TableNode};
use crate::ui::{Icon, icons};
use leptos::html::Div;
use leptos::prelude::*;
use leptos::web_sys;

pub const TABLE_NODE_WIDTH: f64 = 280.0;
pub const TABLE_HEADER_HEIGHT: f64 = 42.0;
pub const TABLE_ROW_HEIGHT: f64 = 30.0;
pub const TABLE_BODY_PADDING_Y: f64 = 6.0;
pub const TABLE_EDGE_GAP: f64 = 30.0;

#[component]
pub fn TableNodeView(
    node: TableNode,
    #[prop(into)] on_mouse_down: Callback<web_sys::MouseEvent>,
    /// Click handler for selecting the table
    #[prop(into)]
    on_click: Callback<web_sys::MouseEvent>,
    /// Whether this table is currently being dragged locally (disables transition)
    #[prop(default = false)]
    is_being_dragged: bool,
    /// Whether this table is selected (highlights all its relationships)
    #[prop(default = false)]
    is_selected: bool,
    /// Index of the column currently open in the floating editor.
    #[prop(default = None)]
    active_column_index: Option<usize>,
) -> impl IntoView {
    let (x, y) = node.position;
    let node_ref = NodeRef::<Div>::new();

    // Сохраняем имя таблицы для избежания клонирования в замыканиях
    let table_name = node.name.clone();
    let has_columns = !node.columns.is_empty();

    // No CSS transition for position - we use requestAnimationFrame interpolation for smooth remote updates
    // is_being_dragged disables any remaining transitions for both local and remote drags
    let table_class = match (is_selected, is_being_dragged) {
        (true, true) => "schema-table-card is-selected is-dragging",
        (true, false) => "schema-table-card is-selected",
        (false, true) => "schema-table-card is-dragging",
        (false, false) => "schema-table-card",
    };

    view! {
        <div
            node_ref=node_ref
            class=table_class
            style:left=format!("{}px", x)
            style:top=format!("{}px", y)
            style:width=format!("{}px", TABLE_NODE_WIDTH)
            style:user-select="none"
            style:z-index=if is_selected { "12" } else { "10" }
        >
            // Заголовок таблицы
            <div
                class="schema-table-header"
                style:height=format!("{}px", TABLE_HEADER_HEIGHT)
                on:mousedown=move |ev| on_mouse_down.run(ev)
                on:click=move |ev| on_click.run(ev)
            >
                <span class="schema-table-title">{table_name}</span>
                <Icon name=icons::GRIP_HORIZONTAL class="w-4 h-4 schema-table-grip"/>
            </div>

            // Список колонок
            <div class="schema-table-body scroll">
                {if !has_columns {
                    view! {
                        <div class="rounded-md border border-theme bg-theme-tertiary/20 py-4 text-center text-sm text-theme-muted">
                            "No columns"
                            <div class="text-xs mt-1 text-theme-tertiary">"Use sidebar to add"</div>
                        </div>
                    }
                        .into_any()
                } else {
                    node
                        .columns
                        .into_iter()
                        .enumerate()
                        .map(|(column_index, column)| {
                            view! { <ColumnRow column=column is_active=active_column_index == Some(column_index)/> }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
        </div>
    }
}

/// Optimized ColumnRow component using CSS-based conditional styling
/// instead of multiple into_any() calls for conditional rendering
#[component]
fn ColumnRow(column: Column, #[prop(default = false)] is_active: bool) -> impl IntoView {
    let data_type_display = column.data_type.clone();
    let column_name = column.name.clone();

    view! {
        <div
            class=if is_active { "schema-table-row is-active" } else { "schema-table-row" }
            style:height=format!("{}px", TABLE_ROW_HEIGHT)
        >
            <div class="column-inline-main">
                <span class="column-inline-name schema-table-column-name" title=column_name.clone()>{column_name.clone()}</span>
                <span class="column-inline-badges">
                {if column.is_primary_key {
                    view! {
                        <span class="schema-table-badge schema-table-badge-pk" title="Primary Key">"PK"</span>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if !column.is_nullable {
                    view! {
                        <span class="schema-table-badge schema-table-badge-nn" title="NOT NULL">"NN"</span>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if column.is_unique {
                    view! {
                        <span class="schema-table-badge schema-table-badge-uq" title="UNIQUE">"UQ"</span>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                </span>
            </div>
            <span class="column-inline-type schema-table-column-type" title=data_type_display.clone()>{data_type_display.clone()}</span>
        </div>
    }
}
