//! Authentication utilities for frontend
//!
//! Provides helper functions for managing JWT tokens stored in localStorage

use serde::{Deserialize, Serialize};

/// Storage key for tokens in localStorage
#[allow(dead_code)]
const STORAGE_KEY_TOKENS: &str = "archischema_tokens";

/// Token pair structure matching the one used in auth/context.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_expires_at: Option<i64>,
}

/// Get tokens from localStorage
#[cfg(not(feature = "ssr"))]
fn get_tokens() -> Option<TokenPair> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let tokens_json = storage.get_item(STORAGE_KEY_TOKENS).ok()??;
    serde_json::from_str(&tokens_json).ok()
}

/// Get the JWT access token from localStorage
///
/// Returns None if localStorage is not available or token doesn't exist
#[cfg(not(feature = "ssr"))]
pub fn get_access_token() -> Option<String> {
    get_tokens().map(|t| t.access_token)
}

/// Get the JWT refresh token from localStorage
///
/// Returns None if localStorage is not available or token doesn't exist
#[cfg(not(feature = "ssr"))]
pub fn get_refresh_token() -> Option<String> {
    get_tokens().map(|t| t.refresh_token)
}

/// Set the JWT access token in localStorage
/// Note: This updates the entire token pair object
#[cfg(not(feature = "ssr"))]
pub fn set_access_token(token: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window available")?;
    let storage = window
        .local_storage()
        .map_err(|_| "Failed to get localStorage")?
        .ok_or("localStorage not available")?;

    let mut tokens = get_tokens().unwrap_or(TokenPair {
        access_token: String::new(),
        refresh_token: String::new(),
        access_expires_at: None,
        refresh_expires_at: None,
    });
    tokens.access_token = token.to_string();

    storage
        .set_item(
            STORAGE_KEY_TOKENS,
            &serde_json::to_string(&tokens).unwrap_or_default(),
        )
        .map_err(|_| "Failed to set tokens")?;
    Ok(())
}

/// Set the JWT refresh token in localStorage
/// Note: This updates the entire token pair object
#[cfg(not(feature = "ssr"))]
pub fn set_refresh_token(token: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window available")?;
    let storage = window
        .local_storage()
        .map_err(|_| "Failed to get localStorage")?
        .ok_or("localStorage not available")?;

    let mut tokens = get_tokens().unwrap_or(TokenPair {
        access_token: String::new(),
        refresh_token: String::new(),
        access_expires_at: None,
        refresh_expires_at: None,
    });
    tokens.refresh_token = token.to_string();

    storage
        .set_item(
            STORAGE_KEY_TOKENS,
            &serde_json::to_string(&tokens).unwrap_or_default(),
        )
        .map_err(|_| "Failed to set tokens")?;
    Ok(())
}

/// Remove authentication tokens from localStorage
#[cfg(not(feature = "ssr"))]
pub fn clear_tokens() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window available")?;
    let storage = window
        .local_storage()
        .map_err(|_| "Failed to get localStorage")?
        .ok_or("localStorage not available")?;
    storage
        .remove_item(STORAGE_KEY_TOKENS)
        .map_err(|_| "Failed to remove tokens")?;
    Ok(())
}

/// Add Authorization header with JWT token to a web_sys::Request
///
/// If token is available in localStorage, adds "Authorization: Bearer <token>" header
#[cfg(not(feature = "ssr"))]
pub fn add_auth_header(request: &web_sys::Request) -> Result<(), String> {
    if let Some(token) = get_access_token() {
        request
            .headers()
            .set("Authorization", &format!("Bearer {}", token))
            .map_err(|_| "Failed to set Authorization header")?;
    }
    Ok(())
}

/// Check if user is authenticated (has valid access token)
#[cfg(not(feature = "ssr"))]
pub fn is_authenticated() -> bool {
    get_access_token().is_some()
}

/// SSR stubs - these functions do nothing on the server
#[cfg(feature = "ssr")]
pub fn get_access_token() -> Option<String> {
    None
}

#[cfg(feature = "ssr")]
pub fn get_refresh_token() -> Option<String> {
    None
}

#[cfg(feature = "ssr")]
pub fn set_access_token(_token: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn set_refresh_token(_token: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn clear_tokens() -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn add_auth_header<T>(_request: &T) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "ssr")]
pub fn is_authenticated() -> bool {
    false
}
