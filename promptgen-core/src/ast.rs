use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Template {
    pub nodes: Vec<Spanned<Node>>,
}

pub type Spanned<T> = (T, Span);

#[derive(Debug, Clone)]
pub enum Node {
    /// Plain literal text
    Text(String),

    /// `{GroupName}`
    GroupRef(String),

    /// `[[ ... ]]` expression block
    ExprBlock(Expr),

    /// `{{ SlotName }}` – freeform area
    FreeformSlot(String),

    /// `# comment to end of line`
    Comment(String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// `"literal string"` inside expressions
    Literal(String),

    /// A reference to a group by name
    GroupRef(String),

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
