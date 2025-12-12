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

    // Toggle mode
    let toggle_mode = move |_| {
        let current = config.get();
        let new_mode = if current.mode == AiMode::Ask {
            AiMode::Write
        } else {
            AiMode::Ask
        };
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
                api_messages.extend(messages.get_untracked().clone());

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
                                                    if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
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
            if ev.key() == "Escape" && is_open.get_untracked() {
                if show_settings.get_untracked() {
                    set_show_settings.set(false);
                } else {
                    is_open.set(false);
                }
            }
        });

        on_cleanup(move || drop(handle_keydown));
    }

    view! {
        <div
            class=move || {
                if is_open.get() {
                    "fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm transition-all duration-300"
                } else {
                    "fixed inset-0 z-50 flex items-center justify-center bg-black/0 backdrop-blur-0 transition-all duration-300 pointer-events-none opacity-0"
                }
            }
            on:click=move |e| {
                // Close when clicking backdrop
                #[cfg(not(feature = "ssr"))]
                {
                    let target = e.target();
                    if let Some(el) = target {
                        if let Some(element) = el.dyn_ref::<web_sys::Element>() {
                            if element.class_list().contains("backdrop-blur-sm") {
                                is_open.set(false);
                            }
                        }
                    }
                }
                #[cfg(feature = "ssr")]
                {
                    let _ = e;
                }
            }
        >
            // Main chat panel
            <div class="w-full max-w-2xl h-[80vh] max-h-[700px] bg-theme-surface rounded-2xl shadow-theme-xl flex flex-col overflow-hidden theme-transition">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-theme-primary bg-theme-secondary">
                    <div class="flex items-center gap-3">
                        <div class="w-10 h-10 rounded-lg flex items-center justify-center" style="background: linear-gradient(to bottom right, var(--accent-primary), var(--accent-secondary));">
                            <Icon name=icons::BOT class="w-6 h-6 text-white"/>
                        </div>
                        <div>
                            <h2 class="text-lg font-semibold text-theme-primary">"AI Assistant"</h2>
                            <p class="text-xs text-theme-tertiary">
                                {move || config.get().mode.description()}
                            </p>
                        </div>
                    </div>
                    <div class="flex items-center gap-2">
                        // Mode toggle button
                        <button
                            class=move || {
                                let mode = config.get().mode;
                                if mode == AiMode::Write {
                                    "px-3 py-1.5 text-sm font-medium rounded-lg transition-colors bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                                } else {
                                    "px-3 py-1.5 text-sm font-medium rounded-lg transition-colors bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
                                }
                            }
                            on:click=toggle_mode
                            title="Toggle AI mode"
                        >
                            {move || config.get().mode.display_name()}
                        </button>
                        // Settings button
                        <button
                            class="flex items-center justify-center p-2 text-theme-tertiary hover:text-theme-accent hover:bg-theme-secondary rounded-lg transition-colors"
                            on:click=open_settings
                            title="AI Settings"
                        >
                            <Icon name=icons::SETTINGS class="w-5 h-5"/>
                        </button>
                        // Clear chat button
                        <button
                            class="flex items-center justify-center p-2 text-theme-tertiary hover:text-red-500 hover:bg-theme-secondary rounded-lg transition-colors"
                            on:click=clear_chat
                            title="Clear chat"
                        >
                            <Icon name=icons::TRASH class="w-5 h-5"/>
                        </button>
                        // Close button
                        <button
                            class="flex items-center justify-center p-2 text-theme-tertiary hover:text-theme-primary hover:bg-theme-secondary rounded-lg transition-colors"
                            on:click=move |_| is_open.set(false)
                            title="Close"
                        >
                            <Icon name=icons::X class="w-5 h-5"/>
                        </button>
                    </div>
                </div>

                // Settings panel (overlay)
                {move || {
                    if show_settings.get() {
                        view! {
                            <div class="absolute inset-0 bg-theme-surface z-10 flex flex-col">
                                <div class="flex items-center justify-between px-6 py-4 border-b border-theme-primary">
                                    <h3 class="text-lg font-semibold text-theme-primary">"AI Settings"</h3>
                                    <button
                                        class="flex items-center justify-center p-1 text-theme-tertiary hover:text-theme-primary rounded"
                                        on:click=move |_| set_show_settings.set(false)
                                    >
                                        <Icon name=icons::X class="w-5 h-5"/>
                                    </button>
                                </div>
                                <div class="flex-1 overflow-y-auto p-6 space-y-4">
                                    // API Key
                                    <div>
                                        <label class="block text-sm font-medium text-theme-secondary mb-1.5">"API Key"</label>
                                        <input
                                            type="password"
                                            class="w-full px-3 py-2 bg-theme-tertiary border border-theme-primary rounded-lg text-theme-primary placeholder-theme-muted focus:ring-2 focus:ring-theme-accent focus:border-transparent"
                                            placeholder="sk-or-v1-..."
                                            prop:value=move || settings_api_key.get()
                                            on:input=move |e| set_settings_api_key.set(event_target_value(&e))
                                        />
                                        <p class="mt-1 text-xs text-theme-muted">"Your OpenRouter API key. Leave empty to use server default."</p>
                                    </div>
                                    // Model
                                    <div>
                                        <label class="block text-sm font-medium text-theme-secondary mb-1.5">"Model"</label>
                                        <input
                                            type="text"
                                            class="w-full px-3 py-2 bg-theme-tertiary border border-theme-primary rounded-lg text-theme-primary placeholder-theme-muted focus:ring-2 focus:ring-theme-accent focus:border-transparent"
                                            placeholder="google/gemini-2.5-flash-lite"
                                            prop:value=move || settings_model.get()
                                            on:input=move |e| set_settings_model.set(event_target_value(&e))
                                        />
                                        <p class="mt-1 text-xs text-theme-muted">"Model identifier (e.g., openai/gpt-4o, anthropic/claude-3-opus)"</p>
                                    </div>
                                    // API Base URL
                                    <div>
                                        <label class="block text-sm font-medium text-theme-secondary mb-1.5">"API Base URL"</label>
                                        <input
                                            type="text"
                                            class="w-full px-3 py-2 bg-theme-tertiary border border-theme-primary rounded-lg text-theme-primary placeholder-theme-muted focus:ring-2 focus:ring-theme-accent focus:border-transparent"
                                            placeholder="https://openrouter.ai/api/v1/chat/completions"
                                            prop:value=move || settings_api_base.get()
                                            on:input=move |e| set_settings_api_base.set(event_target_value(&e))
                                        />
                                        <p class="mt-1 text-xs text-theme-muted">"OpenRouter-compatible API endpoint"</p>
                                    </div>
                                </div>
                                <div class="px-6 py-4 border-t border-theme-primary flex justify-end gap-3">
                                    <button
                                        class="px-4 py-2 text-theme-secondary hover:text-theme-primary rounded-lg transition-colors"
                                        on:click=move |_| set_show_settings.set(false)
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        class="px-4 py-2 btn-theme-primary rounded-lg font-medium"
                                        on:click=save_settings
                                    >
                                        "Save"
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="hidden"></div> }.into_any()
                    }
                }}

                // Messages area
                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                    // Empty state
                    {move || {
                        if messages.get().is_empty() {
                            view! {
                                <div class="h-full flex flex-col items-center justify-center text-center px-8">
                                    <div class="w-16 h-16 mb-4 bg-gradient-to-br from-blue-100 to-purple-100 dark:from-blue-900/30 dark:to-purple-900/30 rounded-full flex items-center justify-center">
                                        <Icon name=icons::BOT class="w-8 h-8 text-blue-600 dark:text-blue-400"/>
                                    </div>
                                    <h3 class="text-lg font-medium text-theme-primary mb-2">"How can I help you?"</h3>
                                    <p class="text-sm text-theme-tertiary max-w-md">
                                        "I can help you design your database schema. Ask me to create tables, add columns, explain relationships, or suggest improvements."
                                    </p>
                                    <div class="mt-6 flex flex-wrap gap-2 justify-center">
                                        <button
                                            class="px-3 py-1.5 text-sm bg-theme-tertiary text-theme-secondary rounded-lg hover:bg-theme-secondary transition-colors"
                                            on:click=move |_| set_input_value.set("What tables are in the schema?".to_string())
                                        >
                                            "What tables exist?"
                                        </button>
                                        <button
                                            class="px-3 py-1.5 text-sm bg-theme-tertiary text-theme-secondary rounded-lg hover:bg-theme-secondary transition-colors"
                                            on:click=move |_| set_input_value.set("Create a users table with common fields".to_string())
                                        >
                                            "Create users table"
                                        </button>
                                        <button
                                            class="px-3 py-1.5 text-sm bg-theme-tertiary text-theme-secondary rounded-lg hover:bg-theme-secondary transition-colors"
                                            on:click=move |_| set_input_value.set("Show me the schema as SQL".to_string())
                                        >
                                            "Export as SQL"
                                        </button>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}

                    // Message list
                    <For
                        each=move || messages.get().into_iter().enumerate()
                        key=|(i, _)| *i
                        children=move |(_, message)| {
                            let is_user = message.role == MessageRole::User;
                            let is_tool = message.role == MessageRole::Tool;
                            let is_assistant = message.role == MessageRole::Assistant;
                            let has_tool_calls = message.tool_calls.is_some();
                            let content = message.content.clone();

                            view! {
                                <div class=move || {
                                    if is_user {
                                        "flex justify-end"
                                    } else {
                                        "flex justify-start"
                                    }
                                }>
                                    <div class=move || {
                                        if is_user {
                                            "max-w-[85%] px-4 py-2.5 rounded-2xl bg-theme-accent text-white rounded-br-md"
                                        } else if is_tool {
                                            "max-w-[85%] px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-800 text-xs font-mono text-theme-tertiary"
                                        } else {
                                            "max-w-[85%] px-4 py-3 rounded-2xl bg-theme-tertiary text-theme-primary rounded-bl-md"
                                        }
                                    }>
                                        {if has_tool_calls {
                                            view! {
                                                <div class="text-xs text-theme-muted mb-2 flex items-center gap-1">
                                                    <Icon name=icons::LIGHTNING class="w-3 h-3"/>
                                                    "Using tools..."
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                        {if is_assistant && !content.is_empty() {
                                            view! {
                                                <Markdown content=content.clone() />
                                            }.into_any()
                                        } else {
                                            view! {
                                                <p class="whitespace-pre-wrap break-words">{content.clone()}</p>
                                            }.into_any()
                                        }}
                                    </div>
                                </div>
                            }
                        }
                    />

                    // Streaming content display
                    {move || {
                        let content = streaming_content.get();
                        if is_loading.get() && !content.is_empty() {
                            view! {
                                <div class="flex justify-start">
                                    <div class="max-w-[85%] px-4 py-3 rounded-2xl bg-theme-tertiary text-theme-primary rounded-bl-md">
                                        <Markdown content=content />
                                        <span class="inline-block w-2 h-4 bg-theme-accent animate-pulse ml-1"></span>
                                    </div>
                                </div>
                            }.into_any()
                        } else if is_loading.get() {
                            view! {
                                <div class="flex justify-start">
                                    <div class="max-w-[85%] px-4 py-3 rounded-2xl bg-theme-tertiary text-theme-primary rounded-bl-md">
                                        <div class="flex items-center gap-2">
                                            <div class="flex gap-1">
                                                <span class="w-2 h-2 bg-theme-accent rounded-full animate-bounce" style="animation-delay: 0ms"></span>
                                                <span class="w-2 h-2 bg-theme-accent rounded-full animate-bounce" style="animation-delay: 150ms"></span>
                                                <span class="w-2 h-2 bg-theme-accent rounded-full animate-bounce" style="animation-delay: 300ms"></span>
                                            </div>
                                            <span class="text-sm text-theme-tertiary">"Thinking..."</span>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}

                    // Error message
                    {move || {
                        if let Some(err) = error_message.get() {
                            view! {
                                <div class="flex justify-center">
                                    <div class="px-4 py-2 rounded-lg bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 text-sm flex items-center gap-2">
                                        <Icon name=icons::ALERT_CIRCLE class="w-4 h-4"/>
                                        <span>{err}</span>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="hidden"></div> }.into_any()
                        }
                    }}
                </div>

                // Input area
                <div class="px-4 py-3 border-t border-theme-primary bg-theme-secondary">
                    <div class="flex items-end gap-2">
                        <div class="flex-1 relative">
                            <textarea
                                class="w-full px-4 py-3 bg-theme-tertiary border border-theme-primary rounded-xl text-theme-primary placeholder-theme-muted resize-none focus:ring-2 focus:ring-theme-accent focus:border-transparent transition-all"
                                placeholder="Ask about your schema..."
                                rows="1"
                                prop:value=move || input_value.get()
                                on:input=move |e| set_input_value.set(event_target_value(&e))
                                on:keydown=move |e| {
                                    if e.key() == "Enter" && !e.shift_key() {
                                        e.prevent_default();
                                        send_message(());
                                    }
                                }
                            />
                        </div>
                        <button
                            class=move || {
                                if is_loading.get() || input_value.get().trim().is_empty() {
                                    "p-3 rounded-xl bg-theme-accent/50 text-white cursor-not-allowed"
                                } else {
                                    "p-3 rounded-xl bg-theme-accent text-white hover:opacity-90 transition-opacity"
                                }
                            }
                            on:click=move |_| send_message(())
                            disabled=move || is_loading.get() || input_value.get().trim().is_empty()
                            title="Send message"
                        >
                            <span class="flex items-center justify-center">
                                <Icon name=icons::SEND class="w-5 h-5"/>
                            </span>
                        </button>
                    </div>
                    <p class="mt-2 text-xs text-theme-muted text-center">
                        {move || {
                            let mode = config.get().mode;
                            if mode == AiMode::Write {
                                "⚠️ Write mode enabled - AI can modify your schema"
                            } else {
                                "Press Enter to send • Shift+Enter for new line"
                            }
                        }}
                    </p>
                </div>
            </div>
        </div>
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
