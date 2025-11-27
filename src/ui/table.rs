use crate::core::{Column, TableNode};
use leptos::html::Div;
use leptos::prelude::*;
use leptos::web_sys;

#[component]
pub fn TableNodeView(
    node: TableNode,
    #[prop(into)] on_mouse_down: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    let (x, y) = node.position;
    let node_ref = NodeRef::<Div>::new();

    view! {
        <div
            node_ref=node_ref
            class="absolute bg-white border-2 border-gray-300 rounded-lg shadow-lg select-none hover:shadow-xl cursor-grab active:cursor-grabbing"
            style:left=format!("{}px", x)
            style:top=format!("{}px", y)
            style:width="250px"
            style:user-select="none"
            on:mousedown=move |ev| on_mouse_down.run(ev)
            on:contextmenu=move |ev: web_sys::MouseEvent| {
                ev.prevent_default();
            }
        >
            // Заголовок таблицы
            <div class="bg-blue-600 text-white px-4 py-2 rounded-t-lg font-bold cursor-move flex items-center justify-between">
                <span class="text-lg">{node.name.clone()}</span>
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8h16M4 16h16"></path>
                </svg>
            </div>

            // Список колонок
            <div class="p-2">
                {node.columns.into_iter().map(|column| {
                    view! { <ColumnRow column=column /> }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn ColumnRow(column: Column) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between py-1 px-2 hover:bg-gray-50 rounded text-sm border-b border-gray-100 last:border-b-0">
            <div class="flex items-center flex-1">
                {if column.is_primary_key {
                    view! { <span class="text-yellow-500 font-bold mr-1" title="Primary Key">"PK"</span> }.into_any()
                } else {
                    view! { <span class="mr-1"></span> }.into_any()
                }}
                <span class="font-medium text-gray-800">{column.name.clone()}</span>
                {if !column.is_nullable {
                    view! { <span class="text-red-500 text-xs ml-1" title="NOT NULL">"*"</span> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if column.is_unique {
                    view! { <span class="text-blue-500 text-xs ml-1" title="UNIQUE">"U"</span> }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>
            <span class="text-gray-500 text-xs ml-2">{column.data_type}</span>
        </div>
    }
}
