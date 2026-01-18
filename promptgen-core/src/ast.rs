use crate::span::Span;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A parsed prompt containing a sequence of nodes.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub nodes: Vec<Spanned<Node>>,
}

/// A value paired with its source location.
pub type Spanned<T> = (T, Span);

/// A reference to a library variable.
///
/// Examples:
/// - `@Hair` -> library: None, variable: "Hair"
/// - `@"Eye Color"` -> library: None, variable: "Eye Color"
/// - `@"MyLib:Hair"` -> library: Some("MyLib"), variable: "Hair"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRef {
    /// Optional library name qualifier. None means search all libraries.
    pub library: Option<String>,
    /// The variable name to reference.
    pub variable: String,
}

impl LibraryRef {
    /// Create a simple library reference (no library qualifier).
    pub fn new(variable: impl Into<String>) -> Self {
        Self {
            library: None,
            variable: variable.into(),
        }
    }

    /// Create a qualified library reference.
    pub fn qualified(library: impl Into<String>, variable: impl Into<String>) -> Self {
        Self {
            library: Some(library.into()),
            variable: variable.into(),
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
    /// `{{ label: reference("PromptName") }}` - inject another prompt's content.
    /// Slots from the referenced prompt are prefixed with this slot's label.
    Reference {
        /// The name of the prompt to reference.
        prompt_name: String,
    },
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
        self.to_definition_with_defaults(&SlotDefaults::default())
    }

    /// Normalize this pick slot with custom defaults.
    ///
    /// The defaults are used when the slot doesn't specify its own values.
    pub fn to_definition_with_defaults(
        &self,
        defaults: &SlotDefaults,
    ) -> Result<SlotDefKind, SlotNormError> {
        let sources: Vec<PickSource> = self.sources.iter().map(|(s, _)| s.clone()).collect();

        // Process operators to determine cardinality, separator, and suffix
        let mut cardinality: Option<Cardinality> = None;
        let mut sep: Option<String> = None;
        let mut suffix: Option<String> = None;

        for (op, _span) in &self.operators {
            match op {
                PickOperator::One(spec) => {
                    if cardinality.is_some() {
                        if matches!(cardinality, Some(Cardinality::One)) {
                            return Err(SlotNormError::DuplicateOne);
                        }
                        return Err(SlotNormError::ConflictingOperators);
                    }
                    cardinality = Some(Cardinality::One);
                    suffix = spec.suffix.clone();
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
                    suffix = spec.suffix.clone();
                }
            }
        }

        // Use slot-specific values, falling back to defaults
        let final_sep = sep.unwrap_or_else(|| defaults.separator().to_string());
        let final_suffix = suffix.or_else(|| defaults.suffix.clone());

        Ok(SlotDefKind::Pick {
            sources,
            cardinality: cardinality.unwrap_or_default(),
            sep: final_sep,
            suffix: final_suffix,
        })
    }
}

/// A source for a pick expression.
#[derive(Debug, Clone, PartialEq)]
pub enum PickSource {
    /// `@VariableName` or `@"Variable Name"` - reference to a library variable.
    VariableRef(LibraryRef),
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
    /// `| one` or `| one(suffix="...")` - select exactly one item.
    One(OneSpec),
    /// `| many(max=N, sep="...", suffix="...")` - select multiple items.
    Many(ManySpec),
}

/// Specification for the `one` operator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OneSpec {
    /// Suffix to append if a value is chosen (default: none).
    pub suffix: Option<String>,
}

/// Specification for the `many` operator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManySpec {
    /// Maximum number of items to select (None = unbounded).
    pub max: Option<u32>,
    /// Separator to join selected items (default: ", ").
    pub sep: Option<String>,
    /// Suffix to append if any values are chosen (default: none).
    pub suffix: Option<String>,
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
    /// The reference prefix path if this slot was expanded from a reference.
    /// None for top-level slots, Some("Prefix") for first-level nested slots,
    /// Some("A - B") for deeper nesting.
    pub reference_prefix: Option<String>,
}

/// The normalized kind of a slot.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotDefKind {
    /// Pick slot with resolved sources and cardinality.
    Pick {
        sources: Vec<PickSource>,
        cardinality: Cardinality,
        sep: String,
        /// Suffix to append if any values are chosen.
        suffix: Option<String>,
    },
    /// Textarea for freeform input.
    Textarea,
    /// Reference to another prompt - injects that prompt's content.
    /// Slots from the referenced prompt are prefixed with this slot's label.
    Reference {
        /// The name of the prompt to reference.
        prompt_name: String,
    },
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

/// Default values for slot normalization.
///
/// These defaults are applied when a slot doesn't specify its own values.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SlotDefaults {
    /// Default separator for `many` slots (default: ", ").
    pub sep: Option<String>,
    /// Default suffix appended when values are selected (default: None).
    pub suffix: Option<String>,
}

impl SlotDefaults {
    /// Create slot defaults with a separator.
    pub fn with_sep(sep: impl Into<String>) -> Self {
        Self {
            sep: Some(sep.into()),
            suffix: None,
        }
    }

    /// Create slot defaults with a suffix.
    pub fn with_suffix(suffix: impl Into<String>) -> Self {
        Self {
            sep: None,
            suffix: Some(suffix.into()),
        }
    }

    /// Set the default separator.
    pub fn sep(mut self, sep: impl Into<String>) -> Self {
        self.sep = Some(sep.into());
        self
    }

    /// Set the default suffix.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Get the separator, falling back to ", " if not set.
    pub fn separator(&self) -> &str {
        self.sep.as_deref().unwrap_or(", ")
    }

    /// Check if this is the default (empty) configuration.
    pub fn is_default(&self) -> bool {
        self.sep.is_none() && self.suffix.is_none()
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
        self.to_definition_with_defaults(&SlotDefaults::default())
    }

    /// Normalize this slot block with custom defaults.
    ///
    /// The defaults are used when a pick slot doesn't specify its own values.
    pub fn to_definition_with_defaults(
        &self,
        defaults: &SlotDefaults,
    ) -> Result<SlotDefinition, SlotNormError> {
        let label = self.label.0.clone();

        match &self.kind.0 {
            SlotKind::Textarea => Ok(SlotDefinition {
                label,
                kind: SlotDefKind::Textarea,
                reference_prefix: None,
            }),
            SlotKind::Pick(pick) => Ok(SlotDefinition {
                label,
                kind: pick.to_definition_with_defaults(defaults)?,
                reference_prefix: None,
            }),
            SlotKind::Reference { prompt_name } => Ok(SlotDefinition {
                label,
                kind: SlotDefKind::Reference {
                    prompt_name: prompt_name.clone(),
                },
                reference_prefix: None,
            }),
        }
    }
}

/// Prompt node types.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Plain literal text.
    Text(String),

    /// `{a|b|c}` – inline options, pick one randomly.
    InlineOptions(Vec<OptionItem>),

    /// `@Name` or `@"Name"` or `@"Lib:Name"` – reference to a library variable.
    LibraryRef(LibraryRef),

    /// `{{ label }}` or `{{ label: pick(...) }}` – slot block.
    SlotBlock(SlotBlock),

    /// `# comment to end of line` – ignored in output.
    Comment(String),
}
