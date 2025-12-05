pub mod ast;
pub mod eval;
#[cfg(feature = "serde")]
pub mod io;
pub mod library;
pub mod parser;
pub mod span;

// Re-exports for convenience
pub use ast::*;
pub use eval::{ChosenOption, EvalContext, RenderError, RenderResult, render};
#[cfg(feature = "serde")]
pub use io::{
    IoError, load_library, load_pack, parse_pack, save_library, save_pack, serialize_pack,
};
pub use library::{
    EngineHint, Library, PromptGroup, PromptOption, PromptTemplate, SlotKind, TemplateSlot, new_id,
};
pub use parser::{ParseError, parse_template};
pub use span::{Span, Spanned};
