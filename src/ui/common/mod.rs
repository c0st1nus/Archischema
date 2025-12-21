//! Common reusable UI components
//!
//! This module provides commonly used UI components that are shared across
//! multiple parts of the application to reduce code duplication and improve
//! consistency.

pub mod badge;
pub mod button;
pub mod dropdown;
pub mod form;
pub mod keyboard;
pub mod message;
pub mod modal;
pub mod spinner;
pub mod tabs;
pub mod tooltip;

pub use badge::{
    Badge, BadgeGroup, BadgeSize, BadgeVariant, CountBadge, DotBadge, KeyboardBadge,
    RemovableBadge, StatusBadge,
};
pub use button::{Button, ButtonGroup, ButtonSize, ButtonVariant, IconButton, SubmitCancelButtons};
pub use dropdown::{
    Dropdown, DropdownAlign, DropdownItem, DropdownItemVariant, IconDropdown, SimpleDropdown,
};
pub use form::{CheckboxField, FormField, SelectField, TextAreaField};
pub use keyboard::{
    CreateCancelHints, Kbd, KeyboardHint, KeyboardHintWithIcon, KeyboardHints, SaveCancelHints,
    SubmitCancelHints,
};
pub use message::{
    ErrorMessage, ErrorMessageStatic, InfoMessage, SuccessMessage, SuccessMessageStatic,
    WarningMessage, WarningMessageStatic,
};
pub use modal::{AlertDialog, BaseModal, ConfirmDialog};
pub use spinner::{
    InlineSpinner, LoadingButton, LoadingOverlay, LoadingSpinner, LoadingWrapper, Skeleton,
    SkeletonGroup, Spinner, SpinnerSize, SpinnerStyle,
};
pub use tabs::{TabItem, TabPanel, Tabs, TabsWithPanels};
pub use tooltip::{
    InfoTooltip, SimpleTooltip, TextTooltip, Tooltip, TooltipIcon, TooltipPosition, TooltipTrigger,
};
