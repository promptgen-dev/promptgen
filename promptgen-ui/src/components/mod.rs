mod autocomplete;
mod editor;
mod focusable_frame;
mod variable_editor;
mod preview;
mod sidebar;
mod slots;
pub mod template_editor;

pub use autocomplete::{AutocompletePopup, CompletionItem, check_autocomplete_trigger, find_autocomplete_context, get_completions, handle_autocomplete_keyboard};
pub use editor::EditorPanel;
pub use focusable_frame::{FocusableFrame, FocusableFrameResponse};
pub use variable_editor::VariableEditorPanel;
pub use preview::PreviewPanel;
pub use sidebar::SidebarPanel;
pub use slots::SlotPanel;
pub use template_editor::{TemplateEditor, TemplateEditorConfig, TemplateEditorResponse};
