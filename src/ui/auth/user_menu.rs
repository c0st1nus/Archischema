//! User menu component
//!
//! A dropdown menu component that displays user info and actions
//! in the header. Shows login/register links when not authenticated,
//! or user avatar and menu when authenticated.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use super::context::{AuthState, User, logout, use_auth_context};
use crate::ui::icon::{Icon, icons};

/// User menu component for the header
#[component]
pub fn UserMenu(
    /// Callback when login button is clicked (optional, uses navigation if not provided)
    #[prop(optional, into)]
    on_login_click: Option<Callback<()>>,
    /// Callback when register button is clicked (optional, uses navigation if not provided)
    #[prop(optional, into)]
    on_register_click: Option<Callback<()>>,
) -> impl IntoView {
    let auth = use_auth_context();

    // Dropdown open state
    let menu_open = RwSignal::new(false);

    // Close menu when clicking outside
    let menu_ref = NodeRef::<leptos::html::Div>::new();

    // Handle logout
    let handle_logout = move |_| {
        menu_open.set(false);
        spawn_local(async move {
            logout().await;
        });
    };

    view! {
        <div class="relative" node_ref=menu_ref>
            {move || {
                match auth.state.get() {
                    AuthState::Loading => {
                        // Loading skeleton
                        view! {
                            <div class="w-8 h-8 rounded-full bg-theme-secondary animate-pulse"></div>
                        }.into_any()
                    }
                    AuthState::Unauthenticated => {
                        // Login/Register buttons or links
                        let has_login_callback = on_login_click.is_some();
                        let has_register_callback = on_register_click.is_some();

                        view! {
                            <div class="flex items-center gap-2">
                                {if has_login_callback {
                                    view! {
                                        <button
                                            class="px-3 py-1.5 text-sm font-medium text-theme-secondary hover:text-theme-primary
                                                   transition-colors"
                                            on:click=move |_| {
                                                if let Some(callback) = on_login_click.as_ref() {
                                                    callback.run(());
                                                }
                                            }
                                        >
                                            "Sign In"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <A
                                            href="/login"
                                            attr:class="px-3 py-1.5 text-sm font-medium text-theme-secondary hover:text-theme-primary transition-colors"
                                        >
                                            "Sign In"
                                        </A>
                                    }.into_any()
                                }}
                                {if has_register_callback {
                                    view! {
                                        <button
                                            class="px-3 py-1.5 text-sm font-medium text-white bg-accent-primary
                                                   hover:bg-accent-primary-hover rounded-lg transition-colors"
                                            on:click=move |_| {
                                                if let Some(callback) = on_register_click.as_ref() {
                                                    callback.run(());
                                                }
                                            }
                                        >
                                            "Sign Up"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <A
                                            href="/register"
                                            attr:class="px-3 py-1.5 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary-hover rounded-lg transition-colors"
                                        >
                                            "Sign Up"
                                        </A>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                    AuthState::Authenticated(user) => {
                        // User avatar and dropdown
                        view! {
                            <div class="relative">
                                <button
                                    class="flex items-center gap-2 p-1 rounded-lg hover:bg-theme-secondary transition-colors"
                                    on:click=move |_| menu_open.update(|v| *v = !*v)
                                >
                                    <UserAvatar user=user.clone() size=32 />
                                    <span class="hidden sm:block text-sm font-medium text-theme-primary max-w-[120px] truncate">
                                        {user.username.clone()}
                                    </span>
                                    <div class="flex items-center justify-center h-4 w-4 text-theme-tertiary transition-transform duration-200" class=("rotate-180", move || menu_open.get())>
                                        <Icon name=icons::CHEVRON_DOWN class="h-4 w-4" />
                                    </div>
                                </button>

                                // Dropdown menu
                                {move || {
                                    if menu_open.get() {
                                        let user_clone = user.clone();
                                        Some(view! {
                                            <div class="absolute right-0 mt-2 w-56 bg-theme-primary rounded-lg shadow-lg border border-theme py-1 z-50">
                                                // User info header
                                                <div class="px-4 py-3 border-b border-theme">
                                                    <p class="text-sm font-medium text-theme-primary truncate">
                                                        {user_clone.username.clone()}
                                                    </p>
                                                    <p class="text-xs text-theme-tertiary truncate">
                                                        {user_clone.email.clone()}
                                                    </p>
                                                </div>

                                                // Menu items
                                                <div class="py-1">
                                                    <A
                                                        href="/profile"
                                                        attr:class="w-full px-4 py-2 text-sm text-left text-theme-primary
                                                               hover:bg-theme-secondary transition-colors flex items-center gap-2"
                                                    >
                                                        <Icon name=icons::USER class="h-4 w-4" />
                                                        "Profile"
                                                    </A>
                                                    <A
                                                        href="/dashboard"
                                                        attr:class="w-full px-4 py-2 text-sm text-left text-theme-primary
                                                               hover:bg-theme-secondary transition-colors flex items-center gap-2"
                                                    >
                                                        <Icon name=icons::SQUARES_2X2 class="h-4 w-4" />
                                                        "My Diagrams"
                                                    </A>
                                                </div>

                                                // Divider
                                                <div class="border-t border-theme my-1"></div>

                                                // Logout
                                                <div class="py-1">
                                                    <button
                                                        class="w-full px-4 py-2 text-sm text-left text-red-500
                                                               hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors
                                                               flex items-center gap-2"
                                                        on:click=handle_logout
                                                    >
                                                        <Icon name=icons::LOGOUT class="h-4 w-4" />
                                                        "Sign Out"
                                                    </button>
                                                </div>
                                            </div>
                                        })
                                    } else {
                                        None
                                    }
                                }}
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

/// User avatar component
#[component]
pub fn UserAvatar(
    /// User data
    user: User,
    /// Avatar size in pixels
    #[prop(default = 32)]
    size: u32,
) -> impl IntoView {
    let initials = {
        let first = user
            .username
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        first
    };

    let size_style = format!(
        "width: {}px; height: {}px; min-width: {}px; min-height: {}px;",
        size, size, size, size
    );
    let font_size = if size >= 40 { "text-lg" } else { "text-sm" };

    if let Some(avatar_url) = &user.avatar_url {
        view! {
            <img
                src=avatar_url.clone()
                alt=format!("{}'s avatar", user.username)
                class="rounded-full object-cover"
                style=size_style
            />
        }
        .into_any()
    } else {
        // Generate a consistent color from the username
        let hash = user
            .username
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let colors = [
            "bg-blue-500",
            "bg-green-500",
            "bg-yellow-500",
            "bg-red-500",
            "bg-purple-500",
            "bg-pink-500",
            "bg-indigo-500",
            "bg-teal-500",
        ];
        let color = colors[(hash as usize) % colors.len()];

        view! {
            <div
                class=format!("{} rounded-full flex items-center justify-center text-white font-medium {}", color, font_size)
                style=size_style
            >
                {initials}
            </div>
        }
        .into_any()
    }
}
