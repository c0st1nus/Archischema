//! AI Assistant Configuration
//!
//! This module provides configuration for the AI assistant, including:
//! - API endpoint configuration (base URL)
//! - API key management
//! - Model selection
//! - System prompt customization
//! - Mode selection (Write/Ask)

use serde::{Deserialize, Serialize};

/// Default API base URL for OpenRouter
pub const DEFAULT_API_BASE: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Default model to use
pub const DEFAULT_MODEL: &str = "google/gemini-2.5-flash-lite";

/// Default system prompt for the AI assistant
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are an AI assistant for Archischema, a visual database schema editor. You help users design and understand database schemas.

**IMPORTANT: Always format your responses using Markdown** for better readability:
- Use **bold** for table and column names
- Use `code` for data types and SQL keywords
- Use bullet lists for enumerating columns or properties
- Use headers (##, ###) to organize longer responses
- Use code blocks with ```sql for SQL examples

You have access to tools that allow you to read and (when in Write mode) modify the database schema.

When the user asks about the schema:
1. First use get_schema_json or list_tables to understand the current state
2. Provide clear explanations about tables, columns, and relationships
3. When suggesting changes, explain the reasoning

When creating tables:
1. Follow database naming conventions (snake_case for tables and columns)
2. Always include appropriate primary keys
3. Use proper data types for the use case
4. IMPORTANT: Tables are displayed on a visual canvas. If you don't specify position_x and position_y, they will be auto-positioned. But for better layout, you can specify positions yourself.
5. Recommended layout: arrange tables in a grid with ~300px spacing. Example positions:
   - First table: position_x=100, position_y=100
   - Second table: position_x=450, position_y=100
   - Third table: position_x=800, position_y=100
   - Fourth table: position_x=100, position_y=350
   - Place related tables near each other for better visualization

When creating relationships:
1. ALWAYS create relationships after creating all tables
2. Use create_relationship tool with from_table, from_column, to_table, to_column
3. relationship_type values: "OneToOne", "OneToMany", "ManyToOne", "ManyToMany"
4. For foreign keys: the "from" side is the table with the FK column, "to" side is the referenced table
5. Example: students.course_id -> courses.id should be from_table="students", from_column="course_id", to_table="courses", to_column="id"

Available data types: INT, BIGINT, VARCHAR, TEXT, BOOLEAN, DATE, DATETIME, TIMESTAMP, DECIMAL, FLOAT, JSON, ENUM, etc.

Be concise but helpful. When creating a schema, create ALL tables first, then ALL relationships. If you're unsure about something, ask for clarification."#;

/// AI Assistant mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AiMode {
    /// Read-only mode - can only read schema, cannot modify
    #[default]
    Ask,
    /// Write mode - can both read and modify schema
    Write,
}

impl AiMode {
    /// Check if this mode allows writing to the schema
    pub fn can_write(&self) -> bool {
        matches!(self, AiMode::Write)
    }

    /// Get display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            AiMode::Ask => "Ask",
            AiMode::Write => "Write",
        }
    }

    /// Get description for the mode
    pub fn description(&self) -> &'static str {
        match self {
            AiMode::Ask => "Read-only mode - AI can analyze but not modify schema",
            AiMode::Write => "Full access - AI can create, modify, and delete schema elements",
        }
    }
}

/// AI Assistant configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiConfig {
    /// API base URL (OpenRouter compatible endpoint)
    pub api_base: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Model identifier (e.g., "openai/gpt-4o", "anthropic/claude-3-opus")
    pub model: String,

    /// System prompt for the AI
    pub system_prompt: String,

    /// Current mode (Ask or Write)
    pub mode: AiMode,

    /// Temperature for generation (0.0 - 2.0)
    pub temperature: f32,

    /// Max tokens for response
    pub max_tokens: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_base: DEFAULT_API_BASE.to_string(),
            api_key: None,
            model: DEFAULT_MODEL.to_string(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            mode: AiMode::default(),
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

impl AiConfig {
    /// Create a new config with custom settings
    pub fn new(
        api_base: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_base: api_base.into(),
            api_key,
            model: model.into(),
            ..Default::default()
        }
    }

    /// Load configuration from environment variables, falling back to defaults
    #[cfg(feature = "ssr")]
    pub fn from_env() -> Self {
        let api_base =
            std::env::var("OPENAPI_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());

        let api_key = std::env::var("OPENAPI_TOKEN").ok();

        let model = std::env::var("DEFAULT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Self {
            api_base,
            api_key,
            model,
            ..Default::default()
        }
    }

    /// Check if the configuration has a valid API key
    pub fn has_api_key(&self) -> bool {
        self.api_key.as_ref().is_some_and(|k| !k.is_empty())
    }

    /// Set the mode
    pub fn with_mode(mut self, mode: AiMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// Chat message role
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single chat message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    /// Tool call ID (for tool responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls made by assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool_response(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// Tool call from assistant
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Chat completion request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Enable streaming responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Tool definition for the API
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// Function definition for tools
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Chat completion response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Response choice
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Streaming response chunk (SSE format)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Streaming choice with delta content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

/// Delta content in streaming response
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StreamDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<StreamToolCall>>,
}

/// Tool call in streaming response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamToolCall {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "type")]
    pub call_type: Option<String>,
    #[serde(default)]
    pub function: Option<StreamFunctionCall>,
}

/// Function call in streaming response
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StreamFunctionCall {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

/// Build tool definitions for OpenRouter API
pub fn build_tool_definitions(mode: AiMode) -> Vec<ToolDefinition> {
    let mut tools = vec![
        // Read-only tools (always available)
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_schema_sql".to_string(),
                description: "Get the current database schema as SQL DDL statements".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_schema_json".to_string(),
                description: "Get the current database schema as structured JSON".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "list_tables".to_string(),
                description: "List all table names in the schema".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_table".to_string(),
                description: "Get detailed information about a specific table".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Name of the table to retrieve"
                        }
                    },
                    "required": ["table_name"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_relationships".to_string(),
                description: "Get all relationships involving a specific table".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Name of the table"
                        }
                    },
                    "required": ["table_name"]
                }),
            },
        },
    ];

    // Write tools (only in Write mode)
    if mode.can_write() {
        tools.extend(vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "create_table".to_string(),
                    description: "Create a new table in the schema".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name of the new table"
                            },
                            "columns": {
                                "type": "array",
                                "description": "Array of column definitions",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "data_type": { "type": "string" },
                                        "is_primary_key": { "type": "boolean" },
                                        "is_nullable": { "type": "boolean" },
                                        "is_unique": { "type": "boolean" },
                                        "default_value": { "type": "string" }
                                    },
                                    "required": ["name", "data_type"]
                                }
                            },
                            "position_x": {
                                "type": "number",
                                "description": "X position on canvas"
                            },
                            "position_y": {
                                "type": "number",
                                "description": "Y position on canvas"
                            }
                        },
                        "required": ["name"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "rename_table".to_string(),
                    description: "Rename an existing table".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "old_name": {
                                "type": "string",
                                "description": "Current name of the table"
                            },
                            "new_name": {
                                "type": "string",
                                "description": "New name for the table"
                            }
                        },
                        "required": ["old_name", "new_name"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "delete_table".to_string(),
                    description: "Delete a table from the schema".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "table_name": {
                                "type": "string",
                                "description": "Name of the table to delete"
                            }
                        },
                        "required": ["table_name"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "add_column".to_string(),
                    description: "Add a new column to an existing table".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "table_name": {
                                "type": "string",
                                "description": "Name of the table"
                            },
                            "column_name": {
                                "type": "string",
                                "description": "Name of the new column"
                            },
                            "data_type": {
                                "type": "string",
                                "description": "Data type of the column"
                            },
                            "is_primary_key": {
                                "type": "boolean",
                                "description": "Whether this is a primary key"
                            },
                            "is_nullable": {
                                "type": "boolean",
                                "description": "Whether this column allows NULL"
                            },
                            "is_unique": {
                                "type": "boolean",
                                "description": "Whether this column must be unique"
                            },
                            "default_value": {
                                "type": "string",
                                "description": "Default value for the column"
                            }
                        },
                        "required": ["table_name", "column_name", "data_type"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "modify_column".to_string(),
                    description: "Modify an existing column".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "table_name": {
                                "type": "string",
                                "description": "Name of the table"
                            },
                            "column_name": {
                                "type": "string",
                                "description": "Current name of the column"
                            },
                            "new_name": {
                                "type": "string",
                                "description": "New name for the column"
                            },
                            "data_type": {
                                "type": "string",
                                "description": "New data type"
                            },
                            "is_primary_key": {
                                "type": "boolean"
                            },
                            "is_nullable": {
                                "type": "boolean"
                            },
                            "is_unique": {
                                "type": "boolean"
                            },
                            "default_value": {
                                "type": "string"
                            }
                        },
                        "required": ["table_name", "column_name"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "delete_column".to_string(),
                    description: "Delete a column from a table".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "table_name": {
                                "type": "string",
                                "description": "Name of the table"
                            },
                            "column_name": {
                                "type": "string",
                                "description": "Name of the column to delete"
                            }
                        },
                        "required": ["table_name", "column_name"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "create_relationship".to_string(),
                    description: "Create a relationship between two tables".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name of the relationship"
                            },
                            "from_table": {
                                "type": "string",
                                "description": "Source table name"
                            },
                            "from_column": {
                                "type": "string",
                                "description": "Source column name"
                            },
                            "to_table": {
                                "type": "string",
                                "description": "Target table name"
                            },
                            "to_column": {
                                "type": "string",
                                "description": "Target column name"
                            },
                            "relationship_type": {
                                "type": "string",
                                "enum": ["OneToOne", "OneToMany", "ManyToOne", "ManyToMany"],
                                "description": "Type of relationship"
                            }
                        },
                        "required": ["from_table", "from_column", "to_table", "to_column"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "delete_relationship".to_string(),
                    description: "Delete a relationship between tables".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "from_table": {
                                "type": "string",
                                "description": "Source table name"
                            },
                            "from_column": {
                                "type": "string",
                                "description": "Source column name"
                            },
                            "to_table": {
                                "type": "string",
                                "description": "Target table name"
                            },
                            "to_column": {
                                "type": "string",
                                "description": "Target column name"
                            }
                        },
                        "required": ["from_table", "from_column", "to_table", "to_column"]
                    }),
                },
            },
        ]);
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AiConfig::default();
        assert_eq!(config.api_base, DEFAULT_API_BASE);
        assert_eq!(config.model, DEFAULT_MODEL);
        assert!(!config.has_api_key());
        assert_eq!(config.mode, AiMode::Ask);
    }

    #[test]
    fn test_mode_can_write() {
        assert!(!AiMode::Ask.can_write());
        assert!(AiMode::Write.can_write());
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("Hello");
        assert_eq!(system.role, MessageRole::System);

        let user = ChatMessage::user("Hi");
        assert_eq!(user.role, MessageRole::User);

        let assistant = ChatMessage::assistant("Hello!");
        assert_eq!(assistant.role, MessageRole::Assistant);
    }

    #[test]
    fn test_tool_definitions_ask_mode() {
        let tools = build_tool_definitions(AiMode::Ask);
        // Should only have read tools
        assert!(tools.iter().all(|t| {
            let name = &t.function.name;
            name.starts_with("get_") || name.starts_with("list_")
        }));
    }

    #[test]
    fn test_tool_definitions_write_mode() {
        let tools = build_tool_definitions(AiMode::Write);
        // Should have both read and write tools
        assert!(tools.iter().any(|t| t.function.name == "create_table"));
        assert!(tools.iter().any(|t| t.function.name == "get_schema_json"));
    }
}
