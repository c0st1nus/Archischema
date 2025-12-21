//! Application pages module
//!
//! This module contains all the page components for the application:
//! - Landing page (home)
//! - Login page
//! - Register page
//! - Dashboard (diagrams list)
//! - Editor page
//! - Profile page

mod dashboard;
mod editor;
mod landing;
mod login;
mod not_found;
mod profile;
mod register;

pub use dashboard::DashboardPage;
pub use editor::EditorPage;
pub use landing::LandingPage;
pub use login::LoginPage;
pub use not_found::NotFoundPage;
pub use profile::ProfilePage;
pub use register::RegisterPage;
