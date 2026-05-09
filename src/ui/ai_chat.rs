//! AI Chat Panel component
//!
//! Provides a chat interface for interacting with the AI assistant.
//! Supports two modes:
//! - Ask: Read-only, can only analyze the schema
//! - Write: Full access, can modify the schema

use crate::core::SchemaGraph;
use crate::core::ai_config::{AiConfig, AiMode, ChatMessage, MessageRole};
#[cfg(not(feature = "ssr"))]
use crate::core::ai_config::{FunctionCall, StreamChunk, ToolCall};
#[cfg(not(feature = "ssr"))]
use crate::core::{ToolExecutor, ToolRequest};
#[cfg(not(feature = "ssr"))]
use crate::ui::liveshare_client::{ConnectionState, try_use_liveshare_context};
use crate::ui::markdown::Markdown;
use crate::ui::{Icon, icons};
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
use leptos::wasm_bindgen::JsCast;

/// Storage key for AI config in localStorage
#[cfg(not(feature = "ssr"))]
const AI_CONFIG_STORAGE_KEY: &str = "archischema_ai_config";

/// AI Chat Panel component
#[component]
pub fn AiChatPanel(
    /// Whether the panel is open
    is_open: RwSignal<bool>,
    /// Schema graph for tool execution
    _graph: RwSignal<SchemaGraph>,
) -> impl IntoView {
    // Chat state
    let (messages, set_messages) = signal::<Vec<ChatMessage>>(Vec::new());
    let (input_value, set_input_value) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    // Streaming content for real-time display
    #[allow(unused_variables)]
    let (streaming_content, set_streaming_content) = signal(String::new());

    // AI config state
    let (config, set_config) = signal(AiConfig::default());
    let (show_settings, set_show_settings) = signal(false);

    // Settings form state
    let (settings_api_key, set_settings_api_key) = signal(String::new());
    let (settings_model, set_settings_model) = signal(String::new());
    let (settings_api_base, set_settings_api_base) = signal(String::new());

    // Load config from localStorage on mount
    #[cfg(not(feature = "ssr"))]
    {
        Effect::new(move |_| {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(stored)) = storage.get_item(AI_CONFIG_STORAGE_KEY) {
                        if let Ok(loaded_config) = serde_json::from_str::<AiConfig>(&stored) {
                            set_config.set(loaded_config.clone());
                            set_settings_api_key.set(loaded_config.api_key.unwrap_or_default());
                            set_settings_model.set(loaded_config.model);
                            set_settings_api_base.set(loaded_config.api_base);
                        }
                    }
                }
            }
        });
    }

    // Save config to localStorage
    let save_config = move |new_config: AiConfig| {
        #[cfg(not(feature = "ssr"))]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(json) = serde_json::to_string(&new_config) {
                        let _ = storage.set_item(AI_CONFIG_STORAGE_KEY, &json);
                    }
                }
            }
        }
        set_config.set(new_config);
    };

    // Set assistant mode from the compact composer segmented control.
    let set_mode = move |new_mode: AiMode| {
        let current = config.get();
        if current.mode == new_mode {
            return;
        }
        let new_config = AiConfig {
            mode: new_mode,
            ..current
        };
        save_config(new_config);
    };

    // Save settings
    let save_settings = move |_| {
        let current = config.get();
        let new_config = AiConfig {
            api_key: {
                let key = settings_api_key.get();
                if key.is_empty() { None } else { Some(key) }
            },
            model: {
                let model = settings_model.get();
                if model.is_empty() {
                    current.model
                } else {
                    model
                }
            },
            api_base: {
                let base = settings_api_base.get();
                if base.is_empty() {
                    current.api_base
                } else {
                    base
                }
            },
            ..current
        };
        save_config(new_config);
        set_show_settings.set(false);
    };

    // Open settings and populate form
    let open_settings = move |_| {
        let current = config.get();
        set_settings_api_key.set(current.api_key.unwrap_or_default());
        set_settings_model.set(current.model);
        set_settings_api_base.set(current.api_base);
        set_show_settings.set(true);
    };

    // Send message action
    let send_message = move |_| {
        let input = input_value.get();
        if input.trim().is_empty() || is_loading.get() {
            return;
        }

        set_input_value.set(String::new());
        set_error_message.set(None);

        // Add user message
        let user_message = ChatMessage::user(input.clone());
        set_messages.update(|msgs| msgs.push(user_message));

        set_is_loading.set(true);

        // Get current config
        let current_config = config.get();

        // Spawn async task to call API
        #[cfg(not(feature = "ssr"))]
        {
            use crate::core::ai_config::{ChatRequest, build_tool_definitions};

            leptos::task::spawn_local(async move {
                // Build messages with system prompt
                // Note: user message was already added to messages signal above
                let mut api_messages = vec![ChatMessage::system(&current_config.system_prompt)];
                api_messages.extend(messages.with_untracked(|v| v.clone()));

                // Build request
                let tools = build_tool_definitions(current_config.mode);
                let request = ChatRequest {
                    model: current_config.model.clone(),
                    messages: api_messages.clone(),
                    tools: Some(tools),
                    temperature: Some(current_config.temperature),
                    max_tokens: Some(current_config.max_tokens),
                    stream: None, // Will be set by streaming function
                };

                // Determine API key - use user's key or try server-side default
                let api_key = current_config.api_key.clone();

                // Clear streaming content
                set_streaming_content.set(String::new());

                // Use streaming API
                let accumulated_content = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
                let accumulated_tool_calls =
                    std::rc::Rc::new(std::cell::RefCell::new(Vec::<ToolCall>::new()));

                let content_clone = accumulated_content.clone();
                let tool_calls_clone = accumulated_tool_calls.clone();

                let stream_result = call_ai_api_streaming(
                    &current_config.api_base,
                    api_key.as_deref(),
                    &request,
                    move |chunk| {
                        // Append to accumulated content
                        content_clone.borrow_mut().push_str(&chunk);
                        // Update streaming display
                        set_streaming_content.set(content_clone.borrow().clone());
                    },
                    move |tool_call| {
                        // Collect tool calls
                        tool_calls_clone.borrow_mut().push(tool_call);
                    },
                )
                .await;

                match stream_result {
                    Ok(()) => {
                        let final_content = accumulated_content.borrow().clone();
                        let tool_calls = accumulated_tool_calls.borrow().clone();

                        // Clear streaming content
                        set_streaming_content.set(String::new());

                        if !tool_calls.is_empty() {
                            // Execute tools and continue conversation
                            let mut current_messages = api_messages.clone();

                            // Add assistant message with tool calls
                            let assistant_message = ChatMessage {
                                role: MessageRole::Assistant,
                                content: final_content.clone(),
                                tool_call_id: None,
                                tool_calls: Some(tool_calls.clone()),
                            };
                            current_messages.push(assistant_message.clone());
                            set_messages.update(|msgs| msgs.push(assistant_message));

                            // Execute each tool call
                            for tool_call in &tool_calls {
                                let tool_name = &tool_call.function.name;
                                let arguments = &tool_call.function.arguments;

                                // Check if write operation is allowed
                                let is_write_op = matches!(
                                    tool_name.as_str(),
                                    "create_table"
                                        | "rename_table"
                                        | "delete_table"
                                        | "add_column"
                                        | "modify_column"
                                        | "delete_column"
                                        | "create_relationship"
                                        | "delete_relationship"
                                        | "apply_sql"
                                );

                                let tool_result = if is_write_op && !current_config.mode.can_write()
                                {
                                    serde_json::json!({
                                        "success": false,
                                        "error": "Write operations are not allowed in Ask mode. Switch to Write mode to modify the schema."
                                    }).to_string()
                                } else {
                                    // Parse arguments and execute tool
                                    let params: serde_json::Value =
                                        serde_json::from_str(arguments).unwrap_or_default();

                                    let tool_request = ToolRequest {
                                        tool_name: tool_name.clone(),
                                        parameters: params,
                                    };

                                    // Execute on graph (need to update signal)
                                    let result = _graph
                                        .try_update(|g| ToolExecutor::execute(g, &tool_request));

                                    match result {
                                        Some(response) => {
                                            // Send graph operations to LiveShare for sync
                                            if !response.graph_ops.is_empty() {
                                                if let Some(liveshare_ctx) = try_use_liveshare_context() {
                                                    if liveshare_ctx.connection_state.with_untracked(|v| *v) == ConnectionState::Connected {
                                                        for op in &response.graph_ops {
                                                            liveshare_ctx.send_graph_op(op.clone());
                                                        }
                                                    }
                                                }
                                            }
                                            serde_json::to_string(&response)
                                                .unwrap_or_else(|_| "Error serializing response".to_string())
                                        }
                                        None => r#"{"success": false, "error": "Failed to access graph"}"#.to_string(),
                                    }
                                };

                                // Add tool response message
                                let tool_response_msg =
                                    ChatMessage::tool_response(&tool_call.id, &tool_result);
                                current_messages.push(tool_response_msg.clone());
                            }

                            // Make another streaming API call with tool results
                            let tools = build_tool_definitions(current_config.mode);
                            let follow_up_request = ChatRequest {
                                model: current_config.model.clone(),
                                messages: current_messages,
                                tools: Some(tools),
                                temperature: Some(current_config.temperature),
                                max_tokens: Some(current_config.max_tokens),
                                stream: None,
                            };

                            let follow_up_content =
                                std::rc::Rc::new(std::cell::RefCell::new(String::new()));
                            let follow_up_clone = follow_up_content.clone();

                            let follow_up_result = call_ai_api_streaming(
                                &current_config.api_base,
                                api_key.as_deref(),
                                &follow_up_request,
                                move |chunk| {
                                    follow_up_clone.borrow_mut().push_str(&chunk);
                                    set_streaming_content.set(follow_up_clone.borrow().clone());
                                },
                                |_| {}, // Ignore tool calls in follow-up
                            )
                            .await;

                            set_streaming_content.set(String::new());

                            match follow_up_result {
                                Ok(()) => {
                                    let content = follow_up_content.borrow().clone();
                                    if !content.trim().is_empty() {
                                        set_messages.update(|msgs| {
                                            msgs.push(ChatMessage::assistant(&content));
                                        });
                                    } else {
                                        set_messages.update(|msgs| {
                                            msgs.push(ChatMessage::assistant(
                                                "✅ Done! The requested changes have been applied to the schema.",
                                            ));
                                        });
                                    }
                                }
                                Err(e) => {
                                    set_error_message.set(Some(e));
                                }
                            }
                        } else if !final_content.trim().is_empty() {
                            // No tool calls, just add the response
                            set_messages.update(|msgs| {
                                msgs.push(ChatMessage::assistant(&final_content));
                            });
                        }
                    }
                    Err(e) => {
                        set_streaming_content.set(String::new());
                        set_error_message.set(Some(e));
                    }
                }

                set_is_loading.set(false);
            });
        }

        #[cfg(feature = "ssr")]
        {
            // SSR placeholder - will be hydrated
            let _ = current_config;
            set_is_loading.set(false);
        }
    };

    // Clear chat
    let clear_chat = move |_| {
        set_messages.set(Vec::new());
        set_error_message.set(None);
    };

    // Close panel on Escape key
    #[cfg(not(feature = "ssr"))]
    {
        use leptos::ev::keydown;

        let handle_keydown = window_event_listener(keydown, move |ev| {
            if ev.key() == "Escape" && is_open.with_untracked(|v| *v) {
                if show_settings.with_untracked(|v| *v) {
                    set_show_settings.set(false);
                } else {
                    is_open.set(false);
                }
            }
        });

        on_cleanup(move || drop(handle_keydown));
    }

    view! {
        <aside class=move || if is_open.get() { "ai-dock-panel" } else { "hidden" }>
            <div class="ai-panel-shell theme-transition">
                <div class="ai-panel-header">
                    <span class="ai-mark">
                        <Icon name=icons::SPARKLES class="icon-text"/>
                    </span>
                    <div class="min-w-0 flex-1">
                        <div class="ai-title">"Assistant"</div>
                        <div class="ai-thread-meta">
                            {move || format!("Current diagram - {} turns", messages.get().len())}
                        </div>
                    </div>
                    <button class="btn-icon btn-sm" on:click=clear_chat title="New thread">
                        <Icon name=icons::PLUS class="icon-btn"/>
                    </button>
                    <button class="btn-icon btn-sm" on:click=open_settings title="AI settings">
                        <Icon name=icons::SETTINGS class="icon-btn"/>
                    </button>
                    <button class="btn-icon btn-sm" on:click=move |_| is_open.set(false) title="Close assistant">
                        <Icon name=icons::X class="icon-btn"/>
                    </button>
                </div>

                {move || {
                    if show_settings.get() {
                        view! {
                            <div class="ai-settings-overlay">
                                <div class="ai-panel-header">
                                    <span class="ai-mark"><Icon name=icons::SETTINGS class="icon-text"/></span>
                                    <div class="min-w-0 flex-1">
                                        <div class="ai-title">"AI assistant settings"</div>
                                        <div class="ai-thread-meta">"OpenRouter-compatible proxy - stored locally"</div>
                                    </div>
                                    <button class="btn-icon btn-sm" on:click=move |_| set_show_settings.set(false) title="Close settings">
                                        <Icon name=icons::X class="icon-btn"/>
                                    </button>
                                </div>
                                <div class="ai-settings-body scroll">
                                    <div class="ai-settings-field">
                                        <label class="label" for="ai-provider">"Provider"</label>
                                        <div id="ai-provider" class="flex flex-wrap gap-2">
                                            <button class="btn-sm btn-secondary" disabled=true>"OpenRouter"</button>
                                            <button class="btn-sm btn-secondary" disabled=true>"Custom"</button>
                                            <button class="btn-sm btn-secondary" disabled=true>"Local (soon)"</button>
                                        </div>
                                        <p class="subtitle mt-2">"Provider switching is UI-only in this pass; the existing proxy remains unchanged."</p>
                                    </div>

                                    <div class="ai-settings-field">
                                        <label class="label" for="ai-api-key">"API key"</label>
                                        <input
                                            id="ai-api-key"
                                            type="password"
                                            autocomplete="off"
                                            class="input-base input-lg"
                                            placeholder="sk-or-v1-..."
                                            prop:value=move || settings_api_key.get()
                                            on:input=move |e| set_settings_api_key.set(event_target_value(&e))
                                        />
                                        <p class="subtitle mt-2">"Leave empty to use the server default. This value is saved in localStorage."</p>
                                    </div>

                                    <div class="ai-settings-field">
                                        <label class="label" for="ai-model">"Model"</label>
                                        <input
                                            id="ai-model"
                                            type="text"
                                            autocomplete="off"
                                            class="input-base input-lg mono"
                                            placeholder="google/gemini-2.5-flash-lite"
                                            prop:value=move || settings_model.get()
                                            on:input=move |e| set_settings_model.set(event_target_value(&e))
                                        />
                                        <p class="subtitle mt-2">"Use any OpenRouter model id supported by the current API key."</p>
                                    </div>

                                    <div class="ai-settings-field">
                                        <label class="label" for="ai-api-base">"Base URL"</label>
                                        <input
                                            id="ai-api-base"
                                            type="url"
                                            autocomplete="off"
                                            class="input-base input-lg mono"
                                            placeholder="https://openrouter.ai/api/v1/chat/completions"
                                            prop:value=move || settings_api_base.get()
                                            on:input=move |e| set_settings_api_base.set(event_target_value(&e))
                                        />
                                        <p class="subtitle mt-2">"Kept for compatibility with the existing config shape."</p>
                                    </div>

                                    <div class="ai-settings-field">
                                        <div class="label">"Tool permissions"</div>
                                        <div class="grid grid-cols-2 gap-2">
                                            <span class="ai-permission-chip can-write"><Icon name=icons::DATABASE class="icon-text"/>"Read schema"</span>
                                            <span class="ai-permission-chip can-write"><Icon name=icons::PLUS class="icon-text"/>"Create tables"</span>
                                            <span class="ai-permission-chip can-write"><Icon name=icons::EDIT class="icon-text"/>"Add columns"</span>
                                            <span class="ai-permission-chip"><Icon name=icons::LOCK class="icon-text"/>"Run migrations"</span>
                                        </div>
                                        <p class="subtitle mt-2">"Effective access is still controlled by Ask/Write mode and existing tool checks."</p>
                                    </div>
                                </div>
                                <div class="ai-settings-footer">
                                    <button class="btn-secondary" on:click=move |_| set_show_settings.set(false)>"Cancel"</button>
                                    <button class="btn-primary" on:click=save_settings>"Save settings"</button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="hidden"></div> }.into_any()
                    }
                }}

                <div class="ai-conversation scroll">
                    {move || {
                        if messages.get().is_empty() {
                            view! {
                                <div class="ai-empty-card">
                                    <span class="ai-empty-icon"><Icon name=icons::BOT class="icon-lg"/></span>
                                    <div class="eyebrow mb-2">"Copilot ready"</div>
                                    <h3 class="title-lg">"Ask about the schema, or describe a change."</h3>
                                    <p class="subtitle mt-2">
                                        "Ask is read-only. Write may call tools that update the current diagram and sync through LiveShare."
                                    </p>
                                    <div class="ai-suggestion-grid">
                                        <button class="ai-suggestion-card" on:click=move |_| set_input_value.set("List the tables and relationships in this diagram".to_string())>
                                            <Icon name=icons::SEARCH class="icon-text"/>
                                            <span><strong>"Summarize schema"</strong><br/><span class="subtitle">"Tables, columns, relationships"</span></span>
                                        </button>
                                        <button class="ai-suggestion-card" on:click=move |_| set_input_value.set("Add audit columns to every table".to_string())>
                                            <Icon name=icons::SPARKLES class="icon-text"/>
                                            <span><strong>"Add audit columns"</strong><br/><span class="subtitle">"created_at, updated_at, deleted_at"</span></span>
                                        </button>
                                        <button class="ai-suggestion-card" on:click=move |_| set_input_value.set("Show me the schema as SQL".to_string())>
                                            <Icon name=icons::CODE class="icon-text"/>
                                            <span><strong>"Export SQL"</strong><br/><span class="subtitle">"Generate DDL from canvas"</span></span>
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}

                    <For
                        each=move || messages.get().into_iter().enumerate()
                        key=|(i, _)| *i
                        children=move |(_, message)| {
                            let is_user = message.role == MessageRole::User;
                            let is_tool = message.role == MessageRole::Tool;
                            let is_assistant = message.role == MessageRole::Assistant;
                            let content = message.content.clone();
                            let tool_calls = message.tool_calls.clone().unwrap_or_default();
                            let tool_call_count = tool_calls.len();
                            let role_label = if is_user {
                                "You"
                            } else if is_tool {
                                "Tool result"
                            } else {
                                "Assistant"
                            };
                            let tool_failed = content.contains("\"success\":false")
                                || content.contains("\"success\": false")
                                || content.contains("\"error\":");
                            let tool_summary = if is_tool {
                                serde_json::from_str::<serde_json::Value>(&content)
                                    .ok()
                                    .and_then(|value| {
                                        value
                                            .get("message")
                                            .and_then(|v| v.as_str())
                                            .or_else(|| value.get("error").and_then(|v| v.as_str()))
                                            .map(str::to_string)
                                    })
                                    .unwrap_or_else(|| content.clone())
                            } else {
                                content.clone()
                            };

                            view! {
                                <div class=if is_user { "ai-message-wrap ai-message-wrap-user" } else { "ai-message-wrap" }>
                                    <span class="ai-message-eyebrow">{role_label}</span>
                                    {if is_tool {
                                        let class_name = if tool_failed { "ai-tool-card ai-tool-card-error" } else { "ai-tool-card ai-tool-card-success" };
                                        let status_icon = if tool_failed { icons::ALERT_CIRCLE } else { icons::CHECK };
                                        view! {
                                            <div class=class_name>
                                                <div class="ai-tool-card-head">
                                                    <Icon name=status_icon class="icon-text"/>
                                                    <span>"Tool response"</span>
                                                </div>
                                                <div class="ai-tool-card-meta">{tool_summary.clone()}</div>
                                            </div>
                                        }.into_any()
                                    } else if is_assistant && !content.is_empty() {
                                        view! {
                                            <div class="ai-message">
                                                <Markdown content=content.clone() />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class=if is_user { "ai-message ai-message-user" } else { "ai-message" }>
                                                <p class="whitespace-pre-wrap break-words">{content.clone()}</p>
                                            </div>
                                        }.into_any()
                                    }}

                                    {if !tool_calls.is_empty() {
                                        view! {
                                            <div class="ai-tool-stack">
                                                <span class="ai-message-eyebrow">
                                                    {format!("Assistant - {} tool call{}", tool_call_count, if tool_call_count == 1 { "" } else { "s" })}
                                                </span>
                                                <For
                                                    each=move || tool_calls.clone().into_iter()
                                                    key=|tool_call| tool_call.id.clone()
                                                    children=move |tool_call| {
                                                        let tool_name = tool_call.function.name.replace('_', " ");
                                                        let arguments = tool_call.function.arguments.clone();
                                                        view! {
                                                            <div class="ai-tool-card ai-tool-card-plan">
                                                                <div class="ai-tool-card-head">
                                                                    <Icon name=icons::GIT_BRANCH class="icon-text"/>
                                                                    <span class="mono">{tool_name}</span>
                                                                    <span class="ai-context-chip">"tool call"</span>
                                                                </div>
                                                                <div class="ai-tool-card-meta">{arguments}</div>
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </div>
                            }
                        }
                    />

                    {move || {
                        let content = streaming_content.get();
                        if is_loading.get() && !content.is_empty() {
                            view! {
                                <div class="ai-message-wrap">
                                    <span class="ai-message-eyebrow">"Assistant - streaming"</span>
                                    <div class="ai-message">
                                        <Markdown content=content />
                                        <span class="ai-stream-caret"></span>
                                    </div>
                                </div>
                            }.into_any()
                        } else if is_loading.get() {
                            view! {
                                <div class="ai-message-wrap">
                                    <span class="ai-message-eyebrow">"Assistant"</span>
                                    <div class="ai-tool-card ai-tool-card-running">
                                        <div class="ai-tool-card-head">
                                            <Icon name=icons::LOADER class="icon-text icon-spin"/>
                                            <span>"Reading context"</span>
                                        </div>
                                        <div class="ai-tool-card-meta">"Waiting for the first streamed token..."</div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}

                    {move || {
                        if let Some(err) = error_message.get() {
                            view! {
                                <div class="ai-error-card">
                                    <Icon name=icons::ALERT_CIRCLE class="icon-text"/>
                                    <div>
                                        <strong>"Request failed"</strong>
                                        <div>{err}</div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}
                </div>

                {move || {
                    if messages.get().is_empty() {
                        view! { <div class="hidden"></div> }.into_any()
                    } else {
                        view! {
                            <div class="ai-suggestion-strip">
                                <button class="chip" on:click=move |_| set_input_value.set("Index hot foreign keys".to_string())>
                                    <Icon name=icons::SPARKLES class="icon-text"/>"Index hot FKs"
                                </button>
                                <button class="chip" on:click=move |_| set_input_value.set("Add audit columns".to_string())>
                                    <Icon name=icons::CLOCK class="icon-text"/>"Add audit columns"
                                </button>
                                <button class="chip" on:click=move |_| set_input_value.set("Convert this schema to PostgreSQL DDL".to_string())>
                                    <Icon name=icons::CODE class="icon-text"/>"Postgres DDL"
                                </button>
                            </div>
                        }.into_any()
                    }
                }}

                <div class="ai-composer-wrap">
                    <div class="ai-composer-shell">
                        <textarea
                            class="ai-composer-input"
                            placeholder="Ask anything, or describe a schema change..."
                            rows="2"
                            prop:value=move || input_value.get()
                            on:input=move |e| set_input_value.set(event_target_value(&e))
                            on:keydown=move |e| {
                                if e.key() == "Enter" && !e.shift_key() {
                                    e.prevent_default();
                                    send_message(());
                                }
                            }
                        />
                        <div class="ai-composer-toolbar">
                            <div class="ai-mode-switch" aria-label="Assistant mode">
                                <button
                                    class=move || if config.get().mode == AiMode::Ask { "ai-mode-option is-active" } else { "ai-mode-option" }
                                    type="button"
                                    on:click=move |_| set_mode(AiMode::Ask)
                                >
                                    <Icon name=icons::SEARCH class="icon-text"/>"Ask"
                                </button>
                                <button
                                    class=move || if config.get().mode == AiMode::Write { "ai-mode-option is-active" } else { "ai-mode-option" }
                                    type="button"
                                    on:click=move |_| set_mode(AiMode::Write)
                                >
                                    <Icon name=icons::EDIT class="icon-text"/>"Write"
                                </button>
                            </div>
                            <span class="ai-context-chip"><Icon name=icons::DATABASE class="icon-text"/>"schema"</span>
                            <span class=move || if config.get().mode == AiMode::Write { "ai-permission-chip can-write" } else { "ai-permission-chip" }>
                                {move || if config.get().mode == AiMode::Write { "tools enabled" } else { "read only" }}
                            </span>
                            <div class="flex-1"></div>
                            <button
                                class=move || if is_loading.get() || input_value.get().trim().is_empty() { "ai-send-button is-disabled" } else { "ai-send-button" }
                                type="button"
                                on:click=move |_| send_message(())
                                disabled=move || is_loading.get() || input_value.get().trim().is_empty()
                                title="Send message"
                            >
                                <Icon name=icons::SEND class="icon-btn"/>
                            </button>
                        </div>
                    </div>
                    <div class="ai-composer-meta">
                        <span>{move || config.get().model}</span>
                        <span>"Enter send - Shift+Enter newline"</span>
                    </div>
                </div>
            </div>
        </aside>
    }
}

/// AI Chat button component (placed above settings button)
#[component]
pub fn AiChatButton(
    /// Signal to control panel visibility
    is_open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            class="fixed bottom-20 right-4 z-40 flex items-center justify-center w-12 h-12 bg-theme-surface border border-theme-primary text-theme-secondary hover:text-theme-accent hover:border-theme-accent theme-transition transition-colors"
            style="border-radius: 12px; box-shadow: var(--shadow-lg);"
            on:click=move |_| is_open.set(true)
            title="AI Assistant"
        >
            <Icon name=icons::BOT class="w-6 h-6"/>
        </button>
    }
}

/// Call the AI API through our server proxy
#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
async fn call_ai_api(
    _api_base: &str, // Ignored - we use server proxy
    api_key: Option<&str>,
    request: &crate::core::ai_config::ChatRequest,
) -> Result<crate::core::ai_config::ChatResponse, String> {
    use leptos::wasm_bindgen::JsValue;
    use web_sys::{Headers, Request, RequestInit, Response};

    let window = web_sys::window().ok_or("No window object")?;

    // Build headers
    let headers = Headers::new().map_err(|e| format!("Failed to create headers: {:?}", e))?;

    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set content-type: {:?}", e))?;

    // If user provided their own API key, pass it to the server
    if let Some(key) = api_key {
        if !key.is_empty() {
            headers
                .set("X-API-Key", key)
                .map_err(|e| format!("Failed to set X-API-Key: {:?}", e))?;
        }
    }

    // Build request body
    let body = serde_json::to_string(request).map_err(|e| format!("Failed to serialize: {}", e))?;

    // Build request options
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&JsValue::from_str(&body));

    // Use our server proxy endpoint instead of calling OpenRouter directly
    let proxy_url = "/api/ai/chat";

    // Create request
    let request = Request::new_with_str_and_init(proxy_url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    // Fetch
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        let status = resp.status();
        let status_text = resp.status_text();

        // Try to get error body
        if let Ok(text_promise) = resp.text() {
            if let Ok(text_value) = wasm_bindgen_futures::JsFuture::from(text_promise).await {
                if let Some(text) = text_value.as_string() {
                    return Err(format!("API error {}: {} - {}", status, status_text, text));
                }
            }
        }

        return Err(format!("API error {}: {}", status, status_text));
    }

    // Parse response
    let text_promise = resp
        .text()
        .map_err(|e| format!("Failed to get response text: {:?}", e))?;

    let text_value = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Failed to read response: {:?}", e))?;

    let text = text_value.as_string().ok_or("Response is not a string")?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse response: {} - {}", e, text))
}

/// Call the AI API with streaming through our server proxy
#[cfg(not(feature = "ssr"))]
async fn call_ai_api_streaming(
    _api_base: &str,
    api_key: Option<&str>,
    request: &crate::core::ai_config::ChatRequest,
    on_chunk: impl Fn(String) + 'static,
    on_tool_call: impl Fn(ToolCall) + 'static,
) -> Result<(), String> {
    use js_sys::{Reflect, Uint8Array};
    use leptos::wasm_bindgen::JsValue;
    use web_sys::{Headers, ReadableStreamDefaultReader, Request, RequestInit, Response};

    let window = web_sys::window().ok_or("No window object")?;

    // Build headers
    let headers = Headers::new().map_err(|e| format!("Failed to create headers: {:?}", e))?;

    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set content-type: {:?}", e))?;

    // If user provided their own API key, pass it to the server
    if let Some(key) = api_key {
        if !key.is_empty() {
            headers
                .set("X-API-Key", key)
                .map_err(|e| format!("Failed to set X-API-Key: {:?}", e))?;
        }
    }

    // Enable streaming in request
    let mut stream_request = request.clone();
    stream_request.stream = Some(true);

    // Build request body
    let body = serde_json::to_string(&stream_request)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    // Build request options
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&JsValue::from_str(&body));

    // Use our server proxy streaming endpoint
    let proxy_url = "/api/ai/chat/stream";

    // Create request
    let request = Request::new_with_str_and_init(proxy_url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    // Fetch
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        let status = resp.status();
        let status_text = resp.status_text();

        if let Ok(text_promise) = resp.text() {
            if let Ok(text_value) = wasm_bindgen_futures::JsFuture::from(text_promise).await {
                if let Some(text) = text_value.as_string() {
                    return Err(format!("API error {}: {} - {}", status, status_text, text));
                }
            }
        }

        return Err(format!("API error {}: {}", status, status_text));
    }

    // Get the response body as a stream
    let body = resp.body().ok_or("No response body")?;
    let reader: ReadableStreamDefaultReader = body
        .get_reader()
        .dyn_into()
        .map_err(|_| "Failed to get stream reader")?;

    let mut buffer = String::new();
    let mut accumulated_tool_calls: std::collections::HashMap<
        u32,
        (String, String, String, String),
    > = std::collections::HashMap::new();

    loop {
        let result = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("Failed to read stream: {:?}", e))?;

        let done = Reflect::get(&result, &JsValue::from_str("done"))
            .map_err(|_| "Failed to get done property")?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let value = Reflect::get(&result, &JsValue::from_str("value"))
            .map_err(|_| "Failed to get value property")?;

        if !value.is_undefined() {
            let array: Uint8Array = value.dyn_into().map_err(|_| "Value is not a Uint8Array")?;
            let bytes = array.to_vec();
            let chunk_str = String::from_utf8_lossy(&bytes);
            buffer.push_str(&chunk_str);

            // Process complete SSE lines
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        for choice in &chunk.choices {
                            // Handle content delta
                            if let Some(content) = &choice.delta.content {
                                if !content.is_empty() {
                                    on_chunk(content.clone());
                                }
                            }

                            // Handle tool calls delta
                            if let Some(tool_calls) = &choice.delta.tool_calls {
                                for tc in tool_calls {
                                    let entry = accumulated_tool_calls
                                        .entry(tc.index)
                                        .or_insert_with(|| {
                                            (
                                                String::new(),
                                                String::new(),
                                                String::new(),
                                                String::new(),
                                            )
                                        });

                                    if let Some(id) = &tc.id {
                                        entry.0 = id.clone();
                                    }
                                    if let Some(call_type) = &tc.call_type {
                                        entry.1 = call_type.clone();
                                    }
                                    if let Some(func) = &tc.function {
                                        if let Some(name) = &func.name {
                                            entry.2.push_str(name);
                                        }
                                        if let Some(args) = &func.arguments {
                                            entry.3.push_str(args);
                                        }
                                    }
                                }
                            }

                            // Check if streaming is done for this choice
                            if choice.finish_reason.is_some() {
                                // Emit accumulated tool calls
                                for (_, (id, call_type, name, args)) in
                                    accumulated_tool_calls.drain()
                                {
                                    if !id.is_empty() && !name.is_empty() {
                                        on_tool_call(ToolCall {
                                            id,
                                            call_type: if call_type.is_empty() {
                                                "function".to_string()
                                            } else {
                                                call_type
                                            },
                                            function: FunctionCall {
                                                name,
                                                arguments: args,
                                            },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
