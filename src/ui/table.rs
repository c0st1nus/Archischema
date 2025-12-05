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

    // Use transition only when NOT being dragged locally (for smooth remote updates)
    let table_class = if is_being_dragged {
        "absolute bg-white border-2 border-gray-300 rounded-lg shadow-lg select-none hover:shadow-xl"
    } else {
        "absolute bg-white border-2 border-gray-300 rounded-lg shadow-lg select-none hover:shadow-xl transition-[left,top] duration-100 ease-out"
    };

    view! {
        <div
            node_ref=node_ref
            class=table_class
            style:left=format!("{}px", x)
            style:top=format!("{}px", y)
            style:width="280px"
            style:user-select="none"
            style:z-index="10"
        >
            // Заголовок таблицы
            <div
                class="bg-gradient-to-r from-blue-600 to-blue-700 text-white px-4 py-3 rounded-t-lg font-bold cursor-move flex items-center justify-between"
                on:mousedown=move |ev| on_mouse_down.run(ev)
            >
                <span class="text-lg">{table_name}</span>
                <Icon name=icons::MENU class="w-5 h-5"/>
            </div>

            // Список колонок
            <div class="p-2 max-h-96 overflow-y-auto">
                {if !has_columns {
                    view! {
                        <div class="text-center py-4 text-gray-400 text-sm">
                            "No columns"
                            <div class="text-xs mt-1 text-gray-500">"Use sidebar to add"</div>
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

#[component]
fn ColumnRow(column: Column) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between py-2 px-2 hover:bg-gray-50 rounded text-sm border-b border-gray-100 last:border-b-0 transition-colors">
            <div class="flex items-center flex-1 min-w-0">
                {if column.is_primary_key {
                    view! {
                        <span class="text-yellow-500 font-bold mr-2 text-xs flex-shrink-0" title="Primary Key">
                            "PK"
                        </span>
                    }
                        .into_any()
                } else {
                    view! { <span class="w-6"></span> }.into_any()
                }}
                <span class="font-medium text-gray-800 truncate">{column.name}</span>
                {if !column.is_nullable {
                    view! {
                        <span class="text-red-500 text-xs ml-1 flex-shrink-0" title="NOT NULL">
                            "*"
                        </span>
                    }
                        .into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if column.is_unique {
                    view! {
                        <span class="text-blue-500 text-xs ml-1 flex-shrink-0" title="UNIQUE">
                            "U"
                        </span>
                    }
                        .into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>
            <div class="flex items-center space-x-2">
                <span class="text-gray-500 text-xs ml-2 flex-shrink-0">{column.data_type.to_string()}</span>
            </div>
        </div>
    }
}
