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
use crate::ui::icon::{Icon, icons};
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
    /// Optional editor mode signal used when SourceEditor owns the source-mode sidebar
    #[prop(optional)]
    editor_mode: Option<RwSignal<EditorMode>>,
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

    // Line numbers derived from the visible buffer, including unsaved edits.
    let line_count = Memo::new(move |_| display_content.with(|s| s.lines().count().max(1)));

    // Handle text input changes
    let on_input = move |ev: leptos::ev::Event| {
        use leptos::wasm_bindgen::JsCast;
        let Some(target) = ev.target() else { return };
        let Some(textarea) = target.dyn_ref::<web_sys::HtmlTextAreaElement>() else {
            return;
        };
        set_local_content.set(textarea.value());
        set_is_modified.set(true);
        // Clear previous validation on edit
        set_validation_result.set(None);
        set_underline_ranges.set(Vec::new());
    };

    // Handle scroll synchronization for underline overlay
    let on_scroll = move |ev: leptos::ev::Event| {
        use leptos::wasm_bindgen::JsCast;
        let Some(target) = ev.target() else { return };
        let Some(textarea) = target.dyn_ref::<web_sys::HtmlTextAreaElement>() else {
            return;
        };
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
                if liveshare_ctx.connection_state.with_untracked(|v| *v)
                    == ConnectionState::Connected
                {
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
        <div class="source-shell theme-transition">
            <div class="source-body">
                <SourceModeSidebar
                    graph=graph
                    editor_mode=editor_mode
                />

                <main class="source-code-pane">
                    <div class="source-code-frame code-frame scroll">
                        // Line numbers with error indicators
                        <div class="source-line-gutter">
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
                                                ("●", "text-theme-error")
                                            } else if has_warning {
                                                ("●", "text-theme-warning")
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
                        <div class="source-text-host">
                            {move || {
                                if readonly {
                                    view! {
                                        <div class="relative min-h-full">
                                            <pre class="source-pre" style="line-height: 1.5rem; tab-size: 4;">
                                                {display_content.get()}
                                            </pre>
                                            <ErrorUnderlinesStatic
                                                content=display_content
                                                ranges=underline_ranges
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="relative h-full overflow-hidden">
                                            <textarea
                                                class="source-textarea"
                                                style="line-height: 1.5rem; tab-size: 4;"
                                                spellcheck="false"
                                                prop:value=move || display_content.get()
                                                on:input=on_input
                                                on:scroll=on_scroll
                                            />
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
                </main>

                <DiagnosticsRail validation_result=validation_result />
            </div>

            <div class="source-footer">
                <div class="source-footer-meta">
                    <span class="mono">{move || format!("{} lines", line_count.get())}</span>
                    {move || {
                        if is_modified.get() {
                            view! {
                                <span class="source-status source-status-warning">
                                    <span class="status-dot status-dot-pending"></span>"Modified"
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="source-status">
                                    <span class="status-dot status-dot-live"></span>"Synced"
                                </span>
                            }.into_any()
                        }
                    }}
                    {move || {
                        validation_result.get().map(|result| {
                            view! {
                                <span class=if result.is_valid { "source-status source-status-success" } else { "source-status source-status-error" }>
                                    {if result.is_valid { "Valid".to_string() } else { format!("{} errors", result.stats.error_count) }}
                                </span>
                            }
                        })
                    }}
                </div>
                <div class="source-footer-actions">
                    <button type="button" class="btn-secondary btn-sm" disabled=true title="Formatting is not implemented yet">
                        <Icon name=icons::CODE class="h-3 w-3" />"Format"
                    </button>
                    <button type="button" class="btn-secondary btn-sm" disabled=true title="Diff confirmation is planned for a later model pass">
                        "Diff"
                    </button>
                    <button
                        type="button"
                        class="btn-secondary btn-sm"
                        on:click=reset_changes
                        disabled=move || readonly || !is_modified.get()
                    >
                        <Icon name=icons::X class="h-3 w-3" />"Reset"
                    </button>
                    <button
                        type="button"
                        class="btn-primary btn-sm"
                        on:click=on_save_click
                        disabled=move || readonly || !is_modified.get() || is_saving.get()
                    >
                        {move || {
                            if is_saving.get() {
                                view! { <Icon name=icons::LOADER class="h-3 w-3 animate-spin" /> }.into_any()
                            } else {
                                view! { <Icon name=icons::CHECK class="h-3 w-3" /> }.into_any()
                            }
                        }}
                        "Apply changes"
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn SourceModeSidebar(
    graph: RwSignal<SchemaGraph>,
    editor_mode: Option<RwSignal<EditorMode>>,
) -> impl IntoView {
    view! {
        <aside class="source-sidebar">
            <div class="border-b border-theme-primary bg-theme-surface px-3 py-3 theme-transition">
                {if let Some(mode) = editor_mode {
                    view! { <EditorModeSwitcher mode=mode /> }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>

            <div class="source-sidebar-section">
                <div class="mb-2 flex items-center justify-between gap-2">
                    <div class="eyebrow">"Objects"</div>
                    <span class="badge badge-outline">"DDL outline"</span>
                </div>
                <div class="flex flex-col gap-px">
                    {move || {
                        let tables = graph.with(|g| {
                            g.node_indices()
                                .filter_map(|idx| {
                                    g.node_weight(idx).map(|node| {
                                        (node.name.clone(), node.columns.len())
                                    })
                                })
                                .collect::<Vec<_>>()
                        });

                        if tables.is_empty() {
                            view! {
                                <div class="rounded-md border border-dashed border-theme px-3 py-6 text-center text-xs text-theme-muted">
                                    "No SQL objects yet. Add tables visually or paste CREATE TABLE statements."
                                </div>
                            }.into_any()
                        } else {
                            tables.into_iter().enumerate().map(|(index, (name, columns))| {
                                view! {
                                    <button type="button" class="source-outline-row">
                                        <Icon name=icons::DATABASE class="h-3 w-3 text-theme-muted" />
                                        <span class="truncate">{name}</span>
                                        <span class="ml-auto text-[10.5px] text-theme-muted">{columns}" cols"</span>
                                        <span class="text-[10.5px] text-theme-muted">{format!("#{}", index + 1)}</span>
                                    </button>
                                }
                            }).collect_view().into_any()
                        }
                    }}
                </div>
            </div>

        </aside>
    }
}

#[component]
fn DiagnosticsRail(validation_result: ReadSignal<Option<SqlValidationResult>>) -> impl IntoView {
    view! {
        <aside class="diagnostics-rail">
            <div class="diagnostics-head">
                <div class="flex items-center gap-2">
                    <span class="font-semibold text-theme-primary">"Diagnostics"</span>
                    <span class="chip h-[18px] text-[10.5px]">
                        {move || format!("{} issues", validation_result.get().map(|r| r.diagnostics.len()).unwrap_or(0))}
                    </span>
                </div>
            </div>

            <div class="grid grid-cols-3 gap-1.5 border-b border-theme px-2 py-2">
                <div class="rounded-md border border-theme bg-theme-secondary px-2 py-1.5">
                    <div class="eyebrow">"Errors"</div>
                    <div class="mono mt-1 text-theme-error">{move || validation_result.get().map(|r| r.stats.error_count).unwrap_or(0)}</div>
                </div>
                <div class="rounded-md border border-theme bg-theme-secondary px-2 py-1.5">
                    <div class="eyebrow">"Warnings"</div>
                    <div class="mono mt-1 text-theme-warning">{move || validation_result.get().map(|r| r.stats.warning_count).unwrap_or(0)}</div>
                </div>
                <div class="rounded-md border border-theme bg-theme-secondary px-2 py-1.5">
                    <div class="eyebrow">"Hints"</div>
                    <div class="mono mt-1 text-theme-accent">{move || validation_result.get().map(|r| r.stats.hint_count).unwrap_or(0)}</div>
                </div>
            </div>

            <div class="scroll flex-1 overflow-y-auto p-1.5">
                {move || {
                    validation_result.get().map(|result| {
                        if result.diagnostics.is_empty() {
                            view! {
                                <div class="m-2 rounded-lg border border-dashed border-theme p-5 text-center text-xs text-theme-muted">
                                    <div class="font-medium text-theme-primary">"No diagnostics"</div>
                                    <div class="mt-1">"SQL is ready to apply."</div>
                                </div>
                            }.into_any()
                        } else {
                            result.diagnostics.into_iter().map(|diag| {
                                let (icon, label, class_name) = match diag.severity {
                                    ErrorSeverity::Error => (icons::ALERT_CIRCLE, "Error", "diagnostic-card diagnostic-error"),
                                    ErrorSeverity::Warning => (icons::WARNING, "Warning", "diagnostic-card diagnostic-warning"),
                                    ErrorSeverity::Hint => (icons::INFORMATION_CIRCLE, "Hint", "diagnostic-card diagnostic-hint"),
                                };
                                let position = diag.span.as_ref()
                                    .map(|span| format!("schema.sql:{}:{}", span.start.line, span.start.column))
                                    .unwrap_or_else(|| "schema.sql".to_string());
                                let message = diag.message;
                                let suggestion = diag.suggestion;
                                view! {
                                     <div class=class_name>
                                         <div class="flex items-center gap-1.5 text-xs">
                                             <Icon name=icon class="h-3 w-3" />
                                             <span class="font-medium text-theme-primary">{message}</span>
                                         </div>
                                         <div class="mt-1 mono text-theme-muted">{label}" · "{position}</div>
                                         {suggestion.map(|text| view! {
                                             <div class="mt-2 text-[11.5px] leading-relaxed text-theme-secondary">{text}</div>
                                         })}
                                         <button type="button" class="btn-secondary btn-sm mt-2" disabled=true title="Quick fixes are display-only in this pass">
                                             "Quick fix"
                                         </button>
                                     </div>
                                 }
                            }).collect_view().into_any()
                        }
                    }).unwrap_or_else(|| {
                        view! {
                            <div class="m-2 rounded-lg border border-dashed border-theme p-5 text-center text-xs text-theme-muted">
                                <div class="font-medium text-theme-primary">"Not validated"</div>
                                <div class="mt-1">"Run Apply changes to validate the current SQL."</div>
                            </div>
                        }.into_any()
                    })
                }}
            </div>
        </aside>
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
                if !(-30.0..=2000.0).contains(&top) {
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
        <div class="segmented w-full">
            // Visual mode button
            <button
                type="button"
                class=move || if mode.get() == EditorMode::Visual { "flex-1 is-active" } else { "flex-1" }
                attr:data-active=move || if mode.get() == EditorMode::Visual { "true" } else { "false" }
                aria-pressed=move || mode.get() == EditorMode::Visual
                on:click=move |_| mode.set(EditorMode::Visual)
            >
                <Icon name=icons::TABLE class="w-4 h-4" />
                "Visual"
            </button>

            // Source mode button
            <button
                type="button"
                class=move || if mode.get() == EditorMode::Source { "flex-1 is-active" } else { "flex-1" }
                attr:data-active=move || if mode.get() == EditorMode::Source { "true" } else { "false" }
                aria-pressed=move || mode.get() == EditorMode::Source
                on:click=move |_| mode.set(EditorMode::Source)
            >
                <Icon name=icons::CODE class="w-4 h-4" />
                "Code"
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
