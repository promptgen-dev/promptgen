pub mod ast;
pub mod eval;
#[cfg(feature = "serde")]
pub mod io;
pub mod library;
pub mod parser;
pub mod search;
pub mod span;

// Re-exports for convenience
pub use ast::{
    Cardinality, LibraryRef, Node, OptionItem, PickSource, Prompt, SlotDefKind, SlotDefinition,
    Spanned,
};

// Eval module exports
pub use eval::{ChosenOption, EvalContext, RenderError, RenderResult, render};

#[cfg(feature = "serde")]
pub use io::{
    IoError, load_library, load_pack, parse_library, parse_pack, prompt_to_source, save_library,
    save_pack, serialize_library, serialize_pack,
};

// Library module exports
pub use library::{
    // Diagnostic types
    DiagnosticError,
    DiagnosticWarning,
    ErrorKind,
    // Core types
    Library,
    ParseResult,
    PromptVariable,
    ReferenceInfo,
    SavedPrompt,
    SlotValue,
    VariableInfo,
    WarningKind,
};

// Search module exports
pub use search::{OptionMatch, OptionSearchResult, SearchResult, VariableSearchResult};

pub use parser::{ParseError, parse_prompt};
pub use span::Span;
