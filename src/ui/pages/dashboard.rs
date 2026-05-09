//! Dashboard page component
//!
//! Displays user's diagrams and folders in a grid view with search,
//! sorting, filtering, and organization features.
//! Supports drag-and-drop to move diagrams and folders between directories.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};

use crate::ui::auth::{AuthContext, AuthState, User, logout, use_auth_context};
use crate::ui::common::{AvatarInitials, BrandMark};
use crate::ui::icon::{Icon, icons};
use crate::ui::theme::{ThemeMode, use_theme_context};

/// Diagram summary data from API (matches DiagramSummary on server)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub id: String,
    #[serde(default)]
    pub owner_id: String,
    pub name: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub is_public: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Folder data from API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub children_count: i64,
    #[serde(default)]
    pub diagrams_count: i64,
}

/// API response wrapper for folders list
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct FolderListResponse {
    pub folders: Vec<Folder>,
    #[allow(dead_code)]
    pub count: usize,
}

/// API response wrapper for diagrams list (matches DiagramListResponse on server)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct DiagramListResponse {
    pub diagrams: Vec<Diagram>,
    #[allow(dead_code)]
    pub total: i64,
    #[allow(dead_code)]
    pub limit: i64,
    #[allow(dead_code)]
    pub offset: i64,
}

/// Sort options for diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOption {
    NameAsc,
    NameDesc,
    UpdatedDesc,
    UpdatedAsc,
    CreatedDesc,
    CreatedAsc,
}

/// Error type for API calls that can detect authentication failures
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ApiError {
    /// User is not authenticated (401 error)
    Unauthorized,
    /// Other error with message
    Other(String),
}

impl SortOption {
    fn label(&self) -> &'static str {
        match self {
            SortOption::NameAsc => "Name (A-Z)",
            SortOption::NameDesc => "Name (Z-A)",
            SortOption::UpdatedDesc => "Recently Updated",
            SortOption::UpdatedAsc => "Oldest Updated",
            SortOption::CreatedDesc => "Recently Created",
            SortOption::CreatedAsc => "Oldest Created",
        }
    }
}

/// Item type for drag-and-drop
#[derive(Debug, Clone, PartialEq)]
pub enum DragItem {
    Diagram(String),
    Folder(String),
}

/// Dashboard page component
#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();
    let navigate = use_navigate();

    // State
    let diagrams = RwSignal::new(Vec::<Diagram>::new());
    let folders = RwSignal::new(Vec::<Folder>::new());
    let current_folder = RwSignal::new(None::<String>);
    let parent_folder_id = RwSignal::new(None::<String>);
    let search_query = RwSignal::new(String::new());
    let sort_option = RwSignal::new(SortOption::UpdatedDesc);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let auth_error = RwSignal::new(false);

    // Drag-and-drop state
    let dragging_item = RwSignal::new(None::<DragItem>);
    let drop_target = RwSignal::new(None::<Option<String>>); // None = no target, Some(None) = root, Some(Some(id)) = folder

    // Modal states
    let show_new_folder = RwSignal::new(false);
    let show_new_diagram = RwSignal::new(false);
    let show_sort_dropdown = RwSignal::new(false);

    // Delete confirmation modal state
    let show_delete_modal = RwSignal::new(false);
    let delete_target = RwSignal::new(None::<(String, String)>); // (id, name)
    let deleting = RwSignal::new(false);

    // Rename modal state
    let show_rename_modal = RwSignal::new(false);
    let rename_target = RwSignal::new(None::<(String, String)>); // (id, current_name)
    let renaming = RwSignal::new(false);

    // Folder delete confirmation modal state
    let show_delete_folder_modal = RwSignal::new(false);
    let delete_folder_target = RwSignal::new(None::<(String, String)>); // (id, name)
    let deleting_folder = RwSignal::new(false);

    // Folder rename modal state
    let show_rename_folder_modal = RwSignal::new(false);
    let rename_folder_target = RwSignal::new(None::<(String, String)>); // (id, current_name)
    let renaming_folder = RwSignal::new(false);

    // Breadcrumb path
    let folder_path = RwSignal::new(Vec::<Folder>::new());

    // Redirect if not authenticated
    Effect::new(move |_| {
        if matches!(auth.state.get(), AuthState::Unauthenticated) {
            navigate("/", Default::default());
        }
    });

    // Load diagrams and folders
    Effect::new(move |_| {
        if matches!(auth.state.get(), AuthState::Authenticated(_)) {
            let folder_id = current_folder.get();
            spawn_local(async move {
                loading.set(true);
                error.set(None);

                // Fetch folders, diagrams, and current folder info in parallel
                let folder_id_clone = folder_id.clone();
                let folders_result = fetch_folders(folder_id.clone()).await;
                let diagrams_result = fetch_diagrams(folder_id.clone()).await;
                let current_folder_info = fetch_folder_info(folder_id_clone).await;

                match (folders_result, diagrams_result) {
                    (Ok(f), Ok(d)) => {
                        folders.set(f);
                        diagrams.set(d);
                        auth_error.set(false);

                        // Set parent folder ID for ".." navigation
                        if let Ok(Some(info)) = current_folder_info {
                            parent_folder_id.set(info.parent_id);
                        } else {
                            parent_folder_id.set(None);
                        }
                    }
                    (Err(ApiError::Unauthorized), _) | (_, Err(ApiError::Unauthorized)) => {
                        auth_error.set(true);
                        error.set(None);
                    }
                    (Err(ApiError::Other(e)), _) | (_, Err(ApiError::Other(e))) => {
                        error.set(Some(e));
                    }
                }

                // Fetch folder path for breadcrumbs
                if let Some(ref folder_id) = current_folder.get_untracked() {
                    if let Ok(path) = fetch_folder_path(folder_id.clone()).await {
                        folder_path.set(path);
                    }
                } else {
                    folder_path.set(vec![]);
                }

                loading.set(false);
            });
        }
    });

    // Filtered and sorted diagrams
    let filtered_diagrams = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        let sort = sort_option.get();
        let mut result: Vec<Diagram> = diagrams
            .get()
            .into_iter()
            .filter(|d| {
                if query.is_empty() {
                    true
                } else {
                    d.name.to_lowercase().contains(&query)
                        || d.description
                            .as_ref()
                            .map(|desc| desc.to_lowercase().contains(&query))
                            .unwrap_or(false)
                }
            })
            .collect();

        // Sort
        match sort {
            SortOption::NameAsc => result.sort_by(|a, b| a.name.cmp(&b.name)),
            SortOption::NameDesc => result.sort_by(|a, b| b.name.cmp(&a.name)),
            SortOption::UpdatedDesc => result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            SortOption::UpdatedAsc => result.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
            SortOption::CreatedDesc => result.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortOption::CreatedAsc => result.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        }

        result
    });

    // Filtered folders (by search)
    let filtered_folders = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        folders
            .get()
            .into_iter()
            .filter(|f| {
                if query.is_empty() {
                    true
                } else {
                    f.name.to_lowercase().contains(&query)
                }
            })
            .collect::<Vec<_>>()
    });

    // Create new diagram handler - now opens modal
    let on_create_diagram = move |_: web_sys::MouseEvent| {
        show_new_diagram.set(true);
    };

    // Handle delete diagram
    let handle_delete = move || {
        if let Some((diagram_id, _)) = delete_target.get() {
            deleting.set(true);
            spawn_local(async move {
                match delete_diagram(&diagram_id).await {
                    Ok(_) => {
                        // Refresh diagrams
                        let folder_id = current_folder.get_untracked();
                        if let Ok(d) = fetch_diagrams(folder_id).await {
                            diagrams.set(d);
                        }
                        show_delete_modal.set(false);
                        delete_target.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                deleting.set(false);
            });
        }
    };

    // Handle rename diagram
    let handle_rename = move |new_name: String| {
        if let Some((diagram_id, _)) = rename_target.get() {
            renaming.set(true);
            spawn_local(async move {
                match rename_diagram(&diagram_id, &new_name).await {
                    Ok(_) => {
                        // Refresh diagrams
                        let folder_id = current_folder.get_untracked();
                        if let Ok(d) = fetch_diagrams(folder_id).await {
                            diagrams.set(d);
                        }
                        show_rename_modal.set(false);
                        rename_target.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                renaming.set(false);
            });
        }
    };

    // Handle delete folder
    let handle_delete_folder = move || {
        if let Some((folder_id, _)) = delete_folder_target.get() {
            deleting_folder.set(true);
            spawn_local(async move {
                match delete_folder(&folder_id).await {
                    Ok(_) => {
                        // Refresh folders
                        let current = current_folder.get_untracked();
                        if let Ok(f) = fetch_folders(current).await {
                            folders.set(f);
                        }
                        show_delete_folder_modal.set(false);
                        delete_folder_target.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                deleting_folder.set(false);
            });
        }
    };

    // Handle rename folder
    let handle_rename_folder = move |new_name: String| {
        if let Some((folder_id, _)) = rename_folder_target.get() {
            renaming_folder.set(true);
            spawn_local(async move {
                match rename_folder(&folder_id, &new_name).await {
                    Ok(_) => {
                        // Refresh folders
                        let current = current_folder.get_untracked();
                        if let Ok(f) = fetch_folders(current).await {
                            folders.set(f);
                        }
                        show_rename_folder_modal.set(false);
                        rename_folder_target.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                renaming_folder.set(false);
            });
        }
    };

    // Handle drop on folder or root
    let handle_drop = move |target_folder_id: Option<String>| {
        let item = dragging_item.get();
        if item.is_none() {
            return;
        }

        let item = item.unwrap();

        // Get the current folder_id of the dragged item to compare
        let item_current_folder = match &item {
            DragItem::Diagram(id) => diagrams
                .get()
                .iter()
                .find(|d| &d.id == id)
                .and_then(|d| d.folder_id.clone()),
            DragItem::Folder(id) => folders
                .get()
                .iter()
                .find(|f| &f.id == id)
                .and_then(|f| f.parent_id.clone()),
        };

        // Don't move to the same folder it's already in
        if target_folder_id == item_current_folder {
            dragging_item.set(None);
            drop_target.set(None);
            return;
        }

        spawn_local(async move {
            let result = match item {
                DragItem::Diagram(id) => {
                    move_diagram_to_folder(&id, target_folder_id.clone()).await
                }
                DragItem::Folder(id) => {
                    // Don't allow moving folder into itself
                    if Some(id.clone()) == target_folder_id {
                        dragging_item.set(None);
                        drop_target.set(None);
                        return;
                    }
                    move_folder_to_parent(&id, target_folder_id.clone()).await
                }
            };

            match result {
                Ok(_) => {
                    // Refresh the current folder
                    let folder_id = current_folder.get();
                    let folders_result = fetch_folders(folder_id.clone()).await;
                    let diagrams_result = fetch_diagrams(folder_id).await;

                    if let Ok(f) = folders_result {
                        folders.set(f);
                    }
                    if let Ok(d) = diagrams_result {
                        diagrams.set(d);
                    }
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }

            dragging_item.set(None);
            drop_target.set(None);
        });
    };

    view! {
        <div class="app-chrome flex min-h-screen flex-col overflow-hidden">
            <header class="chrome-header sticky dashboard-header">
                <A href="/" attr:class="brand-row shrink-0 hover:opacity-85 theme-transition">
                    <BrandMark with_label=true version=Some("v0.9.2") />
                </A>

                <div class="hairline-v hidden h-[18px] self-center lg:block"></div>

                <nav class="chrome-nav hidden lg:flex" aria-label="Dashboard navigation">
                    <A href="/dashboard" attr:class="btn dashboard-nav-active">"Diagrams"</A>
                    <span class="btn btn-ghost dashboard-nav-muted">"Templates"</span>
                    <span class="btn btn-ghost dashboard-nav-muted">"Shared with me"</span>
                </nav>

                <div class="flex-1"></div>

                <div class="chrome-search dashboard-search hidden md:block">
                    <Icon name=icons::SEARCH class="chrome-search-icon" />
                    <input
                        type="search"
                        placeholder="Search diagrams, tables, columns..."
                        class="input"
                        prop:value=move || search_query.get()
                        on:input=move |ev| search_query.set(event_target_value(&ev))
                    />
                    <span class="absolute right-2 top-1/2 -translate-y-1/2">
                        <span class="kbd-key">"⌘K"</span>
                    </span>
                </div>

                <button
                    type="button"
                    class="btn-icon"
                    on:click=move |_| theme.toggle()
                    aria-label="Toggle theme"
                >
                    {move || if theme.mode.get() == ThemeMode::Dark {
                        view! { <Icon name=icons::SUN class="h-4 w-4" /> }.into_any()
                    } else {
                        view! { <Icon name=icons::MOON class="h-4 w-4" /> }.into_any()
                    }}
                </button>

                <button type="button" class="btn-icon hidden sm:inline-flex" aria-label="Notifications">
                    <Icon name=icons::ALERT_CIRCLE class="h-4 w-4" />
                </button>

                <DashboardUserPill auth=auth />
            </header>

            <main class="dashboard-scroll scroll">
                <div class="mb-4 md:hidden">
                    <div class="chrome-search w-full">
                        <Icon name=icons::SEARCH class="chrome-search-icon" />
                        <input
                            type="search"
                            placeholder="Search diagrams, tables, columns..."
                            class="input"
                            prop:value=move || search_query.get()
                            on:input=move |ev| search_query.set(event_target_value(&ev))
                        />
                    </div>
                </div>

                <nav class="dashboard-breadcrumb" aria-label="Breadcrumb">
                    <Icon name=icons::FOLDER class="h-3.5 w-3.5" />
                    <button
                        type="button"
                        class="hover:text-theme-primary theme-transition"
                        on:click=move |_| {
                            current_folder.set(None);
                            parent_folder_id.set(None);
                        }
                    >
                        "All diagrams"
                    </button>
                    {move || {
                        let path = folder_path.get();
                        path.into_iter().map(|folder| {
                            let folder_id = folder.id.clone();
                            let folder_name = folder.name.clone();
                            view! {
                                <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                                <button
                                    type="button"
                                    class="font-medium text-theme-primary"
                                    on:click=move |_| current_folder.set(Some(folder_id.clone()))
                                >
                                    {folder_name}
                                </button>
                            }
                        }).collect_view()
                    }}
                </nav>

                <div class="dashboard-hero-row">
                    <div class="min-w-0">
                        <h1 class="dashboard-title">
                            {move || folder_path.get().last().map(|folder| folder.name.clone()).unwrap_or_else(|| "All diagrams".to_string())}
                        </h1>
                        <p class="dashboard-subtitle">
                            {move || format!(
                                "{} diagrams · {} folders · sorted by {}",
                                filtered_diagrams.get().len(),
                                filtered_folders.get().len(),
                                sort_option.get().label().to_lowercase()
                            )}
                        </p>
                    </div>

                    <div class="dashboard-toolbar">
                        <button
                            type="button"
                            class="btn-primary btn-lg"
                            on:click=on_create_diagram
                        >
                            <Icon name=icons::PLUS class="h-3.5 w-3.5" />
                            "New diagram"
                            <span class="kbd-key bg-black/20 text-white">"N"</span>
                        </button>
                        <button
                            type="button"
                            class="btn btn-lg"
                            on:click=move |_| show_new_folder.set(true)
                        >
                            <Icon name=icons::FOLDER_PLUS class="h-3.5 w-3.5" />
                            "New folder"
                        </button>
                        <button class="btn btn-lg" type="button" disabled title="Import is available in the editor">
                            <Icon name=icons::ARROW_DOWN_TO_LINE class="h-3.5 w-3.5" />
                            "Import"
                        </button>

                        <div class="segmented hidden xl:inline-flex" aria-label="View mode">
                            <button type="button" data-active="true">
                                <Icon name=icons::GRID class="h-3 w-3" />
                                "Grid"
                            </button>
                            <button type="button" data-active="false" disabled>
                                <Icon name=icons::LAYOUT class="h-3 w-3" />
                                "List"
                            </button>
                        </div>

                        <div class="relative">
                            <button
                                type="button"
                                class="btn btn-lg"
                                on:click=move |_| show_sort_dropdown.update(|v| *v = !*v)
                            >
                                <Icon name=icons::SORT class="h-4 w-4" />
                                <span>{move || format!("Sort: {}", sort_option.get().label().replace("Recently ", ""))}</span>
                            </button>

                            {move || {
                                if show_sort_dropdown.get() {
                                    Some(view! {
                                        <div class="dropdown-menu right-0 mt-2 w-52">
                                            <SortButton option=SortOption::NameAsc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                            <SortButton option=SortOption::NameDesc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                            <SortButton option=SortOption::UpdatedDesc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                            <SortButton option=SortOption::UpdatedAsc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                            <SortButton option=SortOption::CreatedDesc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                            <SortButton option=SortOption::CreatedAsc current=sort_option on_select=move |o| { sort_option.set(o); show_sort_dropdown.set(false); } />
                                        </div>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </div>

                        <button class="btn btn-lg" type="button" disabled>
                            <Icon name=icons::FILTER class="h-4 w-4" />
                            "Filter"
                        </button>
                    </div>
                </div>

                {move || {
                    if auth_error.get() {
                        Some(view! {
                            <div class="mb-6 rounded-lg border border-theme-warning bg-theme-warning p-6">
                                <div class="flex items-start gap-4">
                                    <Icon name=icons::WARNING class="h-6 w-6 shrink-0 text-theme-warning" />
                                    <div class="flex-1">
                                        <h3 class="text-sm font-semibold text-theme-primary">"Session expired"</h3>
                                        <p class="mt-1 text-sm text-theme-secondary">
                                            "Your session has expired or is invalid. Please log in again to access your diagrams."
                                        </p>
                                        <div class="mt-4">
                                            <A href="/login" attr:class="btn-primary">
                                                <Icon name=icons::LOGOUT class="h-4 w-4" />
                                                "Go to Login"
                                            </A>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                {move || {
                    error.get().map(|err| view! {
                        <div class="mb-6 rounded-lg border border-theme-error bg-theme-error p-4">
                            <p class="text-sm text-theme-error">{err}</p>
                        </div>
                    })
                }}

                {move || {
                    if loading.get() {
                        Some(view! {
                            <div class="flex items-center justify-center py-20">
                                <Icon name=icons::LOADER class="h-8 w-8 animate-spin text-accent-primary" />
                            </div>
                        }.into_any())
                    } else {
                        let folders_list = filtered_folders.get();
                        let diagrams_list = filtered_diagrams.get();
                        let is_in_subfolder = current_folder.get().is_some();
                        let parent_id = parent_folder_id.get();
                        let folders_count = folders_list.len();
                        let diagrams_count = diagrams_list.len();
                        let handle_drop_clone = handle_drop;

                        Some(view! {
                            <div class="dashboard-content-stack">
                                {if is_in_subfolder || !folders_list.is_empty() {
                                    let parent_id_for_card = parent_id.clone();
                                    Some(view! {
                                        <section>
                                            <div class="eyebrow mb-2 px-0">"Folders"</div>
                                            <div class="dashboard-folder-grid">
                                                {if is_in_subfolder {
                                                    let target_folder = parent_id_for_card.clone();
                                                    let parent_id_click = parent_id_for_card.clone();
                                                    let parent_id_enter = parent_id_for_card.clone();
                                                    let parent_id_drop = parent_id_for_card.clone();
                                                    let handle_drop_parent = handle_drop_clone;
                                                    let is_target = Memo::new(move |_| drop_target.get() == Some(target_folder.clone()));
                                                    Some(view! {
                                                        <ParentFolderCard
                                                            on_click=move || current_folder.set(parent_id_click.clone())
                                                            is_drop_target=is_target
                                                            on_drag_enter=move |_| {
                                                                if dragging_item.get().is_some() {
                                                                    drop_target.set(Some(parent_id_enter.clone()));
                                                                }
                                                            }
                                                            on_drag_leave=move |_| drop_target.set(None)
                                                            on_drop=move |_| handle_drop_parent(parent_id_drop.clone())
                                                        />
                                                    })
                                                } else {
                                                    None
                                                }}

                                                {folders_list.into_iter().enumerate().map(|(index, folder)| {
                                                    let folder_id = folder.id.clone();
                                                    let folder_id_click = folder_id.clone();
                                                    let folder_id_drag = folder_id.clone();
                                                    let folder_id_drop_enter = folder_id.clone();
                                                    let folder_id_drop_action = folder_id.clone();
                                                    let folder_id_target = folder_id.clone();
                                                    let folder_id_delete = folder_id.clone();
                                                    let folder_name_delete = folder.name.clone();
                                                    let folder_id_rename = folder_id.clone();
                                                    let folder_name_rename = folder.name.clone();
                                                    let handle_drop_folder = handle_drop_clone;
                                                    let is_target = Memo::new(move |_| drop_target.get() == Some(Some(folder_id_target.clone())));
                                                    view! {
                                                        <FolderCard
                                                            folder=folder
                                                            accent_index=index
                                                            on_click=move || current_folder.set(Some(folder_id_click.clone()))
                                                            is_drop_target=is_target
                                                            on_drag_start=move |_| dragging_item.set(Some(DragItem::Folder(folder_id_drag.clone())))
                                                            on_drag_end=move |_| {
                                                                dragging_item.set(None);
                                                                drop_target.set(None);
                                                            }
                                                            on_drag_enter=move |_| {
                                                                if let Some(item) = dragging_item.get() {
                                                                    if matches!(item, DragItem::Folder(ref id) if id == &folder_id_drop_enter) {
                                                                        return;
                                                                    }
                                                                    drop_target.set(Some(Some(folder_id_drop_enter.clone())));
                                                                }
                                                            }
                                                            on_drag_leave=move |_| drop_target.set(None)
                                                            on_drop=move |_| handle_drop_folder(Some(folder_id_drop_action.clone()))
                                                            on_delete=move || {
                                                                delete_folder_target.set(Some((folder_id_delete.clone(), folder_name_delete.clone())));
                                                                show_delete_folder_modal.set(true);
                                                            }
                                                            on_rename=move || {
                                                                rename_folder_target.set(Some((folder_id_rename.clone(), folder_name_rename.clone())));
                                                                show_rename_folder_modal.set(true);
                                                            }
                                                        />
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </section>
                                    })
                                } else {
                                    None
                                }}

                                <section
                                    on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                                    on:drop=move |ev: web_sys::DragEvent| {
                                        ev.prevent_default();
                                        if drop_target.get().is_none() {
                                            dragging_item.set(None);
                                        }
                                    }
                                >
                                    <div class="dashboard-section-heading">
                                        <span class="eyebrow px-0">{format!("Diagrams · {}", diagrams_count)}</span>
                                        <span>"Sorted by recently updated"</span>
                                    </div>
                                    <div class="dashboard-diagram-grid">
                                        {diagrams_list.into_iter().enumerate().map(|(index, diagram)| {
                                            let diagram_id = diagram.id.clone();
                                            let diagram_id_delete = diagram.id.clone();
                                            let diagram_name_delete = diagram.name.clone();
                                            let diagram_id_rename = diagram.id.clone();
                                            let diagram_name_rename = diagram.name.clone();
                                            view! {
                                                <DiagramCard
                                                    diagram=diagram
                                                    index=index
                                                    on_drag_start=move |_| dragging_item.set(Some(DragItem::Diagram(diagram_id.clone())))
                                                    on_drag_end=move |_| {
                                                        dragging_item.set(None);
                                                        drop_target.set(None);
                                                    }
                                                    on_delete=move || {
                                                        delete_target.set(Some((diagram_id_delete.clone(), diagram_name_delete.clone())));
                                                        show_delete_modal.set(true);
                                                    }
                                                    on_rename=move || {
                                                        rename_target.set(Some((diagram_id_rename.clone(), diagram_name_rename.clone())));
                                                        show_rename_modal.set(true);
                                                    }
                                                />
                                            }
                                        }).collect_view()}

                                        <NewDiagramCard on_click=move || show_new_diagram.set(true) />
                                    </div>
                                </section>

                                {if folders_count == 0 && diagrams_count == 0 {
                                    Some(view! {
                                        <p class="text-center text-sm text-theme-muted">
                                            "Create your first schema diagram or organize your work with a folder."
                                        </p>
                                    })
                                } else {
                                    None
                                }}
                            </div>
                        }.into_any())
                    }
                }}
            </main>

            // New folder modal
            {move || {
                if show_new_folder.get() {
                    Some(view! {
                        <NewFolderModal
                            current_folder=current_folder
                            folders=folders
                            error=error
                            on_close=move || show_new_folder.set(false)
                        />
                    })
                } else {
                    None
                }
            }}

            // New diagram modal
            {move || {
                if show_new_diagram.get() {
                    Some(view! {
                        <NewDiagramModal
                            current_folder=current_folder
                            error=error
                            on_close=move || show_new_diagram.set(false)
                        />
                    })
                } else {
                    None
                }
            }}

            // Delete confirmation modal
            {move || {
                if show_delete_modal.get() {
                    let target = delete_target.get();
                    let diagram_name = target.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
                    Some(view! {
                        <DeleteDiagramModal
                            diagram_name=diagram_name
                            deleting=deleting
                            on_confirm=move || handle_delete()
                            on_close=move || {
                                show_delete_modal.set(false);
                                delete_target.set(None);
                            }
                        />
                    })
                } else {
                    None
                }
            }}

            // Rename modal
            {move || {
                if show_rename_modal.get() {
                    let target = rename_target.get();
                    let current_name = target.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
                    Some(view! {
                        <RenameDiagramModal
                            current_name=current_name
                            renaming=renaming
                            on_confirm=move |new_name| handle_rename(new_name)
                            on_close=move || {
                                show_rename_modal.set(false);
                                rename_target.set(None);
                            }
                        />
                    })
                } else {
                    None
                }
            }}

            // Delete folder confirmation modal
            {move || {
                if show_delete_folder_modal.get() {
                    let target = delete_folder_target.get();
                    let folder_name = target.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
                    Some(view! {
                        <DeleteFolderModal
                            folder_name=folder_name
                            deleting=deleting_folder
                            on_confirm=move || handle_delete_folder()
                            on_close=move || {
                                show_delete_folder_modal.set(false);
                                delete_folder_target.set(None);
                            }
                        />
                    })
                } else {
                    None
                }
            }}

            // Rename folder modal
            {move || {
                if show_rename_folder_modal.get() {
                    let target = rename_folder_target.get();
                    let current_name = target.as_ref().map(|(_, name)| name.clone()).unwrap_or_default();
                    Some(view! {
                        <RenameFolderModal
                            current_name=current_name
                            renaming=renaming_folder
                            on_confirm=move |new_name| handle_rename_folder(new_name)
                            on_close=move || {
                                show_rename_folder_modal.set(false);
                                rename_folder_target.set(None);
                            }
                        />
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

#[component]
fn DashboardUserPill(auth: AuthContext) -> impl IntoView {
    let menu_open = RwSignal::new(false);

    let handle_logout = move |_| {
        menu_open.set(false);
        spawn_local(async move {
            logout().await;
        });
    };

    view! {
        <div class="relative">
            {move || match auth.state.get() {
                AuthState::Loading => view! {
                    <div class="h-8 w-[148px] rounded-full bg-theme-secondary animate-pulse"></div>
                }.into_any(),
                AuthState::Unauthenticated => view! {
                    <div class="hidden items-center gap-2 sm:flex">
                        <A href="/login" attr:class="btn btn-ghost">"Sign in"</A>
                        <A href="/register" attr:class="btn btn-primary">"Start free"</A>
                    </div>
                }.into_any(),
                AuthState::Authenticated(user) => {
                    let display_name = dashboard_user_label(&user);
                    let avatar_name = if user.username.is_empty() { user.email.clone() } else { user.username.clone() };
                    let dropdown_user = user.clone();

                    view! {
                        <button
                            type="button"
                            class="dashboard-user-pill"
                            aria-haspopup="menu"
                            aria-expanded=move || menu_open.get()
                            on:click=move |_| menu_open.update(|open| *open = !*open)
                        >
                            <span class="dashboard-user-name">{display_name}</span>
                            <AvatarInitials name=avatar_name color="oklch(0.55 0.13 285)".to_string() class="av-lg".to_string() />
                        </button>

                        {move || if menu_open.get() {
                            Some(view! {
                                <div class="dropdown-backdrop" on:click=move |_| menu_open.set(false)></div>
                                <div class="dropdown-menu right-0 mt-2 w-56">
                                    <div class="border-b border-theme px-3 py-2">
                                        <p class="truncate text-sm font-semibold text-theme-primary">{dropdown_user.username.clone()}</p>
                                        <p class="truncate text-xs text-theme-muted">{dropdown_user.email.clone()}</p>
                                    </div>
                                    <A href="/profile" attr:class="dropdown-item">
                                        <Icon name=icons::USER class="h-4 w-4" />
                                        "Profile"
                                    </A>
                                    <A href="/dashboard" attr:class="dropdown-item">
                                        <Icon name=icons::GRID class="h-4 w-4" />
                                        "My diagrams"
                                    </A>
                                    <div class="dropdown-separator"></div>
                                    <button type="button" class="dropdown-item dropdown-item-danger" on:click=handle_logout>
                                        <Icon name=icons::LOGOUT class="h-4 w-4" />
                                        "Sign out"
                                    </button>
                                </div>
                            })
                        } else {
                            None
                        }}
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Sort button component
#[component]
fn SortButton(
    option: SortOption,
    current: RwSignal<SortOption>,
    on_select: impl Fn(SortOption) + Send + Sync + 'static,
) -> impl IntoView {
    let is_selected = move || current.get() == option;

    view! {
        <button
            class="dropdown-item justify-between"
            class:text-accent-primary=is_selected
            class:text-theme-primary=move || !is_selected()
            on:click=move |_| on_select(option)
        >
            {option.label()}
            {move || {
                if is_selected() {
                    Some(view! {
                        <Icon name=icons::CHECK class="w-4 h-4" />
                    })
                } else {
                    None
                }
            }}
        </button>
    }
}

/// Parent folder card component (..)
#[component]
fn ParentFolderCard(
    on_click: impl Fn() + Send + Sync + 'static,
    is_drop_target: Memo<bool>,
    on_drag_enter: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drag_leave: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drop: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div
            class="dashboard-folder-card group relative cursor-pointer border-dashed"
            class=("border-theme", move || !is_drop_target.get())
            class=("border-accent-primary", move || is_drop_target.get())
            class=("bg-theme-accent-light", move || is_drop_target.get())
            on:click=move |_| on_click()
            on:dragover=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
            }
            on:dragenter=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                on_drag_enter(ev);
            }
            on:dragleave=move |ev: web_sys::DragEvent| {
                on_drag_leave(ev);
            }
            on:drop=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                on_drop(ev);
            }
        >
            <div class="dashboard-folder-icon text-theme-accent">
                <Icon name=icons::FOLDER class="h-5 w-5" />
            </div>
            <div class="min-w-0 flex-1">
                <h3 class="dashboard-folder-name">"Parent folder"</h3>
                <p class="dashboard-folder-meta">"Up one level"</p>
            </div>
        </div>
    }
}

/// Folder card component with drag-and-drop support
#[component]
fn FolderCard(
    folder: Folder,
    accent_index: usize,
    on_click: impl Fn() + Send + Sync + 'static,
    is_drop_target: Memo<bool>,
    on_drag_start: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drag_end: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drag_enter: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drag_leave: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drop: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + Clone + 'static,
    on_rename: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let folder_name = folder.name.clone();
    let folder_name_display = folder_name.clone();
    let diagrams_count = folder.diagrams_count;
    let diagrams_label = if diagrams_count == 1 {
        "1 diagram".to_string()
    } else {
        format!("{} diagrams", diagrams_count)
    };
    let accent = folder_accent(accent_index);
    let icon_style = format!(
        "background: oklch(0.74 0.17 {} / 0.14); color: oklch(0.74 0.17 {}); border-color: oklch(0.74 0.17 {} / 0.32);",
        accent, accent, accent
    );

    // Menu open state
    let menu_open = RwSignal::new(false);

    let on_delete_click = on_delete.clone();
    let on_rename_click = on_rename.clone();

    view! {
        <div
            class="dashboard-folder-card group relative z-0 cursor-pointer"
            class=("border-theme", move || !is_drop_target.get())
            class=("border-accent-primary", move || is_drop_target.get())
            class=("bg-theme-accent-light", move || is_drop_target.get())
            class=("z-20", move || menu_open.get())
            draggable="true"
            on:click=move |_| {
                on_click()
            }
            on:dragstart=move |ev: web_sys::DragEvent| {
                on_drag_start(ev);
            }
            on:dragend=move |ev: web_sys::DragEvent| {
                on_drag_end(ev);
            }
            on:dragover=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
            }
            on:dragenter=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                on_drag_enter(ev);
            }
            on:dragleave=move |ev: web_sys::DragEvent| {
                on_drag_leave(ev);
            }
            on:drop=move |ev: web_sys::DragEvent| {
                ev.prevent_default();
                on_drop(ev);
            }
        >
            <div class="dashboard-folder-icon" style=icon_style>
                <Icon name=icons::FOLDER class="h-5 w-5" />
            </div>

            <div class="min-w-0 flex-1">
                <h3 class="dashboard-folder-name truncate">{folder_name_display}</h3>
                <p class="dashboard-folder-meta">{diagrams_label}</p>
            </div>

            <div class="dashboard-folder-menu absolute right-2 top-2 z-10">
                <button
                    class="btn-icon btn-sm bg-theme-surface/80 opacity-0 shadow-theme-sm backdrop-blur transition-opacity group-hover:opacity-100"
                    on:click=move |ev| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        menu_open.update(|v| *v = !*v);
                    }
                >
                    <Icon name=icons::ELLIPSIS class="h-4 w-4 text-theme-secondary" />
                </button>

                {move || {
                    if menu_open.get() {
                        let on_rename_menu = on_rename_click.clone();
                        let on_delete_menu = on_delete_click.clone();
                        Some(view! {
                            // Invisible backdrop to close menu when clicking outside
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                    menu_open.set(false);
                                }
                            ></div>
                            <div class="dropdown-menu right-0 mt-1 w-36">
                                <button
                                    class="dropdown-item"
                                    on:click=move |ev| {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        menu_open.set(false);
                                        on_rename_menu();
                                    }
                                >
                                    <Icon name=icons::EDIT class="w-4 h-4" />
                                    "Rename"
                                </button>
                                <button
                                    class="dropdown-item dropdown-item-danger"
                                    on:click=move |ev| {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        menu_open.set(false);
                                        on_delete_menu();
                                    }
                                >
                                    <Icon name=icons::TRASH class="w-4 h-4" />
                                    "Delete"
                                </button>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}

/// Diagram card component with drag support
#[component]
fn DiagramCard(
    diagram: Diagram,
    index: usize,
    on_drag_start: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_drag_end: impl Fn(web_sys::DragEvent) + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + Clone + 'static,
    on_rename: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let diagram_id = diagram.id.clone();
    let diagram_name = diagram.name.clone();
    let diagram_name_title = diagram_name.clone();
    let diagram_description = diagram
        .description
        .clone()
        .unwrap_or_else(|| "Database schema diagram".to_string());
    let diagram_description_title = diagram_description.clone();
    let diagram_updated = diagram.updated_at.clone();
    let is_public = diagram.is_public;
    let seed = diagram_seed(&diagram_id, index);
    let accent = diagram_accent(index, &diagram_id);
    let (tables_count, relations_count) = diagram_stats(&diagram_id);

    // Menu open state
    let menu_open = RwSignal::new(false);

    let formatted_date = format_dashboard_date(&diagram_updated);

    let on_delete_click = on_delete.clone();
    let on_rename_click = on_rename.clone();

    view! {
        <div
            class="file-card dashboard-diagram-card group relative z-0 cursor-pointer"
            class=("z-20", move || menu_open.get())
            draggable="true"
            on:dragstart=move |ev: web_sys::DragEvent| {
                on_drag_start(ev);
            }
            on:dragend=move |ev: web_sys::DragEvent| {
                on_drag_end(ev);
            }
        >
            <A
                href=format!("/editor/{}", diagram_id)
                attr:class="dashboard-diagram-link"
            >
                <div class="file-thumb dashboard-diagram-thumb">
                    <MiniSchemaPreview seed=seed accent=accent density=tables_count />
                    <div class="absolute left-2 top-2 z-10">
                        <ShareBadge is_public=is_public />
                    </div>
                </div>

                <div class="file-meta min-w-0">
                    <h3 class="truncate text-[13.5px] font-semibold tracking-[-0.01em] text-theme-primary transition-colors group-hover:text-accent-primary" title=diagram_name_title>
                        {diagram_name}
                    </h3>
                    <p class="line-clamp-1 text-xs leading-[1.45] text-theme-muted" title=diagram_description_title>
                        {diagram_description}
                    </p>
                    <div class="mt-1 flex items-center gap-2 border-t border-dashed border-theme pt-2">
                        <span class="font-mono text-[11px] text-theme-secondary">
                            <Icon name=icons::DATABASE class="mr-1 inline h-2.5 w-2.5" />
                            {tables_count.to_string()}
                            <Icon name=icons::GIT_BRANCH class="ml-2 mr-1 inline h-2.5 w-2.5" />
                            {relations_count.to_string()}
                        </span>
                        <div class="flex-1"></div>
                        <span class="text-[11px] text-theme-muted">
                            {formatted_date}
                        </span>
                    </div>
                </div>
            </A>

            <div class="absolute right-2 top-2 z-10 flex gap-1">
                <button
                    type="button"
                    class="btn-icon btn-sm bg-theme-surface/80 shadow-theme-sm backdrop-blur"
                    aria-label="Star diagram"
                    on:click=move |ev| {
                        ev.prevent_default();
                        ev.stop_propagation();
                    }
                >
                    <Icon name=icons::STAR class="h-3.5 w-3.5 text-theme-secondary" />
                </button>

                <button
                    type="button"
                    class="btn-icon btn-sm bg-theme-surface/80 shadow-theme-sm backdrop-blur"
                    aria-label="Diagram actions"
                    on:click=move |ev| {
                        ev.prevent_default();
                        ev.stop_propagation();
                        menu_open.update(|v| *v = !*v);
                    }
                >
                    <Icon name=icons::ELLIPSIS class="h-3.5 w-3.5 text-theme-secondary" />
                </button>

                {move || {
                    if menu_open.get() {
                        let on_rename_menu = on_rename_click.clone();
                        let on_delete_menu = on_delete_click.clone();
                        Some(view! {
                            // Invisible backdrop to close menu when clicking outside
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                    menu_open.set(false);
                                }
                            ></div>
                            <div class="dropdown-menu right-0 mt-1 w-36">
                                <button
                                    class="dropdown-item"
                                    on:click=move |ev| {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        menu_open.set(false);
                                        on_rename_menu();
                                    }
                                >
                                    <Icon name=icons::EDIT class="w-4 h-4" />
                                    "Rename"
                                </button>
                                <button
                                    class="dropdown-item dropdown-item-danger"
                                    on:click=move |ev| {
                                        ev.prevent_default();
                                        ev.stop_propagation();
                                        menu_open.set(false);
                                        on_delete_menu();
                                    }
                                >
                                    <Icon name=icons::TRASH class="w-4 h-4" />
                                    "Delete"
                                </button>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn NewDiagramCard(on_click: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    view! {
        <button type="button" class="dashboard-new-card" on:click=move |_| on_click()>
            <span class="dashboard-new-card-icon">
                <Icon name=icons::PLUS class="h-5 w-5" />
            </span>
            <span class="dashboard-new-card-title">"New diagram"</span>
            <span class="dashboard-new-card-subtitle">"Start from scratch, SQL, or template"</span>
        </button>
    }
}

#[component]
fn ShareBadge(is_public: bool) -> impl IntoView {
    if is_public {
        view! {
            <span class="chip dashboard-share-chip dashboard-share-public">
                <Icon name=icons::GLOBE class="h-3 w-3" />
                "Public"
            </span>
        }
        .into_any()
    } else {
        view! {
            <span class="chip dashboard-share-chip dashboard-share-private">
                <Icon name=icons::LOCK class="h-3 w-3" />
                "Private"
            </span>
        }
        .into_any()
    }
}

#[component]
fn MiniSchemaPreview(seed: usize, accent: i32, density: usize) -> impl IntoView {
    let mut boxes = match seed % 5 {
        0 => vec![(40, 32, 32, 28), (94, 18, 44, 36), (164, 74, 32, 18)],
        1 => vec![
            (55, 26, 40, 34),
            (120, 50, 34, 22),
            (152, 34, 44, 28),
            (82, 76, 36, 20),
        ],
        2 => vec![
            (46, 66, 36, 26),
            (102, 42, 42, 30),
            (154, 58, 40, 36),
            (128, 18, 34, 18),
        ],
        3 => vec![
            (32, 44, 36, 22),
            (86, 70, 44, 34),
            (128, 38, 52, 38),
            (166, 76, 36, 20),
        ],
        _ => vec![
            (48, 40, 34, 28),
            (92, 54, 40, 36),
            (126, 24, 46, 34),
            (166, 70, 34, 20),
        ],
    };
    boxes.truncate(density.clamp(3, 6).min(boxes.len()));

    let pattern_id = format!("mini-schema-{}", seed);
    let pattern_fill = format!("url(#{})", pattern_id);
    let accent_stroke = format!("oklch(0.74 0.17 {} / 0.55)", accent);
    let accent_header = format!("oklch(0.74 0.17 {} / 0.68)", accent);
    let connector_paths = boxes
        .windows(2)
        .map(|pair| {
            let (x1, y1, w1, h1) = pair[0];
            let (x2, y2, _w2, h2) = pair[1];
            let start_x = x1 + w1;
            let start_y = y1 + h1 / 2;
            let end_x = x2;
            let end_y = y2 + h2 / 2;
            let mid_x = (start_x + end_x) / 2;
            format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start_x, start_y, mid_x, start_y, mid_x, end_y, end_x, end_y
            )
        })
        .collect::<Vec<_>>();

    view! {
        <svg viewBox="0 0 220 116" class="h-full w-full" role="img" aria-label="Diagram preview">
            <defs>
                <pattern id=pattern_id width="10" height="10" patternUnits="userSpaceOnUse">
                    <circle cx="1" cy="1" r="0.6" fill="var(--grid)" />
                </pattern>
            </defs>
            <rect width="220" height="116" fill="var(--canvas)" />
            <rect width="220" height="116" fill=pattern_fill />

            {connector_paths.into_iter().map(|path| {
                view! {
                    <path d=path stroke=accent_stroke.clone() stroke-width="1" fill="none" />
                }
            }).collect_view()}

            {boxes.into_iter().enumerate().map(|(box_index, (x, y, width, height))| {
                let line_count = ((height / 5) as usize).clamp(2, 4);
                let header_fill = accent_header.clone();
                view! {
                    <g>
                        <rect
                            x=x.to_string()
                            y=y.to_string()
                            width=width.to_string()
                            height=height.to_string()
                            rx="2"
                            fill="var(--card)"
                            stroke="var(--border-strong)"
                        />
                        <rect
                            x=x.to_string()
                            y=y.to_string()
                            width=width.to_string()
                            height="4"
                            rx="2"
                            fill=header_fill
                        />
                        {(0..line_count).map(move |line_index| {
                            let line_y = y + 7 + (line_index as i32 * 4);
                            let line_width = width - 6 - ((box_index + line_index) as i32 % 3) * 4;
                            view! {
                                <rect
                                    x=(x + 3).to_string()
                                    y=line_y.to_string()
                                    width=line_width.max(8).to_string()
                                    height="1.5"
                                    rx="0.5"
                                    fill="var(--border-strong)"
                                />
                            }
                        }).collect_view()}
                    </g>
                }
            }).collect_view()}
        </svg>
    }
}

fn dashboard_user_label(user: &User) -> String {
    if !user.username.trim().is_empty() {
        user.username.clone()
    } else {
        user.email.split('@').next().unwrap_or("user").to_string()
    }
}

fn folder_accent(index: usize) -> i32 {
    [155, 235, 60][index % 3]
}

fn diagram_seed(id: &str, index: usize) -> usize {
    id.bytes()
        .fold(index.wrapping_mul(31).wrapping_add(7), |acc, byte| {
            acc.wrapping_mul(33).wrapping_add(byte as usize)
        })
}

fn diagram_accent(index: usize, id: &str) -> i32 {
    let accents = [155, 235, 60, 305, 195, 25];
    accents[diagram_seed(id, index) % accents.len()]
}

fn diagram_stats(id: &str) -> (usize, usize) {
    let seed = diagram_seed(id, 0);
    let tables = 4 + seed % 6;
    let relations = tables + (seed / 7) % 5;
    (tables, relations)
}

fn format_dashboard_date(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "recently".to_string();
    }

    if let Some(date_part) = raw.split('T').next()
        && date_part.len() == 10
    {
        return date_part.to_string();
    }

    raw.to_string()
}

/// Delete confirmation modal
#[component]
fn DeleteDiagramModal(
    diagram_name: String,
    deleting: RwSignal<bool>,
    on_confirm: impl Fn() + Send + Sync + Clone + 'static,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let on_close_cancel = on_close.clone();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div
                class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                on:click=move |_| on_close_cancel()
            ></div>

            // Modal
            <div class="relative bg-theme-primary rounded-xl shadow-2xl border border-theme p-6 w-full max-w-md mx-4">
                <div class="flex items-start gap-4">
                    <div class="flex-shrink-0 w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
                        <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                    </div>
                    <div class="flex-1">
                        <h3 class="text-lg font-semibold text-theme-primary">"Delete Diagram"</h3>
                        <p class="mt-2 text-sm text-theme-secondary">
                            "Are you sure you want to delete "
                            <span class="font-medium text-theme-primary">"\""{ diagram_name }"\""</span>
                            "? This action cannot be undone."
                        </p>
                    </div>
                </div>

                <div class="mt-6 flex justify-end gap-3">
                    <button
                        class="px-4 py-2 text-sm font-medium text-theme-secondary border border-theme
                               rounded-lg hover:bg-theme-secondary transition-colors"
                        on:click=move |_| on_close()
                        disabled=move || deleting.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium text-white bg-red-600
                               hover:bg-red-700 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
                        on:click=move |_| on_confirm()
                        disabled=move || deleting.get()
                    >
                        {move || {
                            if deleting.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    "Deleting..."
                                }.into_any()
                            } else {
                                view! { "Delete" }.into_any()
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Rename diagram modal
#[component]
fn RenameDiagramModal(
    current_name: String,
    renaming: RwSignal<bool>,
    on_confirm: impl Fn(String) + Send + Sync + Clone + 'static,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let new_name = RwSignal::new(current_name);
    let local_error = RwSignal::new(None::<String>);

    let on_close_cancel = on_close.clone();
    let on_close_backdrop = on_close.clone();
    let on_confirm_submit = on_confirm.clone();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let value = new_name.get().trim().to_string();
        if value.is_empty() {
            local_error.set(Some("Name is required".to_string()));
            return;
        }
        if value.len() > 255 {
            local_error.set(Some("Name is too long (max 255 characters)".to_string()));
            return;
        }
        on_confirm_submit(value);
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div
                class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            ></div>

            // Modal
            <form
                class="relative bg-theme-primary rounded-xl shadow-2xl border border-theme p-6 w-full max-w-md mx-4"
                on:submit=on_submit
            >
                <h3 class="text-lg font-semibold text-theme-primary mb-4">"Rename Diagram"</h3>

                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-theme-secondary mb-1">"Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 bg-theme-secondary border border-theme rounded-lg
                                   text-theme-primary placeholder-theme-tertiary
                                   focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
                            prop:value=move || new_name.get()
                            on:input=move |ev| {
                                new_name.set(event_target_value(&ev));
                                local_error.set(None);
                            }
                            autofocus
                        />
                    </div>

                    {move || local_error.get().map(|err| view! {
                        <p class="text-sm text-red-500">{err}</p>
                    })}
                </div>

                <div class="mt-6 flex justify-end gap-3">
                    <button
                        type="button"
                        class="px-4 py-2 text-sm font-medium text-theme-secondary border border-theme
                               rounded-lg hover:bg-theme-secondary transition-colors"
                        on:click=move |_| on_close_cancel()
                        disabled=move || renaming.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        type="submit"
                        class="px-4 py-2 text-sm font-medium text-white bg-accent-primary
                               hover:bg-accent-primary-hover rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
                        disabled=move || renaming.get()
                    >
                        {move || {
                            if renaming.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    "Saving..."
                                }.into_any()
                            } else {
                                view! { "Save" }.into_any()
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Delete folder confirmation modal
#[component]
fn DeleteFolderModal(
    folder_name: String,
    deleting: RwSignal<bool>,
    on_confirm: impl Fn() + Send + Sync + Clone + 'static,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let on_close_cancel = on_close.clone();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div
                class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                on:click=move |_| on_close_cancel()
            ></div>

            // Modal
            <div class="relative bg-theme-primary rounded-xl shadow-2xl border border-theme p-6 w-full max-w-md mx-4">
                <div class="flex items-start gap-4">
                    <div class="flex-shrink-0 w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
                        <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                    </div>
                    <div class="flex-1">
                        <h3 class="text-lg font-semibold text-theme-primary">"Delete Folder"</h3>
                        <p class="mt-2 text-sm text-theme-secondary">
                            "Are you sure you want to delete "
                            <span class="font-medium text-theme-primary">"\""{ folder_name }"\""</span>
                            "? All diagrams inside will be moved to root. This action cannot be undone."
                        </p>
                    </div>
                </div>

                <div class="mt-6 flex justify-end gap-3">
                    <button
                        class="px-4 py-2 text-sm font-medium text-theme-secondary border border-theme
                               rounded-lg hover:bg-theme-secondary transition-colors"
                        on:click=move |_| on_close()
                        disabled=move || deleting.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 text-sm font-medium text-white bg-red-600
                               hover:bg-red-700 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
                        on:click=move |_| on_confirm()
                        disabled=move || deleting.get()
                    >
                        {move || {
                            if deleting.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    "Deleting..."
                                }.into_any()
                            } else {
                                view! { "Delete" }.into_any()
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Rename folder modal
#[component]
fn RenameFolderModal(
    current_name: String,
    renaming: RwSignal<bool>,
    on_confirm: impl Fn(String) + Send + Sync + Clone + 'static,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let new_name = RwSignal::new(current_name);
    let local_error = RwSignal::new(None::<String>);

    let on_close_cancel = on_close.clone();
    let on_close_backdrop = on_close.clone();
    let on_confirm_submit = on_confirm.clone();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let value = new_name.get().trim().to_string();
        if value.is_empty() {
            local_error.set(Some("Name is required".to_string()));
            return;
        }
        if value.len() > 255 {
            local_error.set(Some("Name is too long (max 255 characters)".to_string()));
            return;
        }
        on_confirm_submit(value);
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            // Backdrop
            <div
                class="absolute inset-0 bg-black/50 backdrop-blur-sm"
                on:click=move |_| on_close_backdrop()
            ></div>

            // Modal
            <form
                class="relative bg-theme-primary rounded-xl shadow-2xl border border-theme p-6 w-full max-w-md mx-4"
                on:submit=on_submit
            >
                <h3 class="text-lg font-semibold text-theme-primary mb-4">"Rename Folder"</h3>

                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-theme-secondary mb-1">"Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 bg-theme-secondary border border-theme rounded-lg
                                   text-theme-primary placeholder-theme-tertiary
                                   focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
                            prop:value=move || new_name.get()
                            on:input=move |ev| {
                                new_name.set(event_target_value(&ev));
                                local_error.set(None);
                            }
                            autofocus
                        />
                    </div>

                    {move || local_error.get().map(|err| view! {
                        <p class="text-sm text-red-500">{err}</p>
                    })}
                </div>

                <div class="mt-6 flex justify-end gap-3">
                    <button
                        type="button"
                        class="px-4 py-2 text-sm font-medium text-theme-secondary border border-theme
                               rounded-lg hover:bg-theme-secondary transition-colors"
                        on:click=move |_| on_close_cancel()
                        disabled=move || renaming.get()
                    >
                        "Cancel"
                    </button>
                    <button
                        type="submit"
                        class="px-4 py-2 text-sm font-medium text-white bg-accent-primary
                               hover:bg-accent-primary-hover rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
                        disabled=move || renaming.get()
                    >
                        {move || {
                            if renaming.get() {
                                view! {
                                    <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    "Saving..."
                                }.into_any()
                            } else {
                                view! { "Save" }.into_any()
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

/// Modal for creating a new diagram with custom name
#[component]
fn NewDiagramModal(
    current_folder: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let name = RwSignal::new(String::from("Untitled Diagram"));
    let local_error = RwSignal::new(None::<String>);
    let creating = RwSignal::new(false);

    let on_close_clone = on_close.clone();
    let on_close_cancel = on_close.clone();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let value = name.get().trim().to_string();
        if value.is_empty() {
            local_error.set(Some("Diagram name is required".to_string()));
            return;
        }

        if value.len() > 255 {
            local_error.set(Some(
                "Diagram name is too long (max 255 characters)".to_string(),
            ));
            return;
        }

        creating.set(true);
        let folder_id = current_folder.get();
        let on_close = on_close.clone();

        spawn_local(async move {
            match create_new_diagram_with_name(&value, folder_id).await {
                Ok(diagram) => {
                    let nav = use_navigate();
                    nav(&format!("/editor/{}", diagram.id), Default::default());
                    on_close();
                }
                Err(e) => {
                    error.set(Some(e));
                    creating.set(false);
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" on:click=move |_| on_close_clone()></div>
            <div class="relative w-full max-w-md bg-theme-primary rounded-xl shadow-xl p-6 border border-theme">
                <h2 class="text-xl font-bold text-theme-primary mb-6">"Create New Diagram"</h2>

                <form on:submit=on_submit>
                    <div class="mb-6">
                        <label class="block text-sm font-medium text-theme-primary mb-2">"Diagram Name"</label>
                        <input
                            type="text"
                            placeholder="My Database Schema"
                            class="w-full px-3 py-2 bg-theme-secondary border border-theme rounded-lg
                                   text-theme-primary placeholder-theme-tertiary
                                   focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
                            class:border-red-500=move || local_error.get().is_some()
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                name.set(event_target_value(&ev));
                                local_error.set(None);
                            }
                            autofocus
                        />
                        {move || local_error.get().map(|e| view! { <p class="mt-1 text-sm text-red-500">{e}</p> })}
                    </div>

                    <div class="flex justify-end gap-3">
                        <button
                            type="button"
                            class="px-4 py-2 text-sm font-medium text-theme-secondary hover:text-theme-primary transition-colors"
                            on:click=move |_| on_close_cancel()
                            disabled=move || creating.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="px-4 py-2 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary-hover rounded-lg transition-colors disabled:opacity-50"
                            disabled=move || creating.get()
                        >
                            {move || if creating.get() { "Creating..." } else { "Create" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
fn NewFolderModal(
    current_folder: RwSignal<Option<String>>,
    folders: RwSignal<Vec<Folder>>,
    error: RwSignal<Option<String>>,
    on_close: impl Fn() + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let local_error = RwSignal::new(None::<String>);

    let on_close_clone = on_close.clone();

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let value = name.get().trim().to_string();
        if value.is_empty() {
            local_error.set(Some("Folder name is required".to_string()));
            return;
        }

        let parent_id = current_folder.get();
        let on_close = on_close.clone();

        spawn_local(async move {
            match create_new_folder(&value, parent_id).await {
                Ok(folder) => {
                    folders.update(|f| f.push(folder));
                    on_close();
                }
                Err(e) => {
                    error.set(Some(e));
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" on:click=move |_| on_close_clone()></div>
            <div class="relative w-full max-w-md bg-theme-primary rounded-xl shadow-xl p-6 border border-theme">
                <h2 class="text-xl font-bold text-theme-primary mb-6">"Create New Folder"</h2>

                <form on:submit=on_submit>
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-theme-primary mb-1">"Folder Name"</label>
                        <input
                            type="text"
                            placeholder="My Folder"
                            class="w-full px-3 py-2 bg-theme-secondary border border-theme rounded-lg
                                   text-theme-primary placeholder-theme-tertiary
                                   focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
                            class:border-red-500=move || local_error.get().is_some()
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                name.set(event_target_value(&ev));
                                local_error.set(None);
                            }
                        />
                        {move || local_error.get().map(|e| view! { <p class="mt-1 text-sm text-red-500">{e}</p> })}
                    </div>

                    <div class="flex justify-end gap-3">
                        <button
                            type="submit"
                            class="px-4 py-2 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary-hover rounded-lg transition-colors"
                        >
                            "Create"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

// API functions

#[cfg(not(feature = "ssr"))]
async fn fetch_folders(parent_id: Option<String>) -> Result<Vec<Folder>, ApiError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or(ApiError::Unauthorized)?;

    let url = match parent_id {
        Some(id) => format!("/api/folders/nodes?parent_id={}", id),
        None => "/api/folders/nodes".to_string(),
    };

    let window = web_sys::window().ok_or(ApiError::Other("No window".to_string()))?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    if !resp.ok() {
        let status = resp.status();
        if status == 401 {
            return Err(ApiError::Unauthorized);
        }
        return Err(ApiError::Other("Failed to fetch folders".to_string()));
    }

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| ApiError::Other(format!("{:?}", e)))?,
    )
    .await
    .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let response: FolderListResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| ApiError::Other(e.to_string()))?;
    Ok(response.folders)
}

#[cfg(feature = "ssr")]
async fn fetch_folders(_parent_id: Option<String>) -> Result<Vec<Folder>, ApiError> {
    Ok(vec![])
}

#[cfg(not(feature = "ssr"))]
async fn fetch_folder_info(folder_id: Option<String>) -> Result<Option<Folder>, ApiError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let Some(folder_id) = folder_id else {
        return Ok(None);
    };

    let auth = use_auth_context();
    let token = auth.access_token().ok_or(ApiError::Unauthorized)?;

    let url = format!("/api/folders/{}", folder_id);

    let window = web_sys::window().ok_or(ApiError::Other("No window".to_string()))?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    if !resp.ok() {
        let status = resp.status();
        if status == 401 {
            return Err(ApiError::Unauthorized);
        }
        return Err(ApiError::Other("Failed to fetch folder info".to_string()));
    }

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| ApiError::Other(format!("{:?}", e)))?,
    )
    .await
    .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let folder: Folder =
        serde_wasm_bindgen::from_value(json).map_err(|e| ApiError::Other(e.to_string()))?;
    Ok(Some(folder))
}

#[cfg(feature = "ssr")]
async fn fetch_folder_info(_folder_id: Option<String>) -> Result<Option<Folder>, ApiError> {
    Ok(None)
}

#[cfg(not(feature = "ssr"))]
async fn fetch_folder_path(folder_id: String) -> Result<Vec<Folder>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let url = format!("/api/folders/{}/path", folder_id);

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Ok(vec![]);
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    #[derive(Debug, Deserialize)]
    struct FolderPathResponse {
        path: Vec<Folder>,
    }

    let response: FolderPathResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;
    Ok(response.path)
}

#[cfg(feature = "ssr")]
async fn fetch_folder_path(_folder_id: String) -> Result<Vec<Folder>, String> {
    Ok(vec![])
}

#[cfg(not(feature = "ssr"))]
async fn fetch_diagrams(folder_id: Option<String>) -> Result<Vec<Diagram>, ApiError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or(ApiError::Unauthorized)?;

    let url = match folder_id {
        Some(id) => format!("/api/diagrams?folder_id={}", id),
        None => "/api/diagrams".to_string(),
    };

    let window = web_sys::window().ok_or(ApiError::Other("No window".to_string()))?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    if !resp.ok() {
        let status = resp.status();
        if status == 401 {
            return Err(ApiError::Unauthorized);
        }
        return Err(ApiError::Other("Failed to fetch diagrams".to_string()));
    }

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| ApiError::Other(format!("{:?}", e)))?,
    )
    .await
    .map_err(|e| ApiError::Other(format!("{:?}", e)))?;

    let response: DiagramListResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| ApiError::Other(e.to_string()))?;
    Ok(response.diagrams)
}

#[cfg(feature = "ssr")]
async fn fetch_diagrams(_folder_id: Option<String>) -> Result<Vec<Diagram>, ApiError> {
    Ok(vec![])
}

#[cfg(not(feature = "ssr"))]
async fn create_new_diagram_with_name(
    name: &str,
    folder_id: Option<String>,
) -> Result<Diagram, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "name": name,
        "folder_id": folder_id,
        "schema_data": {}
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&body.to_string().into());

    let req =
        Request::new_with_str_and_init("/api/diagrams", &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to create diagram".to_string());
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

#[cfg(feature = "ssr")]
async fn create_new_diagram_with_name(
    _name: &str,
    _folder_id: Option<String>,
) -> Result<Diagram, String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn create_new_folder(name: &str, parent_id: Option<String>) -> Result<Folder, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "name": name,
        "parent_id": parent_id
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&body.to_string().into());

    let req =
        Request::new_with_str_and_init("/api/folders", &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to create folder".to_string());
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())
}

#[cfg(feature = "ssr")]
async fn create_new_folder(_name: &str, _parent_id: Option<String>) -> Result<Folder, String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn move_diagram_to_folder(diagram_id: &str, folder_id: Option<String>) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    // For Option<Option<Uuid>> on server:
    // - Not present in JSON = don't update
    // - Present as null = set to NULL (move to root)
    // - Present as string = set to that UUID
    // We always want to update, so we always include folder_id
    let body = match folder_id {
        Some(id) => serde_json::json!({ "folder_id": id }),
        None => serde_json::json!({ "folder_id": null }),
    };

    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_body(&body.to_string().into());

    let url = format!("/api/diagrams/{}", diagram_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to move diagram".to_string());
    }

    Ok(())
}

#[cfg(not(feature = "ssr"))]
async fn delete_diagram(diagram_id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("DELETE");

    let url = format!("/api/diagrams/{}", diagram_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to delete diagram".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn delete_diagram(_diagram_id: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn rename_diagram(diagram_id: &str, new_name: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({ "name": new_name });

    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_body(&body.to_string().into());

    let url = format!("/api/diagrams/{}", diagram_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to rename diagram".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn rename_diagram(_diagram_id: &str, _new_name: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

#[cfg(feature = "ssr")]
async fn move_diagram_to_folder(
    _diagram_id: &str,
    _folder_id: Option<String>,
) -> Result<(), String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn move_folder_to_parent(folder_id: &str, parent_id: Option<String>) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "parent_id": parent_id
    });

    let opts = RequestInit::new();
    opts.set_method("PATCH");
    opts.set_body(&body.to_string().into());

    let url = format!("/api/folders/{}/move", folder_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to move folder".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn move_folder_to_parent(_folder_id: &str, _parent_id: Option<String>) -> Result<(), String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn delete_folder(folder_id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("DELETE");

    let url = format!("/api/folders/{}", folder_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to delete folder".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn delete_folder(_folder_id: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}

#[cfg(not(feature = "ssr"))]
async fn rename_folder(folder_id: &str, new_name: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let auth = use_auth_context();
    let token = auth.access_token().ok_or("Not authenticated")?;

    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({ "name": new_name });

    let opts = RequestInit::new();
    opts.set_method("PUT");
    opts.set_body(&body.to_string().into());

    let url = format!("/api/folders/{}", folder_id);
    let req = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", token))
        .map_err(|e| format!("{:?}", e))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to rename folder".to_string());
    }

    Ok(())
}

#[cfg(feature = "ssr")]
async fn rename_folder(_folder_id: &str, _new_name: &str) -> Result<(), String> {
    Err("Not available on server".to_string())
}
