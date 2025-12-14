pub mod ast;
pub mod eval;
#[cfg(feature = "serde")]
pub mod io; // TODO: Commented out internally, needs update for new grammar
pub mod library;
pub mod parser;
pub mod span;
pub mod workspace;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-exports for convenience
pub use ast::{LibraryRef, Node, OptionItem, Spanned, Template};

// Eval module exports
pub use eval::{ChosenOption, EvalContext, RenderError, RenderResult, render};

#[cfg(feature = "serde")]
pub use io::{
    IoError, load_library, load_pack, parse_pack, save_library, save_pack, serialize_pack,
};

pub use library::{
    EngineHint, Library, PromptGroup, PromptTemplate, SlotKind, TemplateSlot, new_id,
};
pub use parser::{ParseError, parse_template};
pub use span::Span;

// Workspace exports
pub use workspace::{
    CompletionItem, CompletionKind, DiagnosticError, DiagnosticWarning, ErrorKind, GroupInfo,
    ParseResult, ReferenceInfo, WarningKind, Workspace, WorkspaceBuilder,
};
