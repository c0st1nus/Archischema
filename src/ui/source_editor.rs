//! SQL Source Editor component
//!
//! Provides a text editor for viewing and editing the database schema as SQL DDL statements.
//! This is an alternative view to the visual canvas editor.
//! Includes syntax validation with error underlines and semantic validation.
//! Save button validates SQL and applies changes to graph with LiveShare sync.

use crate::core::{
    CanvasNotification, ErrorSeverity, ExportOptions, SchemaExporter, SchemaGraph, SqlDialect,
    SqlValidationResult, UnderlineRange, apply_sql_to_graph, validate_sql,
};
use crate::ui::liveshare_client::{ConnectionState, use_liveshare_context};
use leptos::prelude::*;
use leptos::web_sys;

/// View mode for the schema editor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Visual canvas editor (default)
    #[default]
    Visual,
    /// SQL source code editor
    Source,
}

/// SQL Source Editor component
#[component]
pub fn SourceEditor(
    /// The schema graph to display/edit
    graph: RwSignal<SchemaGraph>,
    /// Whether the editor is in read-only mode
    #[prop(default = false)]
    readonly: bool,
    /// Optional callback for notifications
    #[prop(optional)]
    on_notification: Option<Callback<CanvasNotification>>,
    /// Optional callback for validation result (for LLM agent)
    #[prop(optional)]
    on_validation: Option<Callback<SqlValidationResult>>,
) -> impl IntoView {
    // Get LiveShare context for sync
    let liveshare_ctx = use_liveshare_context();

    // SQL content derived from graph
    let sql_content = Memo::new(move |_| {
        graph.with(|g| {
            let options = ExportOptions {
                sql_dialect: SqlDialect::MySQL,
                include_positions: true,
                include_drop_statements: false,
                pretty_print: true,
                ..Default::default()
            };
            SchemaExporter::export_sql(g, &options).unwrap_or_else(|e| format!("-- Error: {}", e))
        })
    });

    // Line numbers derived from content
    let line_count = Memo::new(move |_| sql_content.with(|s| s.lines().count().max(1)));

    // Local editable content (for non-readonly mode)
    let (local_content, set_local_content) = signal(String::new());
    let (is_modified, set_is_modified) = signal(false);
    let (validation_result, set_validation_result) = signal::<Option<SqlValidationResult>>(None);
    let (underline_ranges, set_underline_ranges) = signal::<Vec<UnderlineRange>>(Vec::new());
    let (is_saving, set_is_saving) = signal(false);
    let (scroll_top, set_scroll_top) = signal(0.0f64);
    let (scroll_left, set_scroll_left) = signal(0.0f64);

    // Sync local content when graph changes (only if not modified)
    Effect::new(move |_| {
        if !is_modified.get() {
            set_local_content.set(sql_content.get());
            // Clear validation when content syncs from graph
            set_validation_result.set(None);
            set_underline_ranges.set(Vec::new());
        }
    });

    // Display content - either local (if modified) or from graph
    let display_content = Memo::new(move |_| {
        if is_modified.get() {
            local_content.get()
        } else {
            sql_content.get()
        }
    });

    // Handle text input changes
    let on_input = move |ev: leptos::ev::Event| {
        use leptos::wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let textarea = target.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        let value = textarea.value();
        set_local_content.set(value);
        set_is_modified.set(true);
        // Clear previous validation on edit
        set_validation_result.set(None);
        set_underline_ranges.set(Vec::new());
    };

    // Handle scroll synchronization for underline overlay
    let on_scroll = move |ev: leptos::ev::Event| {
        use leptos::wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let textarea = target.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        set_scroll_top.set(textarea.scroll_top() as f64);
        set_scroll_left.set(textarea.scroll_left() as f64);
    };

    // Reset to graph state
    let reset_changes = move |_: leptos::ev::MouseEvent| {
        set_local_content.set(sql_content.get());
        set_is_modified.set(false);
        set_validation_result.set(None);
        set_underline_ranges.set(Vec::new());
    };

    // Save button click - validate and apply
    let on_save_click = move |_: leptos::ev::MouseEvent| {
        set_is_saving.set(true);

        let content = local_content.get();

        // First validate
        let validation = validate_sql(&content, SqlDialect::MySQL);

        // Update underline ranges with source for better context
        let ranges = validation.get_underline_ranges_with_source(Some(&content));
        set_underline_ranges.set(ranges);

        // Notify via validation callback if provided
        if let Some(cb) = on_validation {
            cb.run(validation.clone());
        }

        if !validation.is_valid {
            // Validation failed - show errors
            let error_count = validation.stats.error_count;
            set_validation_result.set(Some(validation));

            if let Some(cb) = on_notification {
                let notification = CanvasNotification::error(
                    "Validation Failed",
                    format!("{} errors found. Fix them before saving.", error_count),
                );
                cb.run(notification);
            }

            set_is_saving.set(false);
            return;
        }

        // Validation passed - apply changes to graph
        graph.update(|g| {
            let result = apply_sql_to_graph(&content, SqlDialect::MySQL, g);

            if result.success {
                // Send graph operations through LiveShare
                if liveshare_ctx.connection_state.get_untracked() == ConnectionState::Connected {
                    for op in &result.graph_ops {
                        liveshare_ctx.send_graph_op(op.clone());
                    }
                }

                // Clear modified flag since changes are now applied
                set_is_modified.set(false);
                set_validation_result.set(None);
                set_underline_ranges.set(Vec::new());

                if let Some(cb) = on_notification {
                    let notification = CanvasNotification::from_apply_result(&result);
                    cb.run(notification);
                }
            } else {
                // Application failed
                if let Some(cb) = on_notification {
                    let error_msg = result
                        .errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Unknown error".to_string());
                    let notification =
                        CanvasNotification::error("Failed to Apply Changes", error_msg);
                    cb.run(notification);
                }
            }
        });

        set_is_saving.set(false);
    };

    view! {
        <div class="h-full flex flex-col bg-theme-primary theme-transition">
            // Toolbar
            <div class="flex items-center justify-between px-4 py-2 bg-theme-secondary border-b border-theme-primary theme-transition">
                <div class="flex items-center gap-2">
                    <span class="text-theme-secondary text-sm font-medium">"SQL Source"</span>
                    <span class="text-theme-muted text-xs">
                        {move || format!("{} lines", line_count.get())}
                    </span>
                    {move || {
                        if is_modified.get() {
                            view! {
                                <span class="px-2 py-0.5 text-xs font-medium rounded-full bg-yellow-500/20 text-yellow-400">
                                    "Modified"
                                </span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                    // Validation status badge
                    {move || {
                        if let Some(result) = validation_result.get() {
                            if result.is_valid {
                                view! {
                                    <span class="px-2 py-0.5 text-xs font-medium rounded-full bg-green-500/20 text-green-400">
                                        "✓ Valid"
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <span class="px-2 py-0.5 text-xs font-medium rounded-full bg-red-500/20 text-red-400">
                                        {format!("{} errors", result.stats.error_count)}
                                    </span>
                                }.into_any()
                            }
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                // Action buttons
                <div class="flex items-center gap-2">
                    // Save button (only show if modified and not readonly)
                    {move || {
                        if !readonly && is_modified.get() {
                            view! {
                                <button
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-green-500/20 text-green-400 hover:bg-green-500/30 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
                                    on:click=on_save_click
                                    disabled=move || is_saving.get()
                                >
                                    {move || {
                                        if is_saving.get() {
                                            view! {
                                                <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                </svg>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
                                                </svg>
                                            }.into_any()
                                        }
                                    }}
                                    "Save"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}

                    // Reset button (only show if modified)
                    {move || {
                        if !readonly && is_modified.get() {
                            view! {
                                <button
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-theme-tertiary text-theme-secondary hover:bg-theme-primary transition-colors"
                                    on:click=reset_changes
                                >
                                    "Reset"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>
            </div>

            // Diagnostics panel (show when there are errors/warnings)
            {move || {
                validation_result.get().and_then(|result| {
                    if result.diagnostics.is_empty() {
                        None
                    } else {
                        Some(view! {
                            <div class="max-h-32 overflow-auto border-b border-theme-primary bg-theme-tertiary">
                                {result.diagnostics.iter().map(|diag| {
                                    let icon = match diag.severity {
                                        ErrorSeverity::Error => "❌",
                                        ErrorSeverity::Warning => "⚠️",
                                        ErrorSeverity::Hint => "💡",
                                    };
                                    let bg_class = match diag.severity {
                                        ErrorSeverity::Error => "bg-red-500/10 border-l-red-500",
                                        ErrorSeverity::Warning => "bg-yellow-500/10 border-l-yellow-500",
                                        ErrorSeverity::Hint => "bg-blue-500/10 border-l-blue-500",
                                    };
                                    let position = diag.span.as_ref().map(|s| format!("[L{}:{}] ", s.start.line, s.start.column)).unwrap_or_default();
                                    let message = diag.message.clone();
                                    let suggestion = diag.suggestion.clone();

                                    view! {
                                        <div class={format!("px-3 py-1.5 text-xs border-l-2 {} flex flex-col gap-0.5", bg_class)}>
                                            <div class="flex items-center gap-1.5">
                                                <span>{icon}</span>
                                                <span class="text-theme-muted font-mono">{position}</span>
                                                <span class="text-theme-primary">{message}</span>
                                            </div>
                                            {suggestion.map(|s| view! {
                                                <div class="text-theme-muted pl-5">
                                                    "→ " {s}
                                                </div>
                                            })}
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        })
                    }
                })
            }}

            // Editor area
            <div class="flex-1 flex overflow-hidden">
                // Line numbers with error indicators
                <div class="flex-shrink-0 w-14 bg-theme-secondary border-r border-theme-primary overflow-hidden theme-transition">
                    <div class="py-3 px-1 text-right font-mono text-xs select-none" style="line-height: 1.5rem;">
                        {move || {
                            let ranges = underline_ranges.get();
                            let error_lines: std::collections::HashSet<usize> = ranges.iter()
                                .filter(|r| r.severity == ErrorSeverity::Error)
                                .map(|r| r.start_line)
                                .collect();
                            let warning_lines: std::collections::HashSet<usize> = ranges.iter()
                                .filter(|r| r.severity == ErrorSeverity::Warning)
                                .map(|r| r.start_line)
                                .collect();

                            (1..=line_count.get())
                                .map(|n| {
                                    let has_error = error_lines.contains(&n);
                                    let has_warning = !has_error && warning_lines.contains(&n);

                                    let (indicator, text_class) = if has_error {
                                        ("●", "text-red-400")
                                    } else if has_warning {
                                        ("●", "text-yellow-400")
                                    } else {
                                        ("", "text-theme-muted")
                                    };

                                    view! {
                                        <div class="flex items-center justify-end gap-1">
                                            <span class={format!("text-[10px] {}", text_class)}>{indicator}</span>
                                            <span class={format!("w-6 {}", text_class)}>{n}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </div>

                // Text area with underlines
                <div class="flex-1 overflow-auto bg-theme-primary relative">
                    {move || {
                        if readonly {
                            // Read-only: use pre element with underlines
                            view! {
                                <div class="relative">
                                    <pre class="p-3 font-mono text-sm text-theme-primary whitespace-pre overflow-x-auto" style="line-height: 1.5rem; tab-size: 4;">
                                        {display_content.get()}
                                    </pre>
                                    <ErrorUnderlinesStatic
                                        content=display_content
                                        ranges=underline_ranges
                                    />
                                </div>
                            }.into_any()
                        } else {
                            // Editable: use textarea with overlay for underlines
                            view! {
                                <div class="relative h-full overflow-hidden">
                                    <textarea
                                        class="w-full h-full p-3 font-mono text-sm text-theme-primary bg-transparent resize-none outline-none absolute inset-0 z-10"
                                        style="line-height: 1.5rem; tab-size: 4;"
                                        spellcheck="false"
                                        prop:value=move || display_content.get()
                                        on:input=on_input
                                        on:scroll=on_scroll
                                    />
                                    // Error underline overlay with tooltips (above textarea for hover)
                                    <ErrorUnderlinesWithScroll
                                        content=display_content
                                        ranges=underline_ranges
                                        scroll_top=scroll_top
                                        scroll_left=scroll_left
                                    />
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Footer with help text and stats
            <div class="px-4 py-2 bg-theme-secondary border-t border-theme-primary flex items-center justify-between text-theme-muted text-xs theme-transition">
                <span>
                    {move || {
                        if readonly {
                            "Read-only view. Switch to Visual mode to edit.".to_string()
                        } else if is_modified.get() {
                            "Modified. Click 'Save' to validate and apply changes.".to_string()
                        } else {
                            "SQL DDL representation of your schema.".to_string()
                        }
                    }}
                </span>
                {move || {
                    validation_result.get().map(|result| {
                        view! {
                            <span class="text-theme-muted">
                                {format!("{} tables • {} relationships • {} errors • {} warnings",
                                    result.stats.table_count,
                                    result.stats.relationship_count,
                                    result.stats.error_count,
                                    result.stats.warning_count
                                )}
                            </span>
                        }
                    })
                }}
            </div>
        </div>
    }
}

/// Component to render error underlines with tooltips (for readonly mode)
#[component]
fn ErrorUnderlinesStatic(
    content: Memo<String>,
    ranges: ReadSignal<Vec<UnderlineRange>>,
) -> impl IntoView {
    // Create underline markers based on ranges
    let underline_markers = move || {
        let text = content.get();
        let error_ranges = ranges.get();

        if error_ranges.is_empty() {
            return Vec::new();
        }

        // Calculate pixel positions for each error
        // This is approximate - real implementation would need more precise measurement
        let line_height = 24.0; // 1.5rem = 24px
        let char_width = 8.4; // Approximate monospace char width
        let padding_top = 12.0; // p-3 = 12px
        let padding_left = 12.0;

        error_ranges
            .iter()
            .map(|range| {
                let top = padding_top + (range.start_line - 1) as f64 * line_height;
                let left = padding_left + (range.start_column - 1) as f64 * char_width;

                // Calculate width based on error span
                let width = if range.start_line == range.end_line {
                    ((range.end_column - range.start_column).max(1)) as f64 * char_width
                } else {
                    // For multi-line, just underline to end of first line
                    let line_start = text.lines().nth(range.start_line - 1).unwrap_or("");
                    (line_start.len() - range.start_column + 1).max(1) as f64 * char_width
                };

                // Get hex color for SVG (without # for URL encoding)
                // Using brighter colors for better visibility
                let (color_hex, color_rgb) = match range.severity {
                    ErrorSeverity::Error => ("ff3333", "rgb(255 51 51)"), // bright red
                    ErrorSeverity::Warning => ("ffcc00", "rgb(255 204 0)"), // bright yellow
                    ErrorSeverity::Hint => ("4d9fff", "rgb(77 159 255)"), // bright blue
                };

                (
                    top,
                    left,
                    width,
                    color_hex.to_string(),
                    color_rgb.to_string(),
                    range.message.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="absolute inset-0 overflow-hidden z-20" style="pointer-events: none;">
            {move || {
                underline_markers()
                    .into_iter()
                    .enumerate()
                    .map(|(i, (top, left, width, color_hex, _color_rgb, message))| {
                        // SVG wavy line pattern - creates a proper squiggly underline like in IDEs
                        // Using thicker stroke and larger wave for better visibility
                        let svg_wave = format!(
                            "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 8 4'%3E%3Cpath d='M0 4 Q 2 0, 4 4 T 8 4' fill='none' stroke='%23{}' stroke-width='2'/%3E%3C/svg%3E\")",
                            color_hex
                        );
                        let underline_style = format!(
                            "top: {}px; left: {}px; width: {}px; height: 6px; background-image: {}; background-repeat: repeat-x; background-position: bottom; background-size: 8px 4px; pointer-events: auto; cursor: help;",
                            top + 15.0, // Position at bottom of line
                            left,
                            width.max(8.0), // Minimum width to show at least one wave
                            svg_wave
                        );
                        let border_color = format!("border-color: #{};", color_hex);
                        view! {
                            <div
                                data-key=i
                                class="absolute group"
                                style=underline_style
                            >
                                // Tooltip on hover - positioned higher above the line
                                <div
                                    class="absolute left-0 bottom-full mb-4 opacity-0 group-hover:opacity-100 transition-opacity duration-150 z-50 min-w-max"
                                    style="pointer-events: none; user-select: none;"
                                >
                                    <div
                                        class="px-3 py-2 text-xs rounded-lg shadow-lg border-l-4 bg-gray-900/95 text-gray-100 whitespace-nowrap"
                                        style=border_color
                                    >
                                        <div class="font-medium">{message}</div>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

/// Error underlines component with scroll synchronization for editable textarea
#[component]
fn ErrorUnderlinesWithScroll(
    content: Memo<String>,
    ranges: ReadSignal<Vec<UnderlineRange>>,
    scroll_top: ReadSignal<f64>,
    scroll_left: ReadSignal<f64>,
) -> impl IntoView {
    // Create underline markers based on ranges
    let underline_markers = move || {
        let text = content.get();
        let error_ranges = ranges.get();
        let st = scroll_top.get();
        let sl = scroll_left.get();

        if error_ranges.is_empty() {
            return Vec::new();
        }

        // Calculate pixel positions for each error
        let line_height = 24.0; // 1.5rem = 24px
        let char_width = 8.4; // Approximate monospace char width
        let padding_top = 12.0; // p-3 = 12px
        let padding_left = 12.0;

        error_ranges
            .iter()
            .filter_map(|range| {
                let top = padding_top + (range.start_line - 1) as f64 * line_height - st;
                let left = padding_left + (range.start_column - 1) as f64 * char_width - sl;

                // Skip if outside visible area
                if top < -30.0 || top > 2000.0 {
                    return None;
                }

                // Calculate width based on error span
                let width = if range.start_line == range.end_line {
                    ((range.end_column - range.start_column).max(1)) as f64 * char_width
                } else {
                    // For multi-line, just underline to end of first line
                    let line_start = text.lines().nth(range.start_line - 1).unwrap_or("");
                    (line_start.len() - range.start_column + 1).max(1) as f64 * char_width
                };

                // Get hex color for SVG (without # for URL encoding)
                // Using brighter colors for better visibility
                let color_hex = match range.severity {
                    ErrorSeverity::Error => "ff3333",   // bright red
                    ErrorSeverity::Warning => "ffcc00", // bright yellow
                    ErrorSeverity::Hint => "4d9fff",    // bright blue
                };

                Some((
                    top,
                    left,
                    width,
                    color_hex.to_string(),
                    range.message.clone(),
                ))
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="absolute inset-0 overflow-hidden z-20" style="pointer-events: none;">
            {move || {
                underline_markers()
                    .into_iter()
                    .enumerate()
                    .map(|(i, (top, left, width, color_hex, message))| {
                        // SVG wavy line pattern - creates a proper squiggly underline like in IDEs
                        // Using thicker stroke and larger wave for better visibility
                        let svg_wave = format!(
                            "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 8 4'%3E%3Cpath d='M0 4 Q 2 0, 4 4 T 8 4' fill='none' stroke='%23{}' stroke-width='2'/%3E%3C/svg%3E\")",
                            color_hex
                        );
                        let underline_style = format!(
                            "top: {}px; left: {}px; width: {}px; height: 6px; background-image: {}; background-repeat: repeat-x; background-position: bottom; background-size: 8px 4px; pointer-events: auto; cursor: help;",
                            top + 15.0, // Position at bottom of line
                            left,
                            width.max(8.0), // Minimum width to show at least one wave
                            svg_wave
                        );
                        let border_color = format!("border-color: #{};", color_hex);
                        view! {
                            <div
                                data-key=i
                                class="absolute group"
                                style=underline_style
                            >
                                // Tooltip on hover - positioned higher above the line
                                <div
                                    class="absolute left-0 bottom-full mb-4 opacity-0 group-hover:opacity-100 transition-opacity duration-150 z-50 min-w-max"
                                    style="pointer-events: none; user-select: none;"
                                >
                                    <div
                                        class="px-3 py-2 text-xs rounded-lg shadow-lg border-l-4 bg-gray-900/95 text-gray-100 whitespace-nowrap"
                                        style=border_color
                                    >
                                        <div class="font-medium">{message}</div>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

/// Mode switcher component for sidebar
#[component]
pub fn EditorModeSwitcher(
    /// Current editor mode
    mode: RwSignal<EditorMode>,
) -> impl IntoView {
    view! {
        <div class="flex items-center bg-theme-tertiary rounded-lg p-1 theme-transition w-full">
            // Visual mode button
            <button
                class="flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-all"
                style=move || {
                    if mode.get() == EditorMode::Visual {
                        "background-color: var(--bg-surface); color: var(--text-primary); box-shadow: var(--shadow-sm);"
                    } else {
                        "background-color: transparent; color: var(--text-tertiary);"
                    }
                }
                on:click=move |_| mode.set(EditorMode::Visual)
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
                </svg>
                "Visual"
            </button>

            // Source mode button
            <button
                class="flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-all"
                style=move || {
                    if mode.get() == EditorMode::Source {
                        "background-color: var(--bg-surface); color: var(--text-primary); box-shadow: var(--shadow-sm);"
                    } else {
                        "background-color: transparent; color: var(--text-tertiary);"
                    }
                }
                on:click=move |_| mode.set(EditorMode::Source)
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
                </svg>
                "Source"
            </button>
        </div>
    }
}

// ============================================================================
// Helper functions for external use
// ============================================================================

/// Validate SQL and return result for LLM agent
/// This is the main entry point for LLM agents to validate SQL changes
pub fn validate_for_llm(sql: &str, dialect: SqlDialect) -> serde_json::Value {
    let result = validate_sql(sql, dialect);
    result.format_for_llm()
}

/// Check schema and return validation result
/// Use this when saving or before applying changes
pub fn check_before_save(graph: &SchemaGraph) -> SqlValidationResult {
    crate::core::check_schema_sql(graph, SqlDialect::MySQL)
}
