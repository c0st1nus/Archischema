//! Validation module for database identifiers (table and column names)
//!
//! Implements standard database naming rules compatible with MySQL, PostgreSQL, and other RDBMS.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Maximum length for identifiers (MySQL standard)
pub const MAX_IDENTIFIER_LENGTH: usize = 64;

/// Minimum length for identifiers
pub const MIN_IDENTIFIER_LENGTH: usize = 1;

/// SQL reserved keywords that cannot be used as identifiers without quoting
/// This is a combined list from MySQL, PostgreSQL, and SQL standard
static RESERVED_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // SQL Standard
        "ADD",
        "ALL",
        "ALTER",
        "AND",
        "ANY",
        "AS",
        "ASC",
        "BETWEEN",
        "BY",
        "CASE",
        "CHECK",
        "COLUMN",
        "CONSTRAINT",
        "CREATE",
        "CROSS",
        "CURRENT",
        "CURRENT_DATE",
        "CURRENT_TIME",
        "CURRENT_TIMESTAMP",
        "CURRENT_USER",
        "DATABASE",
        "DEFAULT",
        "DELETE",
        "DESC",
        "DISTINCT",
        "DROP",
        "ELSE",
        "END",
        "EXISTS",
        "FALSE",
        "FETCH",
        "FOR",
        "FOREIGN",
        "FROM",
        "FULL",
        "GRANT",
        "GROUP",
        "HAVING",
        "IF",
        "IN",
        "INDEX",
        "INNER",
        "INSERT",
        "INTO",
        "IS",
        "JOIN",
        "KEY",
        "LEFT",
        "LIKE",
        "LIMIT",
        "NOT",
        "NULL",
        "OFFSET",
        "ON",
        "OR",
        "ORDER",
        "OUTER",
        "PRIMARY",
        "REFERENCES",
        "RIGHT",
        "SELECT",
        "SET",
        "TABLE",
        "THEN",
        "TO",
        "TRUE",
        "UNION",
        "UNIQUE",
        "UPDATE",
        "USING",
        "VALUES",
        "WHEN",
        "WHERE",
        "WITH",
        // MySQL specific
        "AUTO_INCREMENT",
        "BIGINT",
        "BINARY",
        "BLOB",
        "BOOL",
        "BOOLEAN",
        "CHANGE",
        "CHAR",
        "CHARACTER",
        "COLLATE",
        "DATE",
        "DATETIME",
        "DECIMAL",
        "DOUBLE",
        "ENUM",
        "EXPLAIN",
        "FLOAT",
        "FORCE",
        "IGNORE",
        "INT",
        "INTEGER",
        "INTERVAL",
        "LONGBLOB",
        "LONGTEXT",
        "MEDIUMBLOB",
        "MEDIUMINT",
        "MEDIUMTEXT",
        "MODIFY",
        "NUMERIC",
        "PROCEDURE",
        "REAL",
        "RENAME",
        "REPLACE",
        "SCHEMA",
        "SHOW",
        "SMALLINT",
        "TEXT",
        "TIME",
        "TIMESTAMP",
        "TINYBLOB",
        "TINYINT",
        "TINYTEXT",
        "TRIGGER",
        "TRUNCATE",
        "UNSIGNED",
        "VARBINARY",
        "VARCHAR",
        "VIEW",
        "YEAR",
        "ZEROFILL",
        // PostgreSQL specific
        "ANALYSE",
        "ANALYZE",
        "ARRAY",
        "ASYMMETRIC",
        "AUTHORIZATION",
        "BOTH",
        "CAST",
        "CONCURRENTLY",
        "DEFERRABLE",
        "DO",
        "EXCEPT",
        "FREEZE",
        "GRANT",
        "ILIKE",
        "INITIALLY",
        "INTERSECT",
        "ISNULL",
        "LATERAL",
        "LEADING",
        "LOCALTIME",
        "LOCALTIMESTAMP",
        "NATURAL",
        "NOTNULL",
        "ONLY",
        "OVERLAPS",
        "PLACING",
        "RETURNING",
        "SESSION_USER",
        "SIMILAR",
        "SOME",
        "SYMMETRIC",
        "TABLESAMPLE",
        "TRAILING",
        "VARIADIC",
        "VERBOSE",
        "WINDOW",
    ]
    .into_iter()
    .collect()
});

/// Validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Identifier is empty
    Empty,
    /// Identifier is too long
    TooLong { max: usize, actual: usize },
    /// Identifier contains invalid characters
    InvalidCharacters { invalid: Vec<char> },
    /// Identifier starts with a digit
    StartsWithDigit,
    /// Identifier starts with underscore (warning in some databases)
    StartsWithUnderscore,
    /// Identifier is a reserved keyword
    ReservedKeyword { keyword: String },
    /// Identifier contains only underscores/digits
    NoLetters,
    /// Identifier contains consecutive underscores
    ConsecutiveUnderscores,
    /// Identifier ends with underscore
    EndsWithUnderscore,
    /// Custom validation error
    Custom(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Empty => write!(f, "Name cannot be empty"),
            ValidationError::TooLong { max, actual } => {
                write!(f, "Name is too long ({} chars, max {})", actual, max)
            }
            ValidationError::InvalidCharacters { invalid } => {
                let chars: String = invalid.iter().collect();
                write!(
                    f,
                    "Name contains invalid characters: '{}'. Only letters, numbers, and underscores are allowed",
                    chars
                )
            }
            ValidationError::StartsWithDigit => {
                write!(f, "Name cannot start with a digit")
            }
            ValidationError::StartsWithUnderscore => {
                write!(
                    f,
                    "Name should not start with an underscore (reserved for system use)"
                )
            }
            ValidationError::ReservedKeyword { keyword } => {
                write!(f, "'{}' is a reserved SQL keyword", keyword)
            }
            ValidationError::NoLetters => {
                write!(f, "Name must contain at least one letter")
            }
            ValidationError::ConsecutiveUnderscores => {
                write!(f, "Name cannot contain consecutive underscores")
            }
            ValidationError::EndsWithUnderscore => {
                write!(f, "Name should not end with an underscore")
            }
            ValidationError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validation strictness level
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ValidationLevel {
    /// Only check critical errors (empty, too long, invalid chars, starts with digit)
    Minimal,
    /// Standard validation (minimal + reserved keywords)
    #[default]
    Standard,
    /// Strict validation (standard + style warnings as errors)
    Strict,
}

/// Validation result containing errors and warnings
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// Critical errors that must be fixed
    pub errors: Vec<ValidationError>,
    /// Warnings that are recommended to fix
    pub warnings: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ValidationError) {
        self.warnings.push(warning);
    }

    /// Convert to Result, returning first error if any
    pub fn to_result(&self) -> Result<(), ValidationError> {
        if let Some(error) = self.errors.first() {
            Err(error.clone())
        } else {
            Ok(())
        }
    }

    /// Get all messages (errors and warnings) as strings
    pub fn all_messages(&self) -> Vec<String> {
        self.errors
            .iter()
            .map(|e| format!("Error: {}", e))
            .chain(self.warnings.iter().map(|w| format!("Warning: {}", w)))
            .collect()
    }
}

/// Validates a database identifier (table or column name)
pub fn validate_identifier(name: &str, level: ValidationLevel) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check empty
    let trimmed = name.trim();
    if trimmed.is_empty() {
        result.add_error(ValidationError::Empty);
        return result; // Can't continue validation
    }

    // Check length
    if trimmed.len() > MAX_IDENTIFIER_LENGTH {
        result.add_error(ValidationError::TooLong {
            max: MAX_IDENTIFIER_LENGTH,
            actual: trimmed.len(),
        });
    }

    // Check for invalid characters (only ASCII alphanumeric and underscore allowed)
    let invalid_chars: Vec<char> = trimmed
        .chars()
        .filter(|c| !c.is_ascii_alphanumeric() && *c != '_')
        .collect();
    if !invalid_chars.is_empty() {
        result.add_error(ValidationError::InvalidCharacters {
            invalid: invalid_chars,
        });
    }

    // Check if starts with digit
    if let Some(first_char) = trimmed.chars().next()
        && first_char.is_ascii_digit()
    {
        result.add_error(ValidationError::StartsWithDigit);
    }

    // Check if contains at least one ASCII letter
    if !trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
        result.add_error(ValidationError::NoLetters);
    }

    // Standard and Strict: Check reserved keywords
    if level == ValidationLevel::Standard || level == ValidationLevel::Strict {
        let upper = trimmed.to_uppercase();
        if RESERVED_KEYWORDS.contains(upper.as_str()) {
            result.add_error(ValidationError::ReservedKeyword {
                keyword: trimmed.to_string(),
            });
        }
    }

    // Strict: Additional style checks
    if level == ValidationLevel::Strict {
        // Starts with underscore
        if trimmed.starts_with('_') {
            result.add_error(ValidationError::StartsWithUnderscore);
        }

        // Ends with underscore
        if trimmed.ends_with('_') {
            result.add_error(ValidationError::EndsWithUnderscore);
        }

        // Consecutive underscores
        if trimmed.contains("__") {
            result.add_error(ValidationError::ConsecutiveUnderscores);
        }
    } else {
        // Add as warnings for non-strict modes
        if trimmed.starts_with('_') {
            result.add_warning(ValidationError::StartsWithUnderscore);
        }

        if trimmed.ends_with('_') {
            result.add_warning(ValidationError::EndsWithUnderscore);
        }

        if trimmed.contains("__") {
            result.add_warning(ValidationError::ConsecutiveUnderscores);
        }
    }

    result
}

/// Simple validation function that returns Result<(), String>
/// Uses Standard validation level
pub fn validate_name(name: &str) -> Result<(), String> {
    let result = validate_identifier(name, ValidationLevel::Standard);
    result.to_result().map_err(|e| e.to_string())
}

/// Validate table name with Standard level
pub fn validate_table_name(name: &str) -> Result<(), String> {
    validate_name(name)
}

/// Validate column name with Standard level
pub fn validate_column_name(name: &str) -> Result<(), String> {
    validate_name(name)
}

/// Check if a string is a reserved keyword
pub fn is_reserved_keyword(name: &str) -> bool {
    RESERVED_KEYWORDS.contains(name.to_uppercase().as_str())
}

/// Sanitize an identifier by removing/replacing invalid characters
/// Returns None if the result would be empty
pub fn sanitize_identifier(name: &str) -> Option<String> {
    let sanitized: String = name
        .trim()
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                Some(c)
            } else if c == ' ' || c == '-' {
                Some('_')
            } else {
                None
            }
        })
        .collect();

    // Remove leading digits
    let sanitized = sanitized.trim_start_matches(|c: char| c.is_ascii_digit());

    // Remove consecutive underscores
    let mut result = String::new();
    let mut last_was_underscore = false;
    for c in sanitized.chars() {
        if c == '_' {
            if !last_was_underscore {
                result.push(c);
                last_was_underscore = true;
            }
        } else {
            result.push(c);
            last_was_underscore = false;
        }
    }

    // Trim underscores from start and end
    let result = result.trim_matches('_').to_string();

    if result.is_empty() || !result.chars().any(|c| c.is_ascii_alphabetic()) {
        None
    } else {
        // Truncate if too long
        Some(if result.len() > MAX_IDENTIFIER_LENGTH {
            result[..MAX_IDENTIFIER_LENGTH].to_string()
        } else {
            result
        })
    }
}

/// Suggests a valid identifier based on the input
/// Adds a prefix if the name starts with a digit or is a reserved keyword
pub fn suggest_valid_name(name: &str, prefix: &str) -> String {
    // First try to sanitize
    if let Some(sanitized) = sanitize_identifier(name) {
        // Check if it's a reserved keyword
        if is_reserved_keyword(&sanitized) {
            format!("{}_{}", prefix, sanitized.to_lowercase())
        } else {
            sanitized
        }
    } else {
        // Fallback to prefix with timestamp-like suffix
        format!("{}_1", prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_identifiers() {
        assert!(validate_name("users").is_ok());
        assert!(validate_name("user_id").is_ok());
        assert!(validate_name("User123").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("table_name_here").is_ok());
    }

    #[test]
    fn test_empty_name() {
        assert!(validate_name("").is_err());
        assert!(validate_name("   ").is_err());
    }

    #[test]
    fn test_too_long_name() {
        let long_name = "a".repeat(65);
        assert!(validate_name(&long_name).is_err());

        let ok_name = "a".repeat(64);
        assert!(validate_name(&ok_name).is_ok());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(validate_name("user-name").is_err());
        assert!(validate_name("user name").is_err());
        assert!(validate_name("user@name").is_err());
        assert!(validate_name("user.name").is_err());
        assert!(validate_name("имя").is_err()); // Non-ASCII letters are not allowed
        assert!(validate_name("tëst").is_err()); // Non-ASCII letters are not allowed
    }

    #[test]
    fn test_starts_with_digit() {
        assert!(validate_name("1user").is_err());
        assert!(validate_name("123").is_err());
        assert!(validate_name("0_table").is_err());
    }

    #[test]
    fn test_reserved_keywords() {
        assert!(validate_name("SELECT").is_err());
        assert!(validate_name("select").is_err());
        assert!(validate_name("Table").is_err());
        assert!(validate_name("FROM").is_err());
        assert!(validate_name("user").is_ok()); // Not a reserved keyword
    }

    #[test]
    fn test_no_letters() {
        assert!(validate_name("123").is_err());
        assert!(validate_name("___").is_err());
        assert!(validate_name("_1_2_").is_err());
    }

    #[test]
    fn test_validation_levels() {
        // Minimal level doesn't check keywords
        let result = validate_identifier("select", ValidationLevel::Minimal);
        assert!(result.is_valid());

        // Standard level checks keywords
        let result = validate_identifier("select", ValidationLevel::Standard);
        assert!(!result.is_valid());

        // Strict level fails on underscore prefix
        let result = validate_identifier("_name", ValidationLevel::Strict);
        assert!(!result.is_valid());

        // Standard level only warns
        let result = validate_identifier("_name", ValidationLevel::Standard);
        assert!(result.is_valid());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_warnings() {
        let result = validate_identifier("_private", ValidationLevel::Standard);
        assert!(result.is_valid());
        assert!(result.has_warnings());

        let result = validate_identifier("name_", ValidationLevel::Standard);
        assert!(result.is_valid());
        assert!(result.has_warnings());

        let result = validate_identifier("some__name", ValidationLevel::Standard);
        assert!(result.is_valid());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(
            sanitize_identifier("user name"),
            Some("user_name".to_string())
        );
        assert_eq!(
            sanitize_identifier("user-name"),
            Some("user_name".to_string())
        );
        assert_eq!(sanitize_identifier("123user"), Some("user".to_string()));
        assert_eq!(sanitize_identifier("__name__"), Some("name".to_string()));
        assert_eq!(sanitize_identifier("a  b  c"), Some("a_b_c".to_string()));
        assert_eq!(sanitize_identifier("123"), None);
        assert_eq!(sanitize_identifier("@#$"), None);
    }

    #[test]
    fn test_suggest_valid_name() {
        assert_eq!(suggest_valid_name("user name", "col"), "user_name");
        assert_eq!(suggest_valid_name("SELECT", "col"), "col_select");
        assert_eq!(suggest_valid_name("123", "col"), "col_1");
        assert_eq!(suggest_valid_name("valid_name", "col"), "valid_name");
    }

    #[test]
    fn test_is_reserved_keyword() {
        assert!(is_reserved_keyword("SELECT"));
        assert!(is_reserved_keyword("select"));
        assert!(is_reserved_keyword("Select"));
        assert!(!is_reserved_keyword("users"));
        assert!(!is_reserved_keyword("my_table"));
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(ValidationError::Empty.to_string(), "Name cannot be empty");
        assert_eq!(
            ValidationError::TooLong {
                max: 64,
                actual: 100
            }
            .to_string(),
            "Name is too long (100 chars, max 64)"
        );
        assert_eq!(
            ValidationError::StartsWithDigit.to_string(),
            "Name cannot start with a digit"
        );
    }
}
