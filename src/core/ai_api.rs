//! AI API server-side handler
//!
//! This module provides a server-side proxy for AI API requests.
//! It keeps the API key secure on the server and supports streaming responses.

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc;

/// AI API configuration loaded from environment
#[derive(Clone)]
pub struct AiApiConfig {
    pub api_base: String,
    pub api_token: Option<String>,
    pub default_model: String,
}

impl AiApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            api_base: std::env::var("OPENAPI_BASE")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".to_string()),
            api_token: std::env::var("OPENAPI_TOKEN").ok(),
            default_model: std::env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "google/gemini-2.5-flash-lite".to_string()),
        }
    }

    /// Check if API token is configured
    pub fn has_token(&self) -> bool {
        self.api_token.as_ref().is_some_and(|t| !t.is_empty())
    }
}

/// Chat message for API requests
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

/// Tool definition for API requests
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: serde_json::Value,
}

/// Chat completion request from client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub message: String,
    pub code: u16,
}

/// Create the AI API router
pub fn ai_api_router(config: AiApiConfig) -> Router {
    let state = Arc::new(config);

    Router::new()
        .route("/api/ai/chat", post(chat_handler))
        .route("/api/ai/chat/stream", post(chat_stream_handler))
        .with_state(state)
}

/// Non-streaming chat handler
async fn chat_handler(
    State(config): State<Arc<AiApiConfig>>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Response {
    // Use default model if not specified
    if request.model.is_none() {
        request.model = Some(config.default_model.clone());
    }

    tracing::info!(
        "AI chat request: model={:?}, messages_count={}",
        request.model,
        request.messages.len()
    );

    // Ensure stream is false for this endpoint
    request.stream = Some(false);

    // Get API token (from request header or server config)
    let api_token = get_api_token(&headers, &config);

    if api_token.is_none() {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "No API key configured. Please set OPENAPI_TOKEN in .env or provide your own key.",
        );
    }

    // Make request to OpenRouter
    let client = reqwest::Client::new();

    let response = client
        .post(&config.api_base)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_token.unwrap()))
        .header("HTTP-Referer", get_referer(&headers))
        .header("X-Title", "Archischema")
        .json(&request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            tracing::info!("AI API response status: {}", status);
            match resp.text().await {
                Ok(body) => {
                    // Log response for debugging (truncate if too long)
                    let log_body = if body.len() > 500 {
                        format!(
                            "{}... (truncated, total {} bytes)",
                            &body[..500],
                            body.len()
                        )
                    } else {
                        body.clone()
                    };
                    tracing::debug!("AI API response body: {}", log_body);

                    if status.is_success() {
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(Body::from(body))
                            .unwrap()
                    } else {
                        tracing::warn!("AI API error response: {}", body);
                        Response::builder()
                            .status(
                                StatusCode::from_u16(status.as_u16())
                                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                            )
                            .header("Content-Type", "application/json")
                            .body(Body::from(body))
                            .unwrap()
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read AI API response: {}", e);
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Failed to read response: {}", e),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to connect to AI API: {}", e);
            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to connect to AI API: {}", e),
            )
        }
    }
}

/// Streaming chat handler
async fn chat_stream_handler(
    State(config): State<Arc<AiApiConfig>>,
    headers: HeaderMap,
    Json(mut request): Json<ChatRequest>,
) -> Response {
    // Use default model if not specified
    if request.model.is_none() {
        request.model = Some(config.default_model.clone());
    }

    // Ensure stream is true for this endpoint
    request.stream = Some(true);

    // Get API token (from request header or server config)
    let api_token = get_api_token(&headers, &config);

    if api_token.is_none() {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "No API key configured. Please set OPENAPI_TOKEN in .env or provide your own key.",
        );
    }

    let api_base = config.api_base.clone();
    let referer = get_referer(&headers);
    let token = api_token.unwrap();

    // Create a channel for streaming
    let (tx, rx) = mpsc::channel::<Result<String, Infallible>>(32);

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let response = client
            .post(&api_base)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .header("HTTP-Referer", referer)
            .header("X-Title", "Archischema")
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let error_body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    let error_event =
                        format!("data: {{\"error\": {}}}\n\n", serde_json::json!(error_body));
                    let _ = tx.send(Ok(error_event)).await;
                    return;
                }

                let mut stream = resp.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            // Forward each chunk as-is (SSE format from OpenRouter)
                            if tx.send(Ok(text.to_string())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let error_event = format!("data: {{\"error\": \"{}\"}}\n\n", e);
                            let _ = tx.send(Ok(error_event)).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let error_event = format!("data: {{\"error\": \"{}\"}}\n\n", e);
                let _ = tx.send(Ok(error_event)).await;
            }
        }

        // Send done signal
        let _ = tx.send(Ok("data: [DONE]\n\n".to_string())).await;
    });

    // Convert receiver to stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::from_stream(stream))
        .unwrap()
}

/// Get API token from request header or server config
fn get_api_token(headers: &HeaderMap, config: &AiApiConfig) -> Option<String> {
    // First check if client provided their own key
    if let Some(auth) = headers.get("X-API-Key")
        && let Ok(key) = auth.to_str()
        && !key.is_empty()
    {
        return Some(key.to_string());
    }

    // Fall back to server config
    config.api_token.clone()
}

/// Get referer from headers or use default
fn get_referer(headers: &HeaderMap) -> String {
    headers
        .get("Referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https://archischema.app")
        .to_string()
}

/// Create an error response
fn error_response(status: StatusCode, message: &str) -> Response {
    let error = ErrorResponse {
        error: ErrorDetail {
            message: message.to_string(),
            code: status.as_u16(),
        },
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&error).unwrap()))
        .unwrap()
}
