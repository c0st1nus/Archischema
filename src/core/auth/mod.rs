//! Authentication module for Archischema
//!
//! This module provides authentication functionality including:
//! - JWT token generation and validation
//! - User registration and login
//! - Session management with refresh tokens
//! - REST API endpoints for auth operations

pub mod api;
pub mod jwt;
pub mod service;

pub use api::{AuthApiState, auth_api_router};
pub use jwt::{Claims, JwtConfig, JwtError, JwtService, TokenPair, TokenType};
pub use service::{
    AuthError, AuthResponse, AuthService, LoginRequest, RefreshRequest, RegisterRequest,
};
