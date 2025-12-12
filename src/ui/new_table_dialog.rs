use crate::ui::{Icon, icons};
use leptos::prelude::*;
use leptos::web_sys;

/// Данные для создания новой таблицы
#[derive(Clone, Debug)]
pub struct NewTableData {
    pub table_name: String,
    pub pk_name: String,
    pub pk_type: String,
}

impl Default for NewTableData {
    fn default() -> Self {
        Self {
            table_name: String::new(),
            pk_name: "id".to_string(),
            pk_type: "INT".to_string(),
        }
    }
}

/// Результат создания таблицы
#[derive(Clone, Debug)]
pub enum CreateTableResult {
    /// Успешно создана
    Success,
    /// Ошибка создания
    Error(String),
}

/// Диалог создания новой таблицы с настройкой первичного ключа
#[component]
pub fn NewTableDialog(
    /// Callback при создании таблицы (передаёт данные таблицы), возвращает результат
    #[prop(into)]
    on_create: Callback<NewTableData, CreateTableResult>,
    /// Callback при отмене
    #[prop(into)]
    on_cancel: Callback<()>,
    /// Начальное имя таблицы (опционально)
    #[prop(default = String::new())]
    initial_table_name: String,
    /// Функция проверки существования таблицы (опционально)
    #[prop(optional, into)]
    table_exists: Option<Callback<String, bool>>,
) -> impl IntoView {
    let (table_name, set_table_name) = signal(initial_table_name);
    let (pk_name, set_pk_name) = signal("id".to_string());
    let (pk_type, set_pk_type) = signal("INT".to_string());
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_creating, set_is_creating) = signal(false);

    let table_input_ref = NodeRef::<leptos::html::Input>::new();

    // Auto-focus на input имени таблицы при монтировании
    Effect::new(move || {
        if let Some(input) = table_input_ref.get() {
            let _ = input.focus();
            input.select();
        }
    });

    // Типы данных для PK (только те, что подходят для первичного ключа)
    let pk_types = [
        "INT",
        "BIGINT",
        "TINYINT",
        "SMALLINT",
        "MEDIUMINT",
        "VARCHAR",
        "CHAR",
    ];

    let handle_create = move || {
        let name = table_name.get().trim().to_string();
        let pk = pk_name.get().trim().to_string();
        let pk_t = pk_type.get();

        // Валидация имени таблицы
        if name.is_empty() {
            set_error.set(Some("Table name cannot be empty".to_string()));
            return;
        }

        // Валидация имени PK
        if pk.is_empty() {
            set_error.set(Some("Primary key name cannot be empty".to_string()));
            return;
        }

        // Проверка на валидные символы в имени таблицы
        if !name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        {
            set_error.set(Some(
                "Table name must start with a letter or underscore".to_string(),
            ));
            return;
        }

        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            set_error.set(Some(
                "Table name can only contain letters, numbers, and underscores".to_string(),
            ));
            return;
        }

        // Проверка на валидные символы в имени PK
        if !pk
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        {
            set_error.set(Some(
                "Primary key name must start with a letter or underscore".to_string(),
            ));
            return;
        }

        if !pk.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            set_error.set(Some(
                "Primary key name can only contain letters, numbers, and underscores".to_string(),
            ));
            return;
        }

        // Проверка на существование таблицы с таким именем
        if let Some(ref exists_fn) = table_exists
            && exists_fn.run(name.clone())
        {
            set_error.set(Some(format!("Table '{}' already exists", name)));
            return;
        }

        set_is_creating.set(true);
        set_error.set(None);

        let result = on_create.run(NewTableData {
            table_name: name.clone(),
            pk_name: pk,
            pk_type: pk_t,
        });

        match result {
            CreateTableResult::Success => {
                // Успех - диалог закроется через on_create callback
            }
            CreateTableResult::Error(err) => {
                set_is_creating.set(false);
                set_error.set(Some(err));
            }
        }
    };

    let handle_cancel = move || {
        set_error.set(None);
        on_cancel.run(());
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "Enter" => {
            ev.prevent_default();
            handle_create();
        }
        "Escape" => {
            ev.prevent_default();
            handle_cancel();
        }
        _ => {}
    };

    view! {
        <div class="h-full flex flex-col space-y-4">
            // Заголовок
            <div>
                <h3 class="text-lg font-semibold text-theme-primary">"New Table"</h3>
                <p class="text-sm text-theme-muted">
                    "Create a new table with a primary key"
                </p>
            </div>

            // Форма
            <div class="space-y-4">
                // Поле имени таблицы
                <div>
                    <label class="block text-sm font-medium text-theme-secondary mb-2">
                        "Table Name"
                        <span class="text-red-500">"*"</span>
                    </label>
                    <input
                        node_ref=table_input_ref
                        type="text"
                        class="w-full px-4 py-2.5 input-theme rounded-lg transition-all"
                        placeholder="e.g., users, orders, products"
                        prop:value=move || table_name.get()
                        on:input=move |ev| {
                            set_table_name.set(event_target_value(&ev));
                            set_error.set(None);
                        }
                        on:keydown=handle_keydown
                        disabled=move || is_creating.get()
                    />
                </div>

                // Секция первичного ключа
                <div class="p-4 bg-theme-secondary rounded-lg space-y-3 theme-transition">
                    <div class="flex items-center text-sm font-medium text-theme-primary">
                        <Icon name=icons::KEY class="w-4 h-4 mr-2 text-yellow-500"/>
                        "Primary Key"
                    </div>

                    // Имя первичного ключа
                    <div>
                        <label class="block text-xs font-medium text-theme-tertiary mb-1">
                            "Column Name"
                            <span class="text-red-500">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 input-theme rounded-md text-sm"
                            placeholder="e.g., id, user_id"
                            prop:value=move || pk_name.get()
                            on:input=move |ev| {
                                set_pk_name.set(event_target_value(&ev));
                                set_error.set(None);
                            }
                            on:keydown=handle_keydown
                            disabled=move || is_creating.get()
                        />
                    </div>

                    // Тип данных первичного ключа
                    <div>
                        <label class="block text-xs font-medium text-theme-tertiary mb-1">
                            "Data Type"
                        </label>
                        <select
                            class="w-full px-3 py-2 input-theme rounded-md text-sm"
                            prop:value=move || pk_type.get()
                            on:change=move |ev| {
                                set_pk_type.set(event_target_value(&ev));
                            }
                            disabled=move || is_creating.get()
                        >
                            {pk_types
                                .iter()
                                .map(|&dt| {
                                    view! {
                                        <option value=dt selected=move || pk_type.get() == dt>
                                            {dt}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>

                    // Информация о PK
                    <div class="text-xs text-theme-muted flex items-start">
                        <Icon name=icons::KEY class="w-3 h-3 mr-1 mt-0.5 flex-shrink-0 text-yellow-500"/>
                        <span>"Primary key will be NOT NULL and auto-indexed"</span>
                    </div>
                </div>

                // Ошибка
                {move || {
                    error
                        .get()
                        .map(|err| {
                            view! {
                                <div class="flex items-center text-sm text-theme-error">
                                    <Icon name=icons::ALERT_CIRCLE class="w-4 h-4 mr-1.5"/>
                                    <span>{err}</span>
                                </div>
                            }
                        })
                }}
            </div>

            // Кнопки действий - центрированы
            <div class="flex items-center justify-center space-x-3 pt-4 border-t border-theme-primary">
                <button
                    class="px-5 py-2.5 text-sm font-medium text-theme-secondary hover:bg-theme-tertiary rounded-lg transition-colors"
                    on:click=move |_| handle_cancel()
                    disabled=move || is_creating.get()
                >
                    "Cancel"
                </button>
                <button
                    class="px-6 py-2.5 text-sm font-medium btn-theme-primary rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
                    on:click=move |_| handle_create()
                    disabled=move || {
                        is_creating.get()
                            || table_name.get().trim().is_empty()
                            || pk_name.get().trim().is_empty()
                    }
                >
                    {move || {
                        if is_creating.get() {
                            view! {
                                <>
                                    <Icon name=icons::LOADER class="w-4 h-4 mr-1.5 animate-spin"/>
                                    "Creating..."
                                </>
                            }
                                .into_any()
                        } else {
                            view! {
                                <>
                                    <Icon name=icons::PLUS class="w-4 h-4 mr-1.5"/>
                                    "Create Table"
                                </>
                            }
                                .into_any()
                        }
                    }}
                </button>
            </div>

            // Подсказка по горячим клавишам - внизу с margin
            <div class="mt-auto pt-4 pb-2">
                <div class="flex items-center justify-center space-x-4 text-xs text-theme-muted">
                    <div class="flex items-center">
                        <kbd class="px-2 py-1 bg-theme-tertiary rounded border border-theme-secondary font-mono text-theme-secondary">
                            "Enter"
                        </kbd>
                        <span class="ml-1">"to create"</span>
                    </div>
                    <div class="flex items-center">
                        <kbd class="px-2 py-1 bg-theme-tertiary rounded border border-theme-secondary font-mono text-theme-secondary">
                            "Esc"
                        </kbd>
                        <span class="ml-1">"to cancel"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}
