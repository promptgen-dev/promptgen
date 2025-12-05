pub mod ast;
pub mod parser;
pub mod span;

// Re-exports for convenience
pub use ast::*;
pub use parser::{ParseError, parse_template};
pub use span::{Span, Spanned};
