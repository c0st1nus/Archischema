use crate::core::Column;
use crate::ui::{ColumnEditor, Dialog, ErrorMessage, Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StarterPreset {
    Empty,
    Identity,
    Audit,
    SoftDelete,
}

impl StarterPreset {
    fn label(self) -> &'static str {
        match self {
            StarterPreset::Empty => "Empty",
            StarterPreset::Identity => "Identity",
            StarterPreset::Audit => "Audit",
            StarterPreset::SoftDelete => "Soft delete",
        }
    }

    fn description(self) -> &'static str {
        match self {
            StarterPreset::Empty => "No columns",
            StarterPreset::Identity => "id + created_at",
            StarterPreset::Audit => "+ updated_at, by",
            StarterPreset::SoftDelete => "+ deleted_at",
        }
    }
}

fn preset_columns(preset: StarterPreset) -> Vec<Column> {
    let id = Column::new("id", "uuid")
        .primary_key()
        .with_default("gen_random_uuid()");
    let created_at = Column::new("created_at", "timestamptz")
        .not_null()
        .with_default("now()");

    match preset {
        StarterPreset::Empty => Vec::new(),
        StarterPreset::Identity => vec![id, created_at],
        StarterPreset::Audit => vec![
            id,
            created_at,
            Column::new("updated_at", "timestamptz").with_default("now()"),
            Column::new("updated_by", "uuid"),
        ],
        StarterPreset::SoftDelete => vec![
            id,
            created_at,
            Column::new("updated_at", "timestamptz").with_default("now()"),
            Column::new("deleted_at", "timestamptz"),
        ],
    }
}

/// Data used to create a new table. Only `table_name` and `columns` are applied
/// to the graph in this model-limited redesign pass.
#[derive(Clone, Debug)]
pub struct NewTableData {
    pub table_name: String,
    pub pk_name: String,
    pub pk_type: String,
    pub columns: Vec<Column>,
}

impl Default for NewTableData {
    fn default() -> Self {
        Self {
            table_name: String::new(),
            pk_name: "id".to_string(),
            pk_type: "uuid".to_string(),
            columns: preset_columns(StarterPreset::Identity),
        }
    }
}

/// Result of creating a table.
#[derive(Clone, Debug)]
pub enum CreateTableResult {
    Success,
    Error(String),
}

#[component]
pub fn NewTableDialog(
    /// Whether dialog is open
    is_open: Signal<bool>,
    /// Callback for creating a table. Returns success/error for inline feedback.
    #[prop(into)]
    on_create: Callback<NewTableData, CreateTableResult>,
    /// Cancel callback
    #[prop(into)]
    on_cancel: Callback<()>,
    /// Optional initial table name
    #[prop(default = String::new())]
    initial_table_name: String,
    /// Optional table existence check
    #[prop(optional, into)]
    table_exists: Option<Callback<String, bool>>,
) -> impl IntoView {
    let (table_name, set_table_name) = signal(initial_table_name);
    let (schema_name, set_schema_name) = signal("public".to_string());
    let (folder_name, set_folder_name) = signal("Current diagram".to_string());
    let (description, set_description) = signal(String::new());
    let (preset, set_preset) = signal(StarterPreset::Identity);
    let (columns, set_columns) = signal(preset_columns(StarterPreset::Identity));
    let (editing_column_index, set_editing_column_index) = signal::<Option<usize>>(None);
    let (is_column_editor_open, set_is_column_editor_open) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_creating, set_is_creating) = signal(false);

    let table_input_ref = NodeRef::<leptos::html::Input>::new();

    Effect::new(move |_| {
        if is_open.get()
            && let Some(input) = table_input_ref.get()
        {
            let _ = input.focus();
            input.select();
        }
    });

    let validate_identifier = move |label: &str, value: &str| -> Result<(), String> {
        if value.is_empty() {
            return Err(format!("{} cannot be empty", label));
        }

        if !value
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        {
            return Err(format!("{} must start with a letter or underscore", label));
        }

        if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "{} can only contain letters, numbers, and underscores",
                label
            ));
        }

        Ok(())
    };

    let handle_create = move || {
        let name = table_name.get().trim().to_string();

        if let Err(err) = validate_identifier("Table name", &name) {
            set_error.set(Some(err));
            return;
        }

        if let Some(ref exists_fn) = table_exists
            && exists_fn.run(name.clone())
        {
            set_error.set(Some(format!("Table '{}' already exists", name)));
            return;
        }

        set_is_creating.set(true);
        set_error.set(None);

        let draft_columns = columns.get();
        let first_pk = draft_columns.iter().find(|column| column.is_primary_key);
        let result = on_create.run(NewTableData {
            table_name: name.clone(),
            pk_name: first_pk
                .map(|column| column.name.clone())
                .unwrap_or_else(|| "id".to_string()),
            pk_type: first_pk
                .map(|column| column.data_type.clone())
                .unwrap_or_else(|| "uuid".to_string()),
            columns: draft_columns,
        });

        match result {
            CreateTableResult::Success => {
                set_is_creating.set(false);
                set_table_name.set(String::new());
                set_schema_name.set("public".to_string());
                set_folder_name.set("Current diagram".to_string());
                set_description.set(String::new());
                set_preset.set(StarterPreset::Identity);
                set_columns.set(preset_columns(StarterPreset::Identity));
                set_is_column_editor_open.set(false);
                set_editing_column_index.set(None);
            }
            CreateTableResult::Error(err) => {
                set_is_creating.set(false);
                set_error.set(Some(err));
            }
        }
    };

    let handle_cancel = move || {
        set_error.set(None);
        set_is_creating.set(false);
        set_is_column_editor_open.set(false);
        set_editing_column_index.set(None);
        on_cancel.run(());
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && (ev.ctrl_key() || ev.meta_key()) {
            ev.prevent_default();
            handle_create();
        } else if ev.key() == "Escape" {
            ev.prevent_default();
            handle_cancel();
        }
    };

    let preset_options = [
        StarterPreset::Empty,
        StarterPreset::Identity,
        StarterPreset::Audit,
        StarterPreset::SoftDelete,
    ];

    view! {
        <Dialog
            is_open=is_open
            on_close=Callback::new(move |_| handle_cancel())
            max_width="max-w-5xl"
            close_on_backdrop=true
        >
            <div class="dialog-head form-dialog-head">
                <div>
                    <div class="eyebrow mb-1">"New table"</div>
                    <h3 class="title-lg">"Create table"</h3>
                    <p class="mt-1 text-sm text-theme-muted">
                        "Tables are scoped to the current diagram. Columns can be added now or later."
                    </p>
                </div>
                <button type="button" class="btn-secondary btn-sm" disabled=true title="Use the AI panel from the editor for generation">
                    <Icon name=icons::SPARKLES class="h-3.5 w-3.5" />
                    "Generate with AI"
                </button>
            </div>

            <div class="form-wall-body" on:keydown=handle_keydown>
                <div class="grid gap-4 lg:grid-cols-[1fr_0.72fr_0.72fr]">
                    <label class="block">
                        <span class="field-label">"Name"</span>
                        <input
                            node_ref=table_input_ref
                            type="text"
                            autocomplete="off"
                            spellcheck="false"
                            class="input-base mt-1"
                            placeholder="categories"
                            prop:value=move || table_name.get()
                            on:input=move |ev| {
                                set_table_name.set(event_target_value(&ev));
                                set_error.set(None);
                            }
                            disabled=move || is_creating.get()
                        />
                        <span class="field-help">"snake_case · plural is conventional in Postgres"</span>
                    </label>

                    <label class="block">
                        <span class="field-label">"Schema"</span>
                        <input
                            type="text"
                            autocomplete="off"
                            spellcheck="false"
                            class="input-base mt-1"
                            prop:value=move || schema_name.get()
                            on:input=move |ev| set_schema_name.set(event_target_value(&ev))
                        />
                        <span class="field-help">"Database namespace · preview-only"</span>
                    </label>

                    <label class="block">
                        <span class="field-label">"Folder"</span>
                        <input
                            type="text"
                            autocomplete="off"
                            class="input-base mt-1"
                            prop:value=move || folder_name.get()
                            on:input=move |ev| set_folder_name.set(event_target_value(&ev))
                        />
                        <span class="field-help">"Folder metadata is not persisted yet"</span>
                    </label>
                </div>

                <label class="block">
                    <span class="field-label">"Description"</span>
                    <textarea
                        class="input-base mt-1 min-h-[92px] resize-none"
                        placeholder="Lookup table for post categories."
                        prop:value=move || description.get()
                        on:input=move |ev| set_description.set(event_target_value(&ev))
                    ></textarea>
                    <span class="field-help">"Surfaces in tooltips and exported docs later. Preview-only in this pass."</span>
                </label>

                <ErrorMessage error=error />

                <section>
                    <div class="field-label mb-2">"Starter preset"</div>
                    <div class="preset-grid">
                        {preset_options.into_iter().map(|option| {
                            let option_for_click = option;
                            let option_for_active = option;
                            view! {
                                <button
                                    type="button"
                                    class="preset-card"
                                    attr:data-active=move || if preset.get() == option_for_active { "true" } else { "false" }
                                    on:click=move |_| {
                                        set_preset.set(option_for_click);
                                        set_columns.set(preset_columns(option_for_click));
                                        set_editing_column_index.set(None);
                                        set_is_column_editor_open.set(false);
                                    }
                                >
                                    <span class="font-semibold text-theme-primary">{option.label()}</span>
                                    <span class="text-xs text-theme-muted">{option.description()}</span>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </section>

                <section class="surface overflow-hidden">
                    <div class="form-section-head">
                        <span class="eyebrow">"Columns · " {move || columns.with(|cols| cols.len())}</span>
                        <button
                            type="button"
                            class="btn-ghost btn-sm"
                            on:click=move |_| {
                                set_editing_column_index.set(None);
                                set_is_column_editor_open.set(true);
                            }
                            disabled=move || is_creating.get()
                        >
                            <Icon name=icons::PLUS class="h-3 w-3" />
                            "Add column"
                        </button>
                    </div>
                    <div class="form-column-table-head">
                        <span></span>
                        <span>"Name"</span>
                        <span>"Type"</span>
                        <span>"Not null"</span>
                        <span>"Unique"</span>
                        <span>"Default"</span>
                        <span></span>
                    </div>
                    <div class="max-h-[260px] overflow-y-auto scroll">
                        {move || {
                            let draft_columns = columns.get();
                            if draft_columns.is_empty() {
                                view! {
                                    <div class="rounded-md border border-dashed border-theme p-5 text-center text-xs text-theme-muted">
                                        <div>"No columns yet."</div>
                                        <button
                                            type="button"
                                            class="btn-link mx-auto mt-2"
                                            on:click=move |_| {
                                                set_editing_column_index.set(None);
                                                set_is_column_editor_open.set(true);
                                            }
                                        >
                                            "+ Add first column"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                draft_columns.into_iter().enumerate().map(|(column_idx, column)| {
                                    let name = column.name.clone();
                                    let data_type = column.data_type.clone();
                                    let default = column.default_value.clone().unwrap_or_else(|| "-".to_string());
                                    view! {
                                        <div class="form-column-row">
                                            <span class="form-column-drag"><Icon name=icons::GRIP_HORIZONTAL class="h-3.5 w-3.5" /></span>
                                            <div class="column-inline-main">
                                                <span class="column-inline-badges">
                                                    {if column.is_primary_key {
                                                        view! { <span class="schema-table-badge schema-table-badge-pk">"PK"</span> }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }}
                                                </span>
                                                <span class="column-inline-name" title=name.clone()>{name.clone()}</span>
                                            </div>
                                            <span class="column-inline-type" title=data_type.clone()>{data_type.clone()}</span>
                                            <span class="form-column-check">{if !column.is_nullable { "✓" } else { "-" }}</span>
                                            <span class="form-column-check">{if column.is_unique { "✓" } else { "-" }}</span>
                                            <span class="form-column-default" title=default.clone()>{default.clone()}</span>
                                            <span class="form-column-actions">
                                                <button
                                                    type="button"
                                                    class="btn-icon btn-sm"
                                                    title="Edit column"
                                                    on:click=move |_| {
                                                        set_editing_column_index.set(Some(column_idx));
                                                        set_is_column_editor_open.set(true);
                                                    }
                                                >
                                                    <Icon name=icons::EDIT class="h-3.5 w-3.5" />
                                                </button>
                                                <button
                                                    type="button"
                                                    class="btn-icon btn-sm"
                                                    title="Delete column"
                                                    on:click=move |_| {
                                                        set_columns.update(|draft| {
                                                            if column_idx < draft.len() {
                                                                draft.remove(column_idx);
                                                            }
                                                        });
                                                        if editing_column_index.get() == Some(column_idx) {
                                                            set_editing_column_index.set(None);
                                                            set_is_column_editor_open.set(false);
                                                        }
                                                    }
                                                >
                                                    <Icon name=icons::TRASH class="h-3.5 w-3.5" />
                                                </button>
                                            </span>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        }}
                    </div>
                </section>

                {move || {
                    if is_column_editor_open.get() {
                        let edit_idx = editing_column_index.get();
                        let draft_column = columns.with(|draft| {
                            edit_idx.and_then(|idx| draft.get(idx).cloned())
                        });

                        view! {
                            <section class="create-table-column-editor">
                                <ColumnEditor
                                    column=draft_column
                                    column_index=edit_idx
                                    inline=true
                                    on_column_save=Callback::new(move |column: Column| {
                                        set_columns.update(|draft| {
                                            if let Some(idx) = edit_idx {
                                                if idx < draft.len() {
                                                    draft[idx] = column;
                                                } else {
                                                    draft.push(column);
                                                }
                                            } else {
                                                draft.push(column);
                                            }
                                        });
                                    })
                                    column_name_exists=Callback::new(move |candidate: String| {
                                        columns.with(|draft| {
                                            draft.iter().enumerate().any(|(idx, column)| {
                                                Some(idx) != edit_idx && column.name == candidate
                                            })
                                        })
                                    })
                                    on_save=move || {
                                        set_is_column_editor_open.set(false);
                                        set_editing_column_index.set(None);
                                    }
                                    on_cancel=move || {
                                        set_is_column_editor_open.set(false);
                                        set_editing_column_index.set(None);
                                    }
                                    on_delete=move || {
                                        if let Some(idx) = edit_idx {
                                            set_columns.update(|draft| {
                                                if idx < draft.len() {
                                                    draft.remove(idx);
                                                }
                                            });
                                        }
                                        set_is_column_editor_open.set(false);
                                        set_editing_column_index.set(None);
                                    }
                                />
                            </section>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>

            <div class="form-dialog-footer">
                <div class="text-xs text-theme-muted">
                    <span class="font-medium text-theme-secondary">"Will emit 1 CREATE TABLE"</span>
                    " · " {move || columns.with(|draft| draft.len())} " columns"
                </div>
                <div class="flex items-center justify-end gap-2">
                    <button class="btn-secondary" on:click=move |_| handle_cancel() disabled=move || is_creating.get()>
                        "Cancel"
                    </button>
                    <button class="btn-secondary" type="button" disabled=true title="SQL diff preview is planned for a later model pass">
                        <Icon name=icons::CODE class="icon-btn" />
                        "Preview SQL"
                    </button>
                    <button
                        class="btn-primary"
                        on:click=move |_| handle_create()
                        disabled=move || is_creating.get() || table_name.get().trim().is_empty()
                    >
                        {move || if is_creating.get() {
                            view! { <Icon name=icons::LOADER class="icon-btn animate-spin" /> }.into_any()
                        } else {
                            view! { <Icon name=icons::PLUS class="icon-btn" /> }.into_any()
                        }}
                        "Create table"
                    </button>
                </div>
            </div>
        </Dialog>
    }
}
