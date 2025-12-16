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

// =============================================================================
// Slot DSL v0.1 Types
// =============================================================================

/// A slot block `{{ ... }}` - either a pick slot or textarea slot.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotBlock {
    /// The label for this slot (required).
    pub label: Spanned<String>,
    /// The kind of slot (pick or textarea).
    pub kind: Spanned<SlotKind>,
}

/// The kind of slot within a SlotBlock.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotKind {
    /// `{{ label: pick(...) [| ops] }}` - structured selection from sources.
    Pick(PickSlot),
    /// `{{ label }}` - plain textarea for freeform user input.
    Textarea,
}

/// A pick slot with sources and operators.
#[derive(Debug, Clone, PartialEq)]
pub struct PickSlot {
    /// Sources for the pick expression.
    pub sources: Vec<Spanned<PickSource>>,
    /// Operators applied to the pick (one, many).
    pub operators: Vec<Spanned<PickOperator>>,
}

impl PickSlot {
    /// Normalize this pick slot into a SlotDefKind for evaluation.
    pub fn to_definition(&self) -> Result<SlotDefKind, SlotNormError> {
        let sources: Vec<PickSource> = self.sources.iter().map(|(s, _)| s.clone()).collect();

        // Process operators to determine cardinality and separator
        let mut cardinality: Option<Cardinality> = None;
        let mut sep: Option<String> = None;

        for (op, _span) in &self.operators {
            match op {
                PickOperator::One => {
                    if cardinality.is_some() {
                        if matches!(cardinality, Some(Cardinality::One)) {
                            return Err(SlotNormError::DuplicateOne);
                        }
                        return Err(SlotNormError::ConflictingOperators);
                    }
                    cardinality = Some(Cardinality::One);
                }
                PickOperator::Many(spec) => {
                    if cardinality.is_some() {
                        if matches!(cardinality, Some(Cardinality::Many { .. })) {
                            return Err(SlotNormError::DuplicateMany);
                        }
                        return Err(SlotNormError::ConflictingOperators);
                    }
                    cardinality = Some(Cardinality::Many { max: spec.max });
                    sep = spec.sep.clone();
                }
            }
        }

        Ok(SlotDefKind::Pick {
            sources,
            cardinality: cardinality.unwrap_or_default(),
            sep: sep.unwrap_or_else(|| ", ".to_string()),
        })
    }
}

/// A source for a pick expression.
#[derive(Debug, Clone, PartialEq)]
pub enum PickSource {
    /// `@GroupName` or `@"Group Name"` - reference to a library group.
    GroupRef(LibraryRef),
    /// A literal string option.
    Literal {
        /// The literal value.
        value: String,
        /// Whether the literal was quoted in the source.
        quoted: bool,
    },
}

/// Operators that can be applied to a pick expression.
#[derive(Debug, Clone, PartialEq)]
pub enum PickOperator {
    /// `| one` - select exactly one item.
    One,
    /// `| many(max=N, sep="...")` - select multiple items.
    Many(ManySpec),
}

/// Specification for the `many` operator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManySpec {
    /// Maximum number of items to select (None = unbounded).
    pub max: Option<u32>,
    /// Separator to join selected items (default: ", ").
    pub sep: Option<String>,
}

impl ManySpec {
    /// Get the separator, defaulting to ", ".
    pub fn separator(&self) -> &str {
        self.sep.as_deref().unwrap_or(", ")
    }
}

// =============================================================================
// Derived SlotDefinition (for eval/UI)
// =============================================================================

/// A normalized slot definition for use in evaluation and UI.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotDefinition {
    /// The label for this slot.
    pub label: String,
    /// The kind of slot.
    pub kind: SlotDefKind,
}

/// The normalized kind of a slot.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotDefKind {
    /// Pick slot with resolved sources and cardinality.
    Pick {
        sources: Vec<PickSource>,
        cardinality: Cardinality,
        sep: String,
    },
    /// Textarea for freeform input.
    Textarea,
}

/// Selection cardinality for pick slots.
#[derive(Debug, Clone, PartialEq)]
pub enum Cardinality {
    /// Select exactly one item.
    One,
    /// Select multiple items.
    Many { max: Option<u32> },
}

impl Default for Cardinality {
    fn default() -> Self {
        Cardinality::Many { max: None }
    }
}

/// Error when normalizing a SlotBlock to SlotDefinition.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SlotNormError {
    #[error("conflicting operators: both 'one' and 'many' specified")]
    ConflictingOperators,
    #[error("duplicate 'one' operator")]
    DuplicateOne,
    #[error("duplicate 'many' operator")]
    DuplicateMany,
}

impl SlotBlock {
    /// Normalize this slot block into a SlotDefinition for evaluation.
    pub fn to_definition(&self) -> Result<SlotDefinition, SlotNormError> {
        let label = self.label.0.clone();

        match &self.kind.0 {
            SlotKind::Textarea => Ok(SlotDefinition {
                label,
                kind: SlotDefKind::Textarea,
            }),
            SlotKind::Pick(pick) => {
                let sources: Vec<PickSource> =
                    pick.sources.iter().map(|(s, _)| s.clone()).collect();

                // Process operators to determine cardinality and separator
                let mut cardinality: Option<Cardinality> = None;
                let mut sep: Option<String> = None;

                for (op, _span) in &pick.operators {
                    match op {
                        PickOperator::One => {
                            if cardinality.is_some() {
                                if matches!(cardinality, Some(Cardinality::One)) {
                                    return Err(SlotNormError::DuplicateOne);
                                }
                                return Err(SlotNormError::ConflictingOperators);
                            }
                            cardinality = Some(Cardinality::One);
                        }
                        PickOperator::Many(spec) => {
                            if cardinality.is_some() {
                                if matches!(cardinality, Some(Cardinality::Many { .. })) {
                                    return Err(SlotNormError::DuplicateMany);
                                }
                                return Err(SlotNormError::ConflictingOperators);
                            }
                            cardinality = Some(Cardinality::Many { max: spec.max });
                            sep = spec.sep.clone();
                        }
                    }
                }

                Ok(SlotDefinition {
                    label,
                    kind: SlotDefKind::Pick {
                        sources,
                        cardinality: cardinality.unwrap_or_default(),
                        sep: sep.unwrap_or_else(|| ", ".to_string()),
                    },
                })
            }
        }
    }
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

    /// `{{ label }}` or `{{ label: pick(...) }}` – slot block.
    SlotBlock(SlotBlock),

    /// `# comment to end of line` – ignored in output.
    Comment(String),
}
