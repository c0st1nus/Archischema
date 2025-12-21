//! Authentication UI module
//!
//! This module provides authentication-related components and context
//! for the Archischema frontend.

mod context;
mod login_form;
mod register_form;
mod user_menu;

pub use context::{AuthContext, AuthState, User, logout, provide_auth_context, use_auth_context};
pub use login_form::LoginForm;
pub use register_form::RegisterForm;
pub use user_menu::UserMenu;
