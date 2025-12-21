//! Database connection pool management
//!
//! This module provides connection pool setup and management for PostgreSQL
//! using SQLx.

use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// Database connection URL (e.g., postgres://user:pass@localhost/db)
    pub database_url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to keep open
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Idle timeout for connections in seconds
    pub idle_timeout_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

impl DbConfig {
    /// Create config from DATABASE_URL environment variable
    pub fn from_env() -> Result<Self, DbError> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| DbError::MissingDatabaseUrl)?;

        Ok(Self {
            database_url,
            ..Default::default()
        })
    }

    /// Set max connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set min connections
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, secs: u64) -> Self {
        self.connect_timeout_secs = secs;
        self
    }

    /// Set idle timeout
    pub fn idle_timeout(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }
}

/// Database errors
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("DATABASE_URL environment variable not set")]
    MissingDatabaseUrl,

    #[error("Failed to connect to database: {0}")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Failed to run migrations: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
}

/// Create a new database connection pool
pub async fn create_pool(config: &DbConfig) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .connect(&config.database_url)
        .await?;

    Ok(pool)
}

/// Create pool and run migrations
pub async fn create_pool_with_migrations(config: &DbConfig) -> Result<PgPool, DbError> {
    let pool = create_pool(config).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Check database health
pub async fn health_check(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // DbConfig Default and Builder Tests
    // ========================================================================

    #[test]
    fn test_default_config() {
        let config = DbConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 600);
        assert!(config.database_url.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = DbConfig::default()
            .max_connections(20)
            .min_connections(5)
            .connect_timeout(60)
            .idle_timeout(300);

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connect_timeout_secs, 60);
        assert_eq!(config.idle_timeout_secs, 300);
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = DbConfig::default()
            .max_connections(50)
            .max_connections(25) // Override previous value
            .min_connections(10);

        assert_eq!(config.max_connections, 25);
        assert_eq!(config.min_connections, 10);
    }

    #[test]
    fn test_config_builder_preserves_database_url() {
        let config = DbConfig {
            database_url: "postgres://localhost/test".to_string(),
            ..Default::default()
        };

        let config = config.max_connections(15).min_connections(3);

        assert_eq!(config.database_url, "postgres://localhost/test");
        assert_eq!(config.max_connections, 15);
    }

    #[test]
    fn test_config_clone() {
        let config = DbConfig {
            database_url: "postgres://user:pass@host/db".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout_secs: 45,
            idle_timeout_secs: 900,
        };

        let cloned = config.clone();
        assert_eq!(config.database_url, cloned.database_url);
        assert_eq!(config.max_connections, cloned.max_connections);
        assert_eq!(config.min_connections, cloned.min_connections);
        assert_eq!(config.connect_timeout_secs, cloned.connect_timeout_secs);
        assert_eq!(config.idle_timeout_secs, cloned.idle_timeout_secs);
    }

    #[test]
    fn test_config_debug() {
        let config = DbConfig {
            database_url: "postgres://localhost/test".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("DbConfig"));
        assert!(debug_str.contains("max_connections"));
        assert!(debug_str.contains("postgres://localhost/test"));
    }

    // ========================================================================
    // Environment Variable Tests
    // ========================================================================

    #[test]
    fn test_missing_database_url() {
        // Temporarily remove the env var if it exists
        let original = std::env::var("DATABASE_URL").ok();
        // SAFETY: We're in a single-threaded test environment
        unsafe { std::env::remove_var("DATABASE_URL") };

        let result = DbConfig::from_env();
        assert!(result.is_err());

        // Restore original value if it existed
        if let Some(val) = original {
            // SAFETY: We're in a single-threaded test environment
            unsafe { std::env::set_var("DATABASE_URL", val) };
        }
    }

    #[test]
    fn test_from_env_success() {
        // Save original
        let original = std::env::var("DATABASE_URL").ok();

        // Set test value
        // SAFETY: We're in a single-threaded test environment
        unsafe {
            std::env::set_var(
                "DATABASE_URL",
                "postgres://testuser:testpass@localhost:5432/testdb",
            );
        }

        let result = DbConfig::from_env();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(
            config.database_url,
            "postgres://testuser:testpass@localhost:5432/testdb"
        );
        // Should use default values for other fields
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);

        // Restore original value
        // SAFETY: We're in a single-threaded test environment
        unsafe {
            if let Some(val) = original {
                std::env::set_var("DATABASE_URL", val);
            } else {
                std::env::remove_var("DATABASE_URL");
            }
        }
    }

    #[test]
    fn test_from_env_with_builder() {
        let original = std::env::var("DATABASE_URL").ok();
        // SAFETY: We're in a single-threaded test environment
        unsafe { std::env::set_var("DATABASE_URL", "postgres://localhost/envtest") };

        let config = DbConfig::from_env()
            .unwrap()
            .max_connections(30)
            .min_connections(5);

        assert_eq!(config.database_url, "postgres://localhost/envtest");
        assert_eq!(config.max_connections, 30);
        assert_eq!(config.min_connections, 5);

        // SAFETY: We're in a single-threaded test environment
        unsafe {
            if let Some(val) = original {
                std::env::set_var("DATABASE_URL", val);
            } else {
                std::env::remove_var("DATABASE_URL");
            }
        }
    }

    // ========================================================================
    // DbError Tests
    // ========================================================================

    #[test]
    fn test_db_error_missing_url_display() {
        let err = DbError::MissingDatabaseUrl;
        let display = format!("{}", err);
        assert!(display.contains("DATABASE_URL"));
        assert!(display.contains("not set"));
    }

    #[test]
    fn test_db_error_debug() {
        let err = DbError::MissingDatabaseUrl;
        let debug = format!("{:?}", err);
        assert!(debug.contains("MissingDatabaseUrl"));
    }

    // ========================================================================
    // Edge Cases and Boundary Tests
    // ========================================================================

    #[test]
    fn test_config_zero_values() {
        let config = DbConfig::default()
            .max_connections(0)
            .min_connections(0)
            .connect_timeout(0)
            .idle_timeout(0);

        assert_eq!(config.max_connections, 0);
        assert_eq!(config.min_connections, 0);
        assert_eq!(config.connect_timeout_secs, 0);
        assert_eq!(config.idle_timeout_secs, 0);
    }

    #[test]
    fn test_config_max_values() {
        let config = DbConfig::default()
            .max_connections(u32::MAX)
            .min_connections(u32::MAX)
            .connect_timeout(u64::MAX)
            .idle_timeout(u64::MAX);

        assert_eq!(config.max_connections, u32::MAX);
        assert_eq!(config.min_connections, u32::MAX);
        assert_eq!(config.connect_timeout_secs, u64::MAX);
        assert_eq!(config.idle_timeout_secs, u64::MAX);
    }

    #[test]
    fn test_config_with_empty_database_url() {
        let config = DbConfig {
            database_url: String::new(),
            ..Default::default()
        };

        assert!(config.database_url.is_empty());
    }

    #[test]
    fn test_config_with_special_characters_in_url() {
        let config = DbConfig {
            database_url: "postgres://user:p%40ss%20word@host:5432/db?sslmode=require".to_string(),
            ..Default::default()
        };

        assert!(config.database_url.contains("p%40ss%20word"));
    }

    #[test]
    fn test_config_with_ipv6_host() {
        let config = DbConfig {
            database_url: "postgres://user:pass@[::1]:5432/db".to_string(),
            ..Default::default()
        };

        assert!(config.database_url.contains("[::1]"));
    }

    #[test]
    fn test_config_reasonable_production_values() {
        let config = DbConfig::default()
            .max_connections(100)
            .min_connections(10)
            .connect_timeout(5)
            .idle_timeout(300);

        assert_eq!(config.max_connections, 100);
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.connect_timeout_secs, 5);
        assert_eq!(config.idle_timeout_secs, 300);

        // min should be less than max (logical check)
        assert!(config.min_connections <= config.max_connections);
    }

    #[test]
    fn test_config_min_greater_than_max() {
        // Note: This tests that we CAN set invalid configs
        // Actual validation would happen at runtime when creating the pool
        let config = DbConfig::default().max_connections(5).min_connections(10);

        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 10);
        // This is an invalid configuration, but we allow it at config level
        // The pool creation will fail with this config
    }

    // ========================================================================
    // Integration Test Markers (require real database)
    // ========================================================================

    // These tests are marked with #[ignore] as they require a running database
    // Run with: cargo test --features ssr -- --ignored

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_pool_success() {
        let config = DbConfig::from_env().expect("DATABASE_URL must be set");
        let result = create_pool(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_pool_invalid_url() {
        let config = DbConfig {
            database_url: "postgres://invalid:invalid@nonexistent:5432/db".to_string(),
            connect_timeout_secs: 1, // Short timeout for faster test
            ..Default::default()
        };

        let result = create_pool(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_health_check_success() {
        let config = DbConfig::from_env().expect("DATABASE_URL must be set");
        let pool = create_pool(&config).await.expect("Failed to create pool");

        let result = health_check(&pool).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires running PostgreSQL database"]
    async fn test_create_pool_with_migrations_success() {
        let config = DbConfig::from_env().expect("DATABASE_URL must be set");
        let result = create_pool_with_migrations(&config).await;
        assert!(result.is_ok());
    }
}
