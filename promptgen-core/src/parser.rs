use std::collections::HashMap;

use chumsky::prelude::*;
use chumsky::{error::Simple, extra, span::SimpleSpan};

use crate::ast::{
    LibraryRef, ManySpec, Node, OptionItem, PickOperator, PickSlot, PickSource, SlotBlock,
    SlotKind, Template,
};
use crate::span::Span;

/// Information about a duplicate slot label.
#[derive(Debug, Clone)]
pub struct DuplicateLabelInfo {
    /// The duplicate label name.
    pub label: String,
    /// Span of the first occurrence.
    pub first_span: Span,
    /// Span of the duplicate occurrence.
    pub duplicate_span: Span,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError<'a> {
    #[error("parse error(s): {0:?}")]
    Chumsky(Vec<Simple<'a, char>>),

    #[error("duplicate slot label '{label}' at position {duplicate_span:?}; first defined at {first_span:?}")]
    DuplicateLabel {
        label: String,
        first_span: Span,
        duplicate_span: Span,
    },
}

/// Helper to convert Chumsky spans to our custom Span
fn to_range(span: SimpleSpan<usize>) -> Span {
    span.start..span.end
}

/// Parse a library reference string (the part after @ or inside quotes).
///
/// Examples:
/// - `"Hair"` -> LibraryRef { library: None, variable: "Hair" }
/// - `"Eye Color"` -> LibraryRef { library: None, variable: "Eye Color" }
/// - `"MyLib:Hair"` -> LibraryRef { library: Some("MyLib"), variable: "Hair" }
fn parse_library_ref_string(s: &str) -> LibraryRef {
    if let Some(colon_pos) = s.find(':') {
        let library = s[..colon_pos].to_string();
        let variable = s[colon_pos + 1..].to_string();
        LibraryRef::qualified(library, variable)
    } else {
        LibraryRef::new(s)
    }
}

pub fn parse_template(src: &str) -> Result<Template, ParseError<'_>> {
    let result = template_parser().parse(src);

    match result.into_result() {
        Ok(tmpl) => {
            // Validate for duplicate labels
            if let Some(dup) = find_duplicate_labels(&tmpl) {
                return Err(ParseError::DuplicateLabel {
                    label: dup.label,
                    first_span: dup.first_span,
                    duplicate_span: dup.duplicate_span,
                });
            }
            Ok(tmpl)
        }
        Err(errs) => Err(ParseError::Chumsky(errs)),
    }
}

/// Find the first duplicate slot label in a template.
/// Returns information about the duplicate if found.
fn find_duplicate_labels(template: &Template) -> Option<DuplicateLabelInfo> {
    let mut seen: HashMap<&str, Span> = HashMap::new();

    for (node, _span) in &template.nodes {
        if let Node::SlotBlock(slot_block) = node {
            let label = &slot_block.label.0;
            let label_span = slot_block.label.1.clone();

            if let Some(first_span) = seen.get(label.as_str()) {
                return Some(DuplicateLabelInfo {
                    label: label.clone(),
                    first_span: first_span.clone(),
                    duplicate_span: label_span,
                });
            }
            seen.insert(label, label_span);
        }
    }

    None
}

fn template_parser<'src>() -> impl Parser<'src, &'src str, Template, extra::Err<Simple<'src, char>>>
{
    node_parser()
        .repeated()
        .collect::<Vec<_>>()
        .map(|nodes| Template { nodes })
}

/// Parser for a single node. Used both at the top level and for nested parsing in options.
fn node_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    // Order matters for precedence:
    // 1. {{ slot }} - must come before { to avoid confusion
    // 2. { inline options } - inline options with | separator
    // 3. @"quoted" - quoted library ref
    // 4. @identifier - simple library ref
    // 5. # comment - line comment
    // 6. text - everything else

    let slot_node = slot_block_parser();
    let inline_options_node = inline_options_parser();
    let quoted_lib_ref_node = quoted_library_ref_parser();
    let simple_lib_ref_node = simple_library_ref_parser();
    let comment_node = comment_parser();
    let text_node = text_parser();

    choice((
        slot_node,
        inline_options_node,
        quoted_lib_ref_node,
        simple_lib_ref_node,
        comment_node,
        text_node,
    ))
}

// =============================================================================
// Slot Block Parser (v0.1 DSL)
// =============================================================================

/// Parse `{{ ... }}` - slot block (textarea or pick)
///
/// Precedence:
/// 1. `{{ label: pick(...) [| ops] }}` - pick slot
/// 2. `{{ label }}` - textarea slot
fn slot_block_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("{{")
        .ignore_then(slot_block_content_parser().padded())
        .then_ignore(just("}}"))
        .map_with(|slot_block, e| (Node::SlotBlock(slot_block), to_range(e.span())))
}

/// Parse the content inside {{ ... }}
fn slot_block_content_parser<'src>(
) -> impl Parser<'src, &'src str, SlotBlock, extra::Err<Simple<'src, char>>> + Clone {
    // Try pick slot first (has colon), then textarea
    pick_slot_parser().or(textarea_slot_parser())
}

/// Parse `label: pick(...) [| ops]`
fn pick_slot_parser<'src>(
) -> impl Parser<'src, &'src str, SlotBlock, extra::Err<Simple<'src, char>>> + Clone {
    slot_label_parser()
        .then_ignore(just(':').padded())
        .then(pick_expression_parser())
        .map(|((label, label_span), (pick_slot, kind_span))| SlotBlock {
            label: (label, label_span),
            kind: (SlotKind::Pick(pick_slot), kind_span),
        })
}

/// Parse just a label (textarea slot)
fn textarea_slot_parser<'src>(
) -> impl Parser<'src, &'src str, SlotBlock, extra::Err<Simple<'src, char>>> + Clone {
    slot_label_parser().map_with(|(label, label_span), e| {
        let span = to_range(e.span());
        SlotBlock {
            label: (label, label_span),
            kind: (SlotKind::Textarea, span),
        }
    })
}

/// Parse a slot label (quoted or bare)
fn slot_label_parser<'src>(
) -> impl Parser<'src, &'src str, (String, Span), extra::Err<Simple<'src, char>>> + Clone {
    // Quoted label: "label text"
    let quoted_label = just('"')
        .ignore_then(
            any()
                .filter(|c: &char| *c != '"')
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just('"'))
        .map_with(|s, e| (s, to_range(e.span())));

    // Bare label: anything up to ':' or '}}'
    // We need to be careful not to consume the ':' for pick slots
    let bare_label = none_of(":}")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| s.trim().to_string())
        .map_with(|s, e| (s, to_range(e.span())));

    quoted_label.or(bare_label)
}

/// Parse `pick(...) [| ops]`
fn pick_expression_parser<'src>(
) -> impl Parser<'src, &'src str, (PickSlot, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("pick")
        .ignore_then(just('(').padded())
        .ignore_then(pick_sources_parser())
        .then_ignore(just(')').padded())
        .then(pick_operators_parser())
        .map_with(|(sources, operators), e| {
            (PickSlot { sources, operators }, to_range(e.span()))
        })
}

/// Parse comma-separated pick sources
fn pick_sources_parser<'src>(
) -> impl Parser<'src, &'src str, Vec<(PickSource, Span)>, extra::Err<Simple<'src, char>>> + Clone {
    pick_source_parser()
        .separated_by(just(',').padded())
        .at_least(1)
        .collect::<Vec<_>>()
}

/// Parse a single pick source: @VariableRef or literal
fn pick_source_parser<'src>(
) -> impl Parser<'src, &'src str, (PickSource, Span), extra::Err<Simple<'src, char>>> + Clone {
    // Variable reference: @Name or @"Name"
    let variable_ref = pick_variable_ref_parser();

    // Quoted literal: "text"
    let quoted_literal = just('"')
        .ignore_then(quoted_string_content_parser())
        .then_ignore(just('"'))
        .map_with(|s, e| {
            (
                PickSource::Literal {
                    value: s,
                    quoted: true,
                },
                to_range(e.span()),
            )
        });

    // Bare literal: text until , or )
    let bare_literal = none_of(",)\"@")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| s.trim().to_string())
        .map_with(|s, e| {
            (
                PickSource::Literal {
                    value: s,
                    quoted: false,
                },
                to_range(e.span()),
            )
        });

    choice((variable_ref, quoted_literal, bare_literal)).padded()
}

/// Parse quoted string content with escape sequences
fn quoted_string_content_parser<'src>(
) -> impl Parser<'src, &'src str, String, extra::Err<Simple<'src, char>>> + Clone {
    let escape = just('\\').ignore_then(choice((
        just('"').to('"'),
        just('\\').to('\\'),
        just('n').to('\n'),
        just('t').to('\t'),
    )));

    let normal_char = none_of("\"\\");

    choice((escape, normal_char))
        .repeated()
        .collect::<String>()
}

/// Parse @VariableRef inside pick()
fn pick_variable_ref_parser<'src>(
) -> impl Parser<'src, &'src str, (PickSource, Span), extra::Err<Simple<'src, char>>> + Clone {
    // @"quoted name" or @identifier
    let quoted_ref = just("@\"")
        .ignore_then(none_of("\"").repeated().collect::<String>())
        .then_ignore(just('"'))
        .map(|name| PickSource::VariableRef(parse_library_ref_string(&name)));

    let simple_ref = just('@')
        .ignore_then(
            any()
                .filter(|c: &char| c.is_alphabetic() || *c == '_')
                .then(
                    any()
                        .filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '-')
                        .repeated()
                        .collect::<String>(),
                )
                .map(|(first, rest)| format!("{}{}", first, rest)),
        )
        .map(|name| PickSource::VariableRef(LibraryRef::new(name)));

    quoted_ref
        .or(simple_ref)
        .map_with(|source, e| (source, to_range(e.span())))
}

/// Parse pipe-separated operators: `| one` or `| many(...)`
fn pick_operators_parser<'src>(
) -> impl Parser<'src, &'src str, Vec<(PickOperator, Span)>, extra::Err<Simple<'src, char>>> + Clone
{
    pick_operator_parser()
        .repeated()
        .collect::<Vec<_>>()
}

/// Parse a single operator: `| one` or `| many(...)`
fn pick_operator_parser<'src>(
) -> impl Parser<'src, &'src str, (PickOperator, Span), extra::Err<Simple<'src, char>>> + Clone {
    just('|')
        .padded()
        .ignore_then(choice((one_operator_parser(), many_operator_parser())))
}

/// Parse `one`
fn one_operator_parser<'src>(
) -> impl Parser<'src, &'src str, (PickOperator, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("one").map_with(|_, e| (PickOperator::One, to_range(e.span())))
}

/// Parse `many` or `many(max=N, sep="...")`
fn many_operator_parser<'src>(
) -> impl Parser<'src, &'src str, (PickOperator, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("many")
        .ignore_then(many_args_parser().or_not())
        .map_with(|args, e| {
            let spec = args.unwrap_or_default();
            (PickOperator::Many(spec), to_range(e.span()))
        })
}

/// Parse `(max=N, sep="...")`
fn many_args_parser<'src>(
) -> impl Parser<'src, &'src str, ManySpec, extra::Err<Simple<'src, char>>> + Clone {
    just('(')
        .padded()
        .ignore_then(many_arg_parser().separated_by(just(',').padded()).collect::<Vec<_>>())
        .then_ignore(just(')').padded())
        .map(|args| {
            let mut spec = ManySpec::default();
            for (key, value) in args {
                match key.as_str() {
                    "max" => {
                        if let Ok(n) = value.parse::<u32>() {
                            spec.max = Some(n);
                        }
                    }
                    "sep" => {
                        spec.sep = Some(value);
                    }
                    _ => {} // Ignore unknown args for now
                }
            }
            spec
        })
}

/// Parse a single many arg: `key=value`
fn many_arg_parser<'src>(
) -> impl Parser<'src, &'src str, (String, String), extra::Err<Simple<'src, char>>> + Clone {
    // key
    any()
        .filter(|c: &char| c.is_alphabetic() || *c == '_')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then_ignore(just('=').padded())
        .then(many_arg_value_parser())
}

/// Parse a many arg value: number or quoted string
fn many_arg_value_parser<'src>(
) -> impl Parser<'src, &'src str, String, extra::Err<Simple<'src, char>>> + Clone {
    // Quoted string
    let quoted = just('"')
        .ignore_then(quoted_string_content_parser())
        .then_ignore(just('"'));

    // Number
    let number = any()
        .filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .at_least(1)
        .collect::<String>();

    // Identifier (for None, etc.)
    let ident = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .collect::<String>();

    choice((quoted, number, ident))
}

/// Split a string by a delimiter, but only at depth 0 (outside nested braces).
/// For example, splitting "a|{b|c}|d" by '|' yields ["a", "{b|c}", "d"].
fn split_at_depth_zero(s: &str, delimiter: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: usize = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            c if c == delimiter && depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    // Don't forget the last segment
    result.push(&s[start..]);
    result
}

/// Parse `{a|b|c}` - inline options
/// Options can contain nested grammar (like @Hair or nested {x|y})
fn inline_options_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just('{')
        .ignore_then(brace_balanced_content())
        .then_ignore(just('}'))
        .map_with(|content, e| {
            // Split by | at depth 0 only (respecting nested braces)
            let options: Vec<OptionItem> = split_at_depth_zero(&content, '|')
                .into_iter()
                .map(|opt| {
                    let opt = opt.trim();
                    OptionItem::Text(opt.to_string())
                })
                .collect();

            (Node::InlineOptions(options), to_range(e.span()))
        })
}

/// Parse content inside braces, respecting nested braces.
/// Returns the content string (without outer braces).
/// Uses Chumsky's recursive combinator to handle arbitrary nesting.
fn brace_balanced_content<'src>(
) -> impl Parser<'src, &'src str, String, extra::Err<Simple<'src, char>>> + Clone {
    recursive(|nested| {
        choice((
            // Nested braces: '{' + inner content + '}'
            just('{')
                .then(nested)
                .then(just('}'))
                .map(|((open, inner), close)| format!("{}{}{}", open, inner, close)),
            // Any character except '{' and '}'
            none_of("{}").map(|c: char| c.to_string()),
        ))
        .repeated()
        .collect::<Vec<String>>()
        .map(|parts| parts.join(""))
    })
}

/// Parse `@"Name"` or `@"Lib:Name"` - quoted library reference
fn quoted_library_ref_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("@\"")
        .ignore_then(none_of("\"").repeated().collect::<String>())
        .then_ignore(just('"'))
        .map_with(|name, e| {
            let lib_ref = parse_library_ref_string(&name);
            (Node::LibraryRef(lib_ref), to_range(e.span()))
        })
}

/// Parse `@Name` - simple library reference (no spaces allowed in name)
fn simple_library_ref_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just('@')
        .ignore_then(
            // Identifier: starts with letter or underscore, followed by letters, digits, underscores, hyphens
            any()
                .filter(|c: &char| c.is_alphabetic() || *c == '_')
                .then(
                    any()
                        .filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '-')
                        .repeated()
                        .collect::<String>(),
                )
                .map(|(first, rest)| format!("{}{}", first, rest)),
        )
        .map_with(|name, e| {
            let lib_ref = LibraryRef::new(name);
            (Node::LibraryRef(lib_ref), to_range(e.span()))
        })
}

/// Parse `# comment to end of line`
fn comment_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just('#')
        .ignore_then(none_of("\n").repeated().collect::<String>())
        .map_with(|text, e| (Node::Comment(text.trim().to_string()), to_range(e.span())))
}

/// Parse plain text - everything that's not a special construct
fn text_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    // Stop at special chars: {, @, #
    // Also stop at } to avoid consuming closing braces
    none_of("{@#}")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|value, e| (Node::Text(value), to_range(e.span())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Cardinality, SlotDefKind};

    // =========================================================================
    // Textarea Slot tests (v0.1 DSL)
    // =========================================================================

    #[test]
    fn parses_textarea_slot() {
        let src = "{{ scene description }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "scene description");
                assert!(matches!(slot.kind.0, SlotKind::Textarea));
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_textarea_slot_with_simple_name() {
        let src = "{{ name }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "name");
                assert!(matches!(slot.kind.0, SlotKind::Textarea));
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_textarea_slot_with_quoted_label() {
        let src = r#"{{ "Character Description" }}"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Character Description");
                assert!(matches!(slot.kind.0, SlotKind::Textarea));
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    // =========================================================================
    // Pick Slot tests (v0.1 DSL)
    // =========================================================================

    #[test]
    fn parses_pick_slot_with_variable_ref() {
        let src = "{{ Eyes: pick(@Eyes) }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Eyes");
                match &slot.kind.0 {
                    SlotKind::Pick(pick) => {
                        assert_eq!(pick.sources.len(), 1);
                        match &pick.sources[0].0 {
                            PickSource::VariableRef(lib_ref) => {
                                assert_eq!(lib_ref.variable, "Eyes");
                            }
                            other => panic!("expected VariableRef, got {:?}", other),
                        }
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_pick_slot_with_multiple_sources() {
        let src = r#"{{ Style: pick(@Hair, windswept, "option, comma") }}"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Style");
                match &slot.kind.0 {
                    SlotKind::Pick(pick) => {
                        assert_eq!(pick.sources.len(), 3);
                        assert!(matches!(&pick.sources[0].0, PickSource::VariableRef(_)));
                        assert!(matches!(&pick.sources[1].0, PickSource::Literal { value, quoted: false } if value == "windswept"));
                        assert!(matches!(&pick.sources[2].0, PickSource::Literal { value, quoted: true } if value == "option, comma"));
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_pick_slot_with_one_operator() {
        let src = "{{ Camera: pick(@Framing) | one }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Camera");
                match &slot.kind.0 {
                    SlotKind::Pick(pick) => {
                        assert_eq!(pick.operators.len(), 1);
                        assert!(matches!(&pick.operators[0].0, PickOperator::One));
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_pick_slot_with_many_operator() {
        let src = r#"{{ Tags: pick(@Tags) | many(max=3, sep=", ") }}"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Tags");
                match &slot.kind.0 {
                    SlotKind::Pick(pick) => {
                        assert_eq!(pick.operators.len(), 1);
                        match &pick.operators[0].0 {
                            PickOperator::Many(spec) => {
                                assert_eq!(spec.max, Some(3));
                                assert_eq!(spec.sep, Some(", ".to_string()));
                            }
                            other => panic!("expected Many, got {:?}", other),
                        }
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_pick_slot_with_quoted_label() {
        let src = r#"{{ "Character Eyes": pick(@Eyes, @"Eye Color") | one }}"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                assert_eq!(slot.label.0, "Character Eyes");
                match &slot.kind.0 {
                    SlotKind::Pick(pick) => {
                        assert_eq!(pick.sources.len(), 2);
                        assert_eq!(pick.operators.len(), 1);
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    #[test]
    fn parses_pick_slot_defaults_to_many() {
        let src = "{{ label: pick(@Eyes) }}";
        let tmpl = parse_template(src).expect("should parse");

        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::SlotBlock(slot) => {
                let def = slot.to_definition().expect("should normalize");
                match def.kind {
                    SlotDefKind::Pick { cardinality, sep, .. } => {
                        assert!(matches!(cardinality, Cardinality::Many { max: None }));
                        assert_eq!(sep, ", ");
                    }
                    other => panic!("expected Pick, got {:?}", other),
                }
            }
            other => panic!("expected SlotBlock, got {:?}", other),
        }
    }

    // =========================================================================
    // Inline options tests
    // =========================================================================

    #[test]
    fn parses_inline_options_simple() {
        let src = "{red|blue|green}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 3);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "red"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "blue"));
                assert!(matches!(&options[2], OptionItem::Text(t) if t == "green"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    #[test]
    fn parses_inline_options_with_spaces() {
        let src = "{hot weather | cold weather}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 2);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "hot weather"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "cold weather"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    #[test]
    fn parses_nested_inline_options() {
        // {a|b|{c|d}} should parse as 3 options: "a", "b", "{c|d}"
        let src = "{a|b|{c|d}}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 3);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "a"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "b"));
                assert!(matches!(&options[2], OptionItem::Text(t) if t == "{c|d}"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    #[test]
    fn parses_nested_inline_options_at_start() {
        // {{a|b}|c} should parse as 2 options: "{a|b}", "c"
        let src = "{{a|b}|c}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 2);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "{a|b}"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "c"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    #[test]
    fn parses_deeply_nested_inline_options() {
        // {a|{b|{c|d}}} should parse as 2 options: "a", "{b|{c|d}}"
        let src = "{a|{b|{c|d}}}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 2);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "a"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "{b|{c|d}}"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    #[test]
    fn parses_nested_inline_options_with_library_ref() {
        // {@Hair|{red|blue} hair} should parse as 2 options
        let src = "{@Hair|{red|blue} hair}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::InlineOptions(options) => {
                assert_eq!(options.len(), 2);
                assert!(matches!(&options[0], OptionItem::Text(t) if t == "@Hair"));
                assert!(matches!(&options[1], OptionItem::Text(t) if t == "{red|blue} hair"));
            }
            other => panic!("expected InlineOptions, got {:?}", other),
        }
    }

    // =========================================================================
    // Library reference tests
    // =========================================================================

    #[test]
    fn parses_simple_library_ref() {
        let src = "@Hair";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, None);
                assert_eq!(lib_ref.variable, "Hair");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_simple_library_ref_with_underscore() {
        let src = "@Hair_Color";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, None);
                assert_eq!(lib_ref.variable, "Hair_Color");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_simple_library_ref_with_hyphen() {
        let src = "@hair-color";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, None);
                assert_eq!(lib_ref.variable, "hair-color");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_quoted_library_ref() {
        let src = r#"@"Eye Color""#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, None);
                assert_eq!(lib_ref.variable, "Eye Color");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_qualified_library_ref() {
        let src = r#"@"MyLib:Hair""#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, Some("MyLib".to_string()));
                assert_eq!(lib_ref.variable, "Hair");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_qualified_library_ref_with_spaces() {
        let src = r#"@"My Library:Eye Color""#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::LibraryRef(lib_ref) => {
                assert_eq!(lib_ref.library, Some("My Library".to_string()));
                assert_eq!(lib_ref.variable, "Eye Color");
            }
            other => panic!("expected LibraryRef, got {:?}", other),
        }
    }

    // =========================================================================
    // Comment tests
    // =========================================================================

    #[test]
    fn parses_comment() {
        let src = "# this is a comment";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::Comment(text) => assert_eq!(text, "this is a comment"),
            other => panic!("expected Comment, got {:?}", other),
        }
    }

    // =========================================================================
    // Plain text tests
    // =========================================================================

    #[test]
    fn parses_plain_text() {
        let src = "Plain text here";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::Text(value) => assert_eq!(value, "Plain text here"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    // =========================================================================
    // Mixed template tests
    // =========================================================================

    #[test]
    fn parses_mixed_template() {
        let src = "@Hair, @Eyes, with {red|blue} accents";
        let tmpl = parse_template(src).expect("should parse");

        // Should have: @Hair, Text(", "), @Eyes, Text(", with "), InlineOptions, Text(" accents")
        let node_types: Vec<&str> = tmpl
            .nodes
            .iter()
            .map(|(node, _)| match node {
                Node::Text(_) => "Text",
                Node::InlineOptions(_) => "InlineOptions",
                Node::LibraryRef(_) => "LibraryRef",
                Node::SlotBlock(_) => "SlotBlock",
                Node::Comment(_) => "Comment",
            })
            .collect();

        assert!(node_types.contains(&"LibraryRef"));
        assert!(node_types.contains(&"InlineOptions"));
        assert!(node_types.contains(&"Text"));
    }

    #[test]
    fn parses_template_with_slot() {
        let src = "A {{ character type }} with @Hair stands in {{ scene }}.";
        let tmpl = parse_template(src).expect("should parse");

        let node_types: Vec<&str> = tmpl
            .nodes
            .iter()
            .map(|(node, _)| match node {
                Node::Text(_) => "Text",
                Node::InlineOptions(_) => "InlineOptions",
                Node::LibraryRef(_) => "LibraryRef",
                Node::SlotBlock(_) => "SlotBlock",
                Node::Comment(_) => "Comment",
            })
            .collect();

        assert!(node_types.contains(&"LibraryRef"));
        assert!(node_types.contains(&"SlotBlock"));
        assert!(node_types.contains(&"Text"));

        // Count slots
        let slot_count = tmpl
            .nodes
            .iter()
            .filter(|(node, _)| matches!(node, Node::SlotBlock(_)))
            .count();
        assert_eq!(slot_count, 2);
    }

    #[test]
    fn parses_template_with_inline_comment() {
        let src = "@Hair, @Eyes  # inline comment";
        let tmpl = parse_template(src).expect("should parse");

        let has_comment = tmpl
            .nodes
            .iter()
            .any(|(node, _)| matches!(node, Node::Comment(_)));
        assert!(has_comment);

        let has_lib_ref = tmpl
            .nodes
            .iter()
            .any(|(node, _)| matches!(node, Node::LibraryRef(_)));
        assert!(has_lib_ref);
    }

    #[test]
    fn parses_complex_template() {
        let src = r#"# Random character
@Hair, @"Eye Color"
A {big|small} {cat|dog}
{{ description }}
"#;
        let tmpl = parse_template(src).expect("should parse");

        let node_types: Vec<&str> = tmpl
            .nodes
            .iter()
            .map(|(node, _)| match node {
                Node::Text(_) => "Text",
                Node::InlineOptions(_) => "InlineOptions",
                Node::LibraryRef(_) => "LibraryRef",
                Node::SlotBlock(_) => "SlotBlock",
                Node::Comment(_) => "Comment",
            })
            .collect();

        assert!(node_types.contains(&"Comment"));
        assert!(node_types.contains(&"LibraryRef"));
        assert!(node_types.contains(&"InlineOptions"));
        assert!(node_types.contains(&"SlotBlock"));
        assert!(node_types.contains(&"Text"));
    }

    // =========================================================================
    // Span tests
    // =========================================================================

    #[test]
    fn spans_are_correct_for_library_ref() {
        let src = "@Hair";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (_node, span) = &tmpl.nodes[0];
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
    }

    #[test]
    fn spans_are_correct_for_inline_options() {
        let src = "{a|b}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (_node, span) = &tmpl.nodes[0];
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
    }

    // =========================================================================
    // Duplicate label error tests
    // =========================================================================

    #[test]
    fn duplicate_labels_error() {
        let src = "{{ Name }} and {{ Name }}";
        let result = parse_template(src);

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ParseError::DuplicateLabel {
                label,
                first_span,
                duplicate_span,
            } => {
                assert_eq!(label, "Name");
                // First occurrence is at position 3 (after "{{ ")
                assert_eq!(first_span.start, 3);
                // Second occurrence is at position 18 (after " and {{ ")
                assert_eq!(duplicate_span.start, 18);
            }
            other => panic!("expected DuplicateLabel error, got {:?}", other),
        }
    }

    #[test]
    fn different_labels_ok() {
        let src = "{{ Name }} and {{ Age }}";
        let result = parse_template(src);
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_pick_labels_error() {
        let src = "{{ Choice: pick(@A) }} or {{ Choice: pick(@B) }}";
        let result = parse_template(src);

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::DuplicateLabel { label, .. } => {
                assert_eq!(label, "Choice");
            }
            other => panic!("expected DuplicateLabel error, got {:?}", other),
        }
    }
}
