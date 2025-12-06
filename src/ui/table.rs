use crate::core::{Column, TableNode};
use crate::ui::{Icon, icons};
use leptos::html::Div;
use leptos::prelude::*;
use leptos::web_sys;

#[component]
pub fn TableNodeView(
    node: TableNode,
    #[prop(into)] on_mouse_down: Callback<web_sys::MouseEvent>,
    /// Whether this table is currently being dragged locally (disables transition)
    #[prop(default = false)]
    is_being_dragged: bool,
) -> impl IntoView {
    let (x, y) = node.position;
    let node_ref = NodeRef::<Div>::new();

    // Сохраняем имя таблицы для избежания клонирования в замыканиях
    let table_name = node.name.clone();
    let has_columns = !node.columns.is_empty();

    // No CSS transition for position - we use requestAnimationFrame interpolation for smooth remote updates
    // is_being_dragged disables any remaining transitions for both local and remote drags
    let table_class = "absolute bg-theme-surface border-2 border-theme-primary shadow-theme-lg select-none hover:shadow-theme-xl theme-transition";
    let _ = is_being_dragged; // Used in canvas.rs to track dragging state

    view! {
        <div
            node_ref=node_ref
            class=table_class
            style:left=format!("{}px", x)
            style:top=format!("{}px", y)
            style:width="280px"
            style:user-select="none"
            style:z-index="10"
            style:border-radius="8px"
        >
            // Заголовок таблицы
            <div
                class="text-white px-4 py-3 font-bold cursor-move flex items-center justify-between"
                style="background: linear-gradient(to right, var(--accent-primary), var(--accent-secondary)); border-radius: 6px 6px 0 0;"
                on:mousedown=move |ev| on_mouse_down.run(ev)
            >
                <span class="text-lg">{table_name}</span>
                <Icon name=icons::MENU class="w-5 h-5"/>
            </div>

            // Список колонок
            <div class="p-2 max-h-96 overflow-y-auto">
                {if !has_columns {
                    view! {
                        <div class="text-center py-4 text-theme-muted text-sm">
                            "No columns"
                            <div class="text-xs mt-1 text-theme-tertiary">"Use sidebar to add"</div>
                        </div>
                    }
                        .into_any()
                } else {
                    node
                        .columns
                        .into_iter()
                        .map(|column| {
                            view! { <ColumnRow column=column/> }
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
fn ColumnRow(column: Column) -> impl IntoView {
    // Pre-compute CSS classes and text content to avoid runtime branching in view
    let pk_class = if column.is_primary_key {
        "text-yellow-500 font-bold mr-2 text-xs flex-shrink-0"
    } else {
        "w-6 inline-block"
    };
    let pk_text = if column.is_primary_key { "PK" } else { "" };

    // NOT NULL indicator - use visibility instead of conditional render
    let not_null_class = if !column.is_nullable {
        "text-red-500 text-xs ml-1 flex-shrink-0"
    } else {
        "hidden"
    };

    // UNIQUE indicator
    let unique_class = if column.is_unique {
        "text-blue-500 text-xs ml-1 flex-shrink-0"
    } else {
        "hidden"
    };

    // Clone data_type once for display
    let data_type_display = column.data_type.clone();

    view! {
        <div class="flex items-center justify-between py-2 px-2 hover:bg-theme-secondary rounded text-sm border-b border-theme-primary last:border-b-0 theme-transition">
            <div class="flex items-center flex-1 min-w-0">
                <span class=pk_class title="Primary Key">{pk_text}</span>
                <span class="font-medium text-theme-primary truncate">{column.name}</span>
                <span class=not_null_class title="NOT NULL">"*"</span>
                <span class=unique_class title="UNIQUE">"U"</span>
            </div>
            <div class="flex items-center space-x-2">
                <span class="text-theme-tertiary text-xs ml-2 flex-shrink-0">{data_type_display}</span>
            </div>
        </div>
    }
}
