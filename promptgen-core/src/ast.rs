use crate::span::Span;

/// A parsed template containing a sequence of nodes.
#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Spanned<Node>>,
}

/// A value paired with its source location.
pub type Spanned<T> = (T, Span);

/// A reference to a library group.
///
/// Examples:
/// - `@Hair` -> library: None, group: "Hair"
/// - `@"Eye Color"` -> library: None, group: "Eye Color"
/// - `@"MyLib:Hair"` -> library: Some("MyLib"), group: "Hair"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRef {
    /// Optional library name qualifier. None means search all libraries.
    pub library: Option<String>,
    /// The group name to reference.
    pub group: String,
}

impl LibraryRef {
    /// Create a simple library reference (no library qualifier).
    pub fn new(group: impl Into<String>) -> Self {
        Self {
            library: None,
            group: group.into(),
        }
    }

    /// Create a qualified library reference.
    pub fn qualified(library: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            library: Some(library.into()),
            group: group.into(),
        }
    }
}

/// An item within inline options `{a|b|c}`.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionItem {
    /// Plain text option.
    Text(String),
    /// Option containing nested grammar (e.g., `{@Hair|bald}` where `@Hair` is nested).
    Nested(Vec<Spanned<Node>>),
}

/// Template node types.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Plain literal text.
    Text(String),

    /// `{a|b|c}` – inline options, pick one randomly.
    InlineOptions(Vec<OptionItem>),

    /// `@Name` or `@"Name"` or `@"Lib:Name"` – reference to a library group.
    LibraryRef(LibraryRef),

    /// `{{ name }}` – user-provided slot value.
    Slot(String),

    /// `# comment to end of line` – ignored in output.
    Comment(String),
}
