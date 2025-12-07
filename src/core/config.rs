//! Application configuration from environment variables.
//!
//! Load configuration using `Config::from_env()` after calling `dotenvy::dotenv()`.

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// MySQL database connection URL
    /// Example: mysql://user:password@localhost:3306/database
    pub database_url: Option<String>,

    /// Redis connection URL
    /// Example: redis://localhost:6379
    pub redis_url: Option<String>,

    /// Secret key for signing tokens, cookies, etc.
    /// Should be a long random string in production
    pub secret_key: Option<String>,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Call `dotenvy::dotenv()` before this to load from `.env` file.
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").ok(),
            redis_url: std::env::var("REDIS_URL").ok(),
            secret_key: std::env::var("SECRET_KEY").ok(),
        }
    }

    /// Check if database is configured
    pub fn has_database(&self) -> bool {
        self.database_url.is_some()
    }

    /// Check if Redis is configured
    pub fn has_redis(&self) -> bool {
        self.redis_url.is_some()
    }

    /// Check if secret key is configured
    pub fn has_secret_key(&self) -> bool {
        self.secret_key.is_some()
    }

    /// Get database URL or panic with a helpful message
    pub fn database_url_or_panic(&self) -> &str {
        self.database_url
            .as_deref()
            .expect("DATABASE_URL environment variable is not set")
    }

    /// Get Redis URL or panic with a helpful message
    pub fn redis_url_or_panic(&self) -> &str {
        self.redis_url
            .as_deref()
            .expect("REDIS_URL environment variable is not set")
    }

    /// Get secret key or panic with a helpful message
    pub fn secret_key_or_panic(&self) -> &str {
        self.secret_key
            .as_deref()
            .expect("SECRET_KEY environment variable is not set")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Config Struct Tests (no env var dependencies - thread safe)
    // ========================================================================

    #[test]
    fn test_config_with_all_fields() {
        let config = Config {
            database_url: Some("mysql://user:pass@localhost:3306/testdb".to_string()),
            redis_url: Some("redis://localhost:6379".to_string()),
            secret_key: Some("super-secret-key-123".to_string()),
        };

        assert_eq!(
            config.database_url,
            Some("mysql://user:pass@localhost:3306/testdb".to_string())
        );
        assert_eq!(config.redis_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.secret_key, Some("super-secret-key-123".to_string()));
    }

    #[test]
    fn test_config_with_no_fields() {
        let config = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        assert!(config.database_url.is_none());
        assert!(config.redis_url.is_none());
        assert!(config.secret_key.is_none());
    }

    #[test]
    fn test_config_with_partial_fields() {
        let config = Config {
            database_url: Some("mysql://localhost/db".to_string()),
            redis_url: None,
            secret_key: Some("my-secret".to_string()),
        };

        assert_eq!(
            config.database_url,
            Some("mysql://localhost/db".to_string())
        );
        assert!(config.redis_url.is_none());
        assert_eq!(config.secret_key, Some("my-secret".to_string()));
    }

    #[test]
    fn test_has_database() {
        let config_with = Config {
            database_url: Some("mysql://localhost".to_string()),
            redis_url: None,
            secret_key: None,
        };
        let config_without = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        assert!(config_with.has_database());
        assert!(!config_without.has_database());
    }

    #[test]
    fn test_has_redis() {
        let config_with = Config {
            database_url: None,
            redis_url: Some("redis://localhost:6379".to_string()),
            secret_key: None,
        };
        let config_without = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        assert!(config_with.has_redis());
        assert!(!config_without.has_redis());
    }

    #[test]
    fn test_has_secret_key() {
        let config_with = Config {
            database_url: None,
            redis_url: None,
            secret_key: Some("secret".to_string()),
        };
        let config_without = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        assert!(config_with.has_secret_key());
        assert!(!config_without.has_secret_key());
    }

    #[test]
    fn test_database_url_or_panic_success() {
        let config = Config {
            database_url: Some("mysql://localhost/db".to_string()),
            redis_url: None,
            secret_key: None,
        };

        assert_eq!(config.database_url_or_panic(), "mysql://localhost/db");
    }

    #[test]
    #[should_panic(expected = "DATABASE_URL environment variable is not set")]
    fn test_database_url_or_panic_failure() {
        let config = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        config.database_url_or_panic();
    }

    #[test]
    fn test_redis_url_or_panic_success() {
        let config = Config {
            database_url: None,
            redis_url: Some("redis://localhost:6379".to_string()),
            secret_key: None,
        };

        assert_eq!(config.redis_url_or_panic(), "redis://localhost:6379");
    }

    #[test]
    #[should_panic(expected = "REDIS_URL environment variable is not set")]
    fn test_redis_url_or_panic_failure() {
        let config = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        config.redis_url_or_panic();
    }

    #[test]
    fn test_secret_key_or_panic_success() {
        let config = Config {
            database_url: None,
            redis_url: None,
            secret_key: Some("my-super-secret".to_string()),
        };

        assert_eq!(config.secret_key_or_panic(), "my-super-secret");
    }

    #[test]
    #[should_panic(expected = "SECRET_KEY environment variable is not set")]
    fn test_secret_key_or_panic_failure() {
        let config = Config {
            database_url: None,
            redis_url: None,
            secret_key: None,
        };

        config.secret_key_or_panic();
    }

    #[test]
    fn test_config_from_env_returns_config() {
        // Just verify from_env() returns a Config without errors
        // Actual values depend on environment, so we don't assert specific values
        let config = Config::from_env();

        // These methods should work regardless of env var values
        let _ = config.has_database();
        let _ = config.has_redis();
        let _ = config.has_secret_key();
    }

    #[test]
    fn test_config_default_calls_from_env() {
        // Default implementation calls from_env()
        let config = Config::default();

        // Should return a valid Config struct
        let _ = config.has_database();
        let _ = config.has_redis();
        let _ = config.has_secret_key();
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            database_url: Some("mysql://localhost".to_string()),
            redis_url: Some("redis://localhost".to_string()),
            secret_key: Some("secret".to_string()),
        };

        let cloned = config.clone();

        assert_eq!(config.database_url, cloned.database_url);
        assert_eq!(config.redis_url, cloned.redis_url);
        assert_eq!(config.secret_key, cloned.secret_key);
    }

    #[test]
    fn test_config_debug() {
        let config = Config {
            database_url: Some("mysql://localhost".to_string()),
            redis_url: None,
            secret_key: Some("secret".to_string()),
        };

        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("database_url"));
        assert!(debug_str.contains("mysql://localhost"));
    }

    #[test]
    fn test_config_with_empty_string_values() {
        // Test that empty strings are treated as Some(""), not None
        let config = Config {
            database_url: Some("".to_string()),
            redis_url: Some("".to_string()),
            secret_key: Some("".to_string()),
        };

        assert_eq!(config.database_url, Some("".to_string()));
        assert_eq!(config.redis_url, Some("".to_string()));
        assert_eq!(config.secret_key, Some("".to_string()));

        // Empty strings still count as "having" the config
        assert!(config.has_database());
        assert!(config.has_redis());
        assert!(config.has_secret_key());
    }

    #[test]
    fn test_config_with_special_characters() {
        let config = Config {
            database_url: Some(
                "mysql://user:p@ss=w0rd!@localhost:3306/db?charset=utf8".to_string(),
            ),
            redis_url: Some("redis://:password@localhost:6379/0".to_string()),
            secret_key: Some("key-with-special-chars!@#$%^&*()".to_string()),
        };

        assert_eq!(
            config.database_url,
            Some("mysql://user:p@ss=w0rd!@localhost:3306/db?charset=utf8".to_string())
        );
        assert_eq!(
            config.redis_url,
            Some("redis://:password@localhost:6379/0".to_string())
        );
        assert_eq!(
            config.secret_key,
            Some("key-with-special-chars!@#$%^&*()".to_string())
        );
    }
}
