use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Spanned<Node>>,
}

pub type Spanned<T> = (T, Span);

/// A query that selects options from groups by tags.
///
/// Examples:
/// - `{eyes}` -> include: ["eyes"], exclude: []
/// - `{eyes - anime}` -> include: ["eyes"], exclude: ["anime"]
/// - `{eyes - anime - crazy}` -> include: ["eyes"], exclude: ["anime", "crazy"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagQuery {
    /// Tags to include (selects groups that have ANY of these tags).
    pub include: Vec<String>,
    /// Tags to exclude (removes groups that have ANY of these tags).
    pub exclude: Vec<String>,
}

impl TagQuery {
    /// Create a new query with a single include tag.
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            include: vec![tag.into()],
            exclude: Vec::new(),
        }
    }

    /// Create a query from include and exclude tags.
    pub fn with_exclude(include: Vec<String>, exclude: Vec<String>) -> Self {
        Self { include, exclude }
    }

    /// Add an exclude tag.
    pub fn exclude(mut self, tag: impl Into<String>) -> Self {
        self.exclude.push(tag.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    /// Plain literal text
    Text(String),

    /// `{tag}` or `{tag - exclude}` – tag-based query
    TagQuery(TagQuery),

    /// `[[ ... ]]` expression block
    ExprBlock(Expr),

    /// `{{ SlotName }}` – freeform area
    FreeformSlot(String),

    /// `# comment to end of line`
    Comment(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// `"literal string"` inside expressions – interpreted as a tag query
    Literal(String),

    /// A tag query (parsed from a string like "eyes - anime")
    Query(TagQuery),

    /// Base expression + chain of operations: `"Hair" | some | assign("hair")`
    Pipeline(Box<Expr>, Vec<Op>),
}

#[derive(Debug, Clone)]
pub enum Op {
    Some,
    ExcludeGroup(String),
    Assign(String),
    // Later: If/When/etc
}
