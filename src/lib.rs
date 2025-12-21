//! Archischema - Database Schema Diagram Editor
//!
//! A modern web application for creating and visualizing database schema diagrams,
//! built with Leptos and WebAssembly.

#![recursion_limit = "4096"]

pub mod app;
pub mod core;
pub mod ui;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
