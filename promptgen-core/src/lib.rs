pub mod ast;
pub mod eval;
pub mod library;
pub mod parser;
pub mod span;

// Re-exports for convenience
pub use ast::*;
pub use eval::{ChosenOption, EvalContext, RenderError, RenderResult, render};
pub use library::{
    EngineHint, Library, PromptGroup, PromptOption, PromptTemplate, SlotKind, TemplateSlot, new_id,
};
pub use parser::{ParseError, parse_template};
pub use span::{Span, Spanned};
