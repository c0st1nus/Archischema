//! Auth context for managing user authentication state
//!
//! This module provides a reactive authentication context that:
//! - Stores the current user and authentication tokens
//! - Handles login, logout, registration flows
//! - Automatically refreshes tokens
//! - Persists auth state to localStorage

use leptos::prelude::*;
#[cfg(not(feature = "ssr"))]
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

/// User information from the API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

/// Token pair from auth API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: i64,
    pub refresh_expires_at: i64,
}

/// Authentication state
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AuthState {
    /// Initial state, checking localStorage
    #[default]
    Loading,
    /// User is not authenticated
    Unauthenticated,
    /// User is authenticated
    Authenticated(User),
}

/// Auth error types
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthApiError {
    pub error: String,
    pub code: String,
}

/// Auth context providing authentication state and actions
#[derive(Clone, Copy)]
pub struct AuthContext {
    /// Current authentication state
    pub state: RwSignal<AuthState>,
    /// Current tokens (if authenticated)
    tokens: RwSignal<Option<TokenPair>>,
    /// Loading state for auth operations
    pub loading: RwSignal<bool>,
    /// Error message from last operation
    pub error: RwSignal<Option<String>>,
}

impl AuthContext {
    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self.state.get(), AuthState::Authenticated(_))
    }

    /// Get current user (if authenticated)
    pub fn user(&self) -> Option<User> {
        match self.state.get() {
            AuthState::Authenticated(user) => Some(user),
            _ => None,
        }
    }

    /// Get access token (if authenticated)
    /// Uses get_untracked() since this is typically called outside reactive contexts
    pub fn access_token(&self) -> Option<String> {
        self.tokens.get_untracked().map(|t| t.access_token)
    }

    /// Clear error message
    pub fn clear_error(&self) {
        self.error.set(None);
    }
}

#[allow(dead_code)]
const STORAGE_KEY_TOKENS: &str = "archischema_tokens";
#[allow(dead_code)]
const STORAGE_KEY_USER: &str = "archischema_user";

/// Provide auth context to the component tree
pub fn provide_auth_context() -> AuthContext {
    // Start with Unauthenticated on both server and client to avoid hydration mismatch
    let state = RwSignal::new(AuthState::Unauthenticated);
    let tokens = RwSignal::new(None::<TokenPair>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let ctx = AuthContext {
        state,
        tokens,
        loading,
        error,
    };

    // Try to restore auth state from localStorage after hydration (client-side only)
    #[cfg(not(feature = "ssr"))]
    {
        // Use Effect to run after hydration is complete
        Effect::new(move |_| {
            // Set loading state while we verify the token
            state.set(AuthState::Loading);

            // Restore state from localStorage
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    // Try to load tokens
                    if let Ok(Some(tokens_json)) = storage.get_item(STORAGE_KEY_TOKENS) {
                        if let Ok(stored_tokens) = serde_json::from_str::<TokenPair>(&tokens_json) {
                            // Check if access token is still valid (with 60s buffer)
                            let now = js_sys::Date::now() as i64 / 1000;
                            if stored_tokens.access_expires_at > now + 60 {
                                // Token might be valid by timestamp, but we need to verify with server
                                let access_token = stored_tokens.access_token.clone();
                                spawn_local(async move {
                                    // Always verify token with server to prevent tampering
                                    match fetch_current_user(&access_token).await {
                                        Ok(user) => {
                                            // Token is valid, update storage with fresh user data
                                            save_to_storage(&stored_tokens, &user);
                                            tokens.set(Some(stored_tokens));
                                            state.set(AuthState::Authenticated(user));
                                        }
                                        Err(_) => {
                                            // Token is invalid (possibly tampered), clear everything
                                            clear_storage();
                                            tokens.set(None);
                                            state.set(AuthState::Unauthenticated);
                                        }
                                    }
                                });
                                return;
                            } else if stored_tokens.refresh_expires_at > now + 60 {
                                // Access token expired but refresh token valid
                                // We'll refresh on first API call or trigger refresh here
                                let refresh_token = stored_tokens.refresh_token.clone();
                                spawn_local(async move {
                                    if let Ok((new_tokens, user)) =
                                        refresh_tokens(&refresh_token).await
                                    {
                                        save_to_storage(&new_tokens, &user);
                                        tokens.set(Some(new_tokens));
                                        state.set(AuthState::Authenticated(user));
                                    } else {
                                        // Refresh failed, clear storage
                                        clear_storage();
                                        tokens.set(None);
                                        state.set(AuthState::Unauthenticated);
                                    }
                                });
                                return;
                            } else {
                                // Both tokens expired, clear storage
                                clear_storage();
                            }
                        }
                    }
                }
            }

            // No valid tokens found
            state.set(AuthState::Unauthenticated);
        });
    }

    provide_context(ctx);
    ctx
}

/// Get auth context from the component tree
pub fn use_auth_context() -> AuthContext {
    expect_context::<AuthContext>()
}

/// Login request
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct LoginRequest {
    email: String,
    password: String,
}

/// Register request
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct RegisterRequest {
    email: String,
    username: String,
    password: String,
}

/// Auth API response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AuthResponse {
    user: UserResponse,
    tokens: TokenPairResponse,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    id: String,
    email: String,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenPairResponse {
    access_token: String,
    refresh_token: String,
    access_expires_at: i64,
    refresh_expires_at: i64,
}

/// Refresh request
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct RefreshRequest {
    refresh_token: String,
}

/// Refresh response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RefreshResponse {
    tokens: TokenPairResponse,
}

impl From<UserResponse> for User {
    fn from(u: UserResponse) -> Self {
        User {
            id: u.id,
            email: u.email,
            username: u.username,
            avatar_url: u.avatar_url,
        }
    }
}

impl From<TokenPairResponse> for TokenPair {
    fn from(t: TokenPairResponse) -> Self {
        TokenPair {
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            access_expires_at: t.access_expires_at,
            refresh_expires_at: t.refresh_expires_at,
        }
    }
}

/// Login with email and password
#[cfg(not(feature = "ssr"))]
pub async fn login(email: &str, password: &str) -> Result<(TokenPair, User), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let ctx = use_auth_context();
    ctx.loading.set(true);
    ctx.error.set(None);

    let request = LoginRequest {
        email: email.to_string(),
        password: password.to_string(),
    };

    let result = async {
        let window = web_sys::window().ok_or("No window")?;

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_body(
            &serde_json::to_string(&request)
                .map_err(|e| e.to_string())?
                .into(),
        );

        let req = Request::new_with_str_and_init("/api/auth/login", &opts)
            .map_err(|e| format!("{:?}", e))?;

        req.headers()
            .set("Content-Type", "application/json")
            .map_err(|e| format!("{:?}", e))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&req))
            .await
            .map_err(|e| format!("{:?}", e))?;

        let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

        let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
            .await
            .map_err(|e| format!("{:?}", e))?;

        if resp.ok() {
            let auth_resp: AuthResponse =
                serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

            let user: User = auth_resp.user.into();
            let tokens: TokenPair = auth_resp.tokens.into();

            save_to_storage(&tokens, &user);
            ctx.tokens.set(Some(tokens.clone()));
            ctx.state.set(AuthState::Authenticated(user.clone()));

            Ok((tokens, user))
        } else {
            let err: AuthApiError =
                serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;
            Err(err.error)
        }
    }
    .await;

    ctx.loading.set(false);

    if let Err(ref e) = result {
        ctx.error.set(Some(e.clone()));
    }

    result
}

#[cfg(feature = "ssr")]
pub async fn login(_email: &str, _password: &str) -> Result<(TokenPair, User), String> {
    Err("Login not available on server".to_string())
}

/// Register a new user
#[cfg(not(feature = "ssr"))]
pub async fn register(
    email: &str,
    username: &str,
    password: &str,
) -> Result<(TokenPair, User), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let ctx = use_auth_context();
    ctx.loading.set(true);
    ctx.error.set(None);

    let request = RegisterRequest {
        email: email.to_string(),
        username: username.to_string(),
        password: password.to_string(),
    };

    let result = async {
        let window = web_sys::window().ok_or("No window")?;

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_body(
            &serde_json::to_string(&request)
                .map_err(|e| e.to_string())?
                .into(),
        );

        let req = Request::new_with_str_and_init("/api/auth/register", &opts)
            .map_err(|e| format!("{:?}", e))?;

        req.headers()
            .set("Content-Type", "application/json")
            .map_err(|e| format!("{:?}", e))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&req))
            .await
            .map_err(|e| format!("{:?}", e))?;

        let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

        let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
            .await
            .map_err(|e| format!("{:?}", e))?;

        if resp.ok() {
            let auth_resp: AuthResponse =
                serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

            let user: User = auth_resp.user.into();
            let tokens: TokenPair = auth_resp.tokens.into();

            save_to_storage(&tokens, &user);
            ctx.tokens.set(Some(tokens.clone()));
            ctx.state.set(AuthState::Authenticated(user.clone()));

            Ok((tokens, user))
        } else {
            let err: AuthApiError =
                serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;
            Err(err.error)
        }
    }
    .await;

    ctx.loading.set(false);

    if let Err(ref e) = result {
        ctx.error.set(Some(e.clone()));
    }

    result
}

#[cfg(feature = "ssr")]
pub async fn register(
    _email: &str,
    _username: &str,
    _password: &str,
) -> Result<(TokenPair, User), String> {
    Err("Register not available on server".to_string())
}

/// Logout the current user
#[cfg(not(feature = "ssr"))]
pub async fn logout() {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let ctx = use_auth_context();

    // Try to call logout API if we have a refresh token
    if let Some(tokens) = ctx.tokens.get() {
        let request = RefreshRequest {
            refresh_token: tokens.refresh_token,
        };

        let _ = async {
            let window = web_sys::window()?;

            let opts = RequestInit::new();
            opts.set_method("POST");
            opts.set_body(&serde_json::to_string(&request).ok()?.into());

            let req = Request::new_with_str_and_init("/api/auth/logout", &opts).ok()?;
            req.headers().set("Content-Type", "application/json").ok()?;

            let resp_value = JsFuture::from(window.fetch_with_request(&req)).await.ok()?;

            let _resp: Response = resp_value.dyn_into().ok()?;

            Some(())
        }
        .await;
    }

    // Clear local state regardless of API call result
    clear_storage();
    ctx.tokens.set(None);
    ctx.state.set(AuthState::Unauthenticated);
}

#[cfg(feature = "ssr")]
pub async fn logout() {}

/// Refresh tokens using refresh token
#[cfg(not(feature = "ssr"))]
async fn refresh_tokens(refresh_token: &str) -> Result<(TokenPair, User), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let request = RefreshRequest {
        refresh_token: refresh_token.to_string(),
    };

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(
        &serde_json::to_string(&request)
            .map_err(|e| e.to_string())?
            .into(),
    );

    let req = Request::new_with_str_and_init("/api/auth/refresh", &opts)
        .map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to refresh tokens".to_string());
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let refresh_resp: RefreshResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    let tokens: TokenPair = refresh_resp.tokens.into();

    // Also fetch current user info
    let user = fetch_current_user(&tokens.access_token).await?;

    Ok((tokens, user))
}

#[cfg(feature = "ssr")]
#[allow(dead_code)]
async fn refresh_tokens(_refresh_token: &str) -> Result<(TokenPair, User), String> {
    Err("Refresh not available on server".to_string())
}

/// Fetch current user info
#[cfg(not(feature = "ssr"))]
async fn fetch_current_user(access_token: &str) -> Result<User, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let window = web_sys::window().ok_or("No window")?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let req =
        Request::new_with_str_and_init("/api/auth/me", &opts).map_err(|e| format!("{:?}", e))?;

    req.headers()
        .set("Authorization", &format!("Bearer {}", access_token))
        .map_err(|e| format!("{:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{:?}", e))?;

    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;

    if !resp.ok() {
        return Err("Failed to fetch user".to_string());
    }

    let json = JsFuture::from(resp.json().map_err(|e| format!("{:?}", e))?)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let user_resp: UserResponse =
        serde_wasm_bindgen::from_value(json).map_err(|e| e.to_string())?;

    Ok(user_resp.into())
}

/// Save tokens and user to localStorage
#[cfg(not(feature = "ssr"))]
fn save_to_storage(tokens: &TokenPair, user: &User) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(
                STORAGE_KEY_TOKENS,
                &serde_json::to_string(tokens).unwrap_or_default(),
            );
            let _ = storage.set_item(
                STORAGE_KEY_USER,
                &serde_json::to_string(user).unwrap_or_default(),
            );
        }
    }
}

/// Clear auth data from localStorage
#[cfg(not(feature = "ssr"))]
fn clear_storage() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(STORAGE_KEY_TOKENS);
            let _ = storage.remove_item(STORAGE_KEY_USER);
        }
    }
}

#[cfg(feature = "ssr")]
#[allow(dead_code)]
fn save_to_storage(_tokens: &TokenPair, _user: &User) {}

#[cfg(feature = "ssr")]
#[allow(dead_code)]
fn clear_storage() {}
