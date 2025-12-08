use chumsky::prelude::*;
use chumsky::{error::Simple, extra, span::SimpleSpan};

use crate::ast::{LibraryRef, Node, OptionItem, Template};
use crate::span::Span;

#[derive(Debug, thiserror::Error)]
pub enum ParseError<'a> {
    #[error("parse error(s): {0:?}")]
    Chumsky(Vec<Simple<'a, char>>),
}

/// Helper to convert Chumsky spans to our custom Span
fn to_range(span: SimpleSpan<usize>) -> Span {
    span.start..span.end
}

/// Parse a library reference string (the part after @ or inside quotes).
///
/// Examples:
/// - `"Hair"` -> LibraryRef { library: None, group: "Hair" }
/// - `"Eye Color"` -> LibraryRef { library: None, group: "Eye Color" }
/// - `"MyLib:Hair"` -> LibraryRef { library: Some("MyLib"), group: "Hair" }
fn parse_library_ref_string(s: &str) -> LibraryRef {
    if let Some(colon_pos) = s.find(':') {
        let library = s[..colon_pos].to_string();
        let group = s[colon_pos + 1..].to_string();
        LibraryRef::qualified(library, group)
    } else {
        LibraryRef::new(s)
    }
}

pub fn parse_template(src: &str) -> Result<Template, ParseError<'_>> {
    let result = template_parser().parse(src);

    match result.into_result() {
        Ok(tmpl) => Ok(tmpl),
        Err(errs) => Err(ParseError::Chumsky(errs)),
    }
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

    let slot_node = slot_parser();
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

/// Parse `{{ slot name }}` - user-provided slot
fn slot_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just("{{")
        .ignore_then(
            none_of("}")
                .repeated()
                .collect::<String>()
                .map(|s| s.trim().to_string()),
        )
        .then_ignore(just("}}"))
        .map_with(|name, e| (Node::Slot(name), to_range(e.span())))
}

/// Parse `{a|b|c}` - inline options
/// Options can contain nested grammar (like @Hair)
fn inline_options_parser<'src>(
) -> impl Parser<'src, &'src str, (Node, Span), extra::Err<Simple<'src, char>>> + Clone {
    just('{')
        .ignore_then(
            // Parse content between braces, split by |
            none_of("}").repeated().collect::<String>(),
        )
        .then_ignore(just('}'))
        .map_with(|content, e| {
            // Split by | and parse each option
            let options: Vec<OptionItem> = content
                .split('|')
                .map(|opt| {
                    let opt = opt.trim();
                    // Check if option contains grammar (@ for lib refs)
                    if opt.contains('@') {
                        // For now, treat as text - nested parsing will be added later
                        // TODO: Parse nested grammar in options
                        OptionItem::Text(opt.to_string())
                    } else {
                        OptionItem::Text(opt.to_string())
                    }
                })
                .collect();

            (Node::InlineOptions(options), to_range(e.span()))
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

    // =========================================================================
    // Slot tests
    // =========================================================================

    #[test]
    fn parses_slot() {
        let src = "{{ scene description }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::Slot(name) => assert_eq!(name, "scene description"),
            other => panic!("expected Slot, got {:?}", other),
        }
    }

    #[test]
    fn parses_slot_with_simple_name() {
        let src = "{{ name }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::Slot(name) => assert_eq!(name, "name"),
            other => panic!("expected Slot, got {:?}", other),
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
                assert_eq!(lib_ref.group, "Hair");
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
                assert_eq!(lib_ref.group, "Hair_Color");
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
                assert_eq!(lib_ref.group, "hair-color");
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
                assert_eq!(lib_ref.group, "Eye Color");
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
                assert_eq!(lib_ref.group, "Hair");
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
                assert_eq!(lib_ref.group, "Eye Color");
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
                Node::Slot(_) => "Slot",
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
                Node::Slot(_) => "Slot",
                Node::Comment(_) => "Comment",
            })
            .collect();

        assert!(node_types.contains(&"LibraryRef"));
        assert!(node_types.contains(&"Slot"));
        assert!(node_types.contains(&"Text"));

        // Count slots
        let slot_count = tmpl
            .nodes
            .iter()
            .filter(|(node, _)| matches!(node, Node::Slot(_)))
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
                Node::Slot(_) => "Slot",
                Node::Comment(_) => "Comment",
            })
            .collect();

        assert!(node_types.contains(&"Comment"));
        assert!(node_types.contains(&"LibraryRef"));
        assert!(node_types.contains(&"InlineOptions"));
        assert!(node_types.contains(&"Slot"));
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
}
