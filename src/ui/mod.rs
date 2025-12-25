pub mod activity_tracker;
pub mod ai_chat;
pub mod auth;
pub mod canvas;
pub mod column_editor;
pub mod common;
pub mod graph_ops;
pub mod icon;
pub mod liveshare_client;
pub mod liveshare_panel;
pub mod markdown;
pub mod new_table_dialog;
pub mod notifications;
pub mod pages;
pub mod remote_cursors;
pub mod settings_modal;
pub mod sidebar;
pub mod source_editor;
pub mod sync_status_indicator;
pub mod table;
pub mod table_editor;
pub mod theme;

pub use activity_tracker::ActivityTracker;
pub use ai_chat::{AiChatButton, AiChatPanel};
pub use auth::{
    AuthContext, AuthState, LoginForm, RegisterForm, User, UserMenu, provide_auth_context,
    use_auth_context,
};
pub use canvas::SchemaCanvas;
pub use column_editor::ColumnEditor;
pub use common::{
    AlertDialog, BaseModal, Button, ButtonGroup, ButtonSize, ButtonVariant, CheckboxField,
    ConfirmDialog, CreateCancelHints, ErrorMessage, ErrorMessageStatic, FormField, IconButton,
    InfoMessage, Kbd, KeyboardHint, KeyboardHintWithIcon, KeyboardHints, SaveCancelHints,
    SelectField, SubmitCancelButtons, SubmitCancelHints, SuccessMessage, SuccessMessageStatic,
    TextAreaField, WarningMessage, WarningMessageStatic,
};
pub use graph_ops::{GraphOpsSender, use_graph_ops};
pub use icon::{Icon, icons};
pub use liveshare_client::{LiveShareContext, provide_liveshare_context, use_liveshare_context};
pub use liveshare_panel::LiveSharePanel;
pub use markdown::Markdown;
pub use new_table_dialog::{CreateTableResult, NewTableData, NewTableDialog};
pub use notifications::{NotificationItem, NotificationManager, NotificationsContainer};
pub use pages::{
    DashboardPage, EditorPage, LandingPage, LoginPage, NotFoundPage, ProfilePage, RegisterPage,
};
pub use remote_cursors::{CursorTracker, RemoteCursors};
pub use settings_modal::{SettingsButton, SettingsModal};
pub use sidebar::Sidebar;
pub use source_editor::{
    EditorMode, EditorModeSwitcher, SourceEditor, check_before_save, validate_for_llm,
};
pub use sync_status_indicator::{
    ConnectionStatusBar, SnapshotSaveIndicator, SyncStatusBadge, UserPresenceIndicator,
};
pub use table_editor::TableEditor;
pub use theme::{ThemeContext, ThemeMode, provide_theme_context, use_theme_context};
