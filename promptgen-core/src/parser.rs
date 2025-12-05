use chumsky::prelude::*;
use chumsky::{error::Simple, extra, span::SimpleSpan};

// Assuming these exist in your crate::ast / crate::span
use crate::ast::{Expr, Node, Op, Template};
use crate::span::Span;

#[derive(Debug, thiserror::Error)]
pub enum ParseError<'a> {
    #[error("parse error(s): {0:?}")]
    Chumsky(Vec<Simple<'a, char>>),
}

/// Helper to convert Chumsky spans to your custom Span
fn to_range(span: SimpleSpan<usize>) -> Span {
    span.start..span.end
}

pub fn parse_template(src: &str) -> Result<Template, ParseError<'_>> {
    // We map the error to our custom error type
    let result = template_parser().parse(src);

    match result.into_result() {
        Ok(tmpl) => Ok(tmpl),
        Err(errs) => Err(ParseError::Chumsky(errs)),
    }
}

fn template_parser<'src>() -> impl Parser<'src, &'src str, Template, extra::Err<Simple<'src, char>>>
{
    // 1. Define low-level helpers
    //    Note: .ignored() is useful for whitespace
    let whitespace = any().filter(|c: &char| c.is_whitespace()).repeated(); // <--- This returns a parser object, not a function

    // 2. Expression Parsing (Logic from your `expr` and `string_lit` functions)
    //    We define this first so it can be used in the block parsers below
    let string_lit_inner = just('"')
        .ignore_then(none_of("\"").repeated().collect::<String>())
        .then_ignore(just('"'));

    let string_lit_expr = string_lit_inner.map(Expr::Literal);

    let op_some = just("some").to(Op::Some);

    let op_exclude = just("excludeGroup")
        .ignore_then(string_lit_inner.delimited_by(
            whitespace.then(just('(')).then(whitespace),
            whitespace.then(just(')')),
        ))
        .map(Op::ExcludeGroup);

    let op_assign = just("assign")
        .ignore_then(string_lit_inner.delimited_by(
            whitespace.then(just('(')).then(whitespace),
            whitespace.then(just(')')),
        ))
        .map(Op::Assign);

    let pipe = just('|').padded_by(whitespace).ignored();
    let op = choice((op_some, op_exclude, op_assign)).padded_by(whitespace);

    let expr = string_lit_expr
        .then(
            // Parse the leading pipe and whitespace, then ignore its output (`()`),
            // ensuring the output of `op` is the value collected by .repeated()
            pipe.ignore_then(op).repeated().collect::<Vec<Op>>(),
        )
        .map(|(base, ops)| {
            // `ops` is now Vec<Op>, so `.is_empty()` works.
            if ops.is_empty() {
                base
            } else {
                Expr::Pipeline(Box::new(base), ops)
            }
        });

    // 3. Define the Node Parsers

    // [[ expr ]]
    let expr_block_node = just("[[")
        .ignore_then(expr.padded_by(whitespace))
        .then_ignore(just("]]"))
        .map_with(|expr, e| (Node::ExprBlock(expr), to_range(e.span())));

    // {{ SlotName }}
    let freeform_slot_node = just("{{")
        .ignore_then(
            none_of("}")
                .repeated()
                .collect::<String>()
                .map(|s| s.trim().to_string()),
        )
        .then_ignore(just("}}"))
        .map_with(|name, e| (Node::FreeformSlot(name), to_range(e.span())));

    // {GroupName}
    let group_ref_node = just('{')
        .ignore_then(
            none_of("}\n")
                .repeated()
                .collect::<String>()
                .map(|s| s.trim().to_string()),
        )
        .then_ignore(just('}'))
        .map_with(|name, e| (Node::GroupRef(name), to_range(e.span())));

    // # Comment
    let comment_node = just('#')
        .ignore_then(none_of("\n").repeated().collect::<String>())
        .map_with(|text, e| (Node::Comment(text.trim().to_string()), to_range(e.span())));

    // Plain text
    // Stops at special chars: {, [, #
    let text_node = none_of("{[#")
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|value, e| (Node::Text(value), to_range(e.span())));

    // 4. Combine them into the final sequence
    choice((
        expr_block_node,
        freeform_slot_node,
        group_ref_node,
        comment_node,
        text_node,
    ))
    .repeated()
    .collect::<Vec<_>>()
    .map(|nodes| Template { nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_ref() {
        let src = "{Hair}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::GroupRef(name) => assert_eq!(name, "Hair"),
            other => panic!("expected GroupRef, got {:?}", other),
        }
    }

    #[test]
    fn parses_freeform_slot() {
        let src = "{{ SceneDescription }}";
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::FreeformSlot(name) => assert_eq!(name, "SceneDescription"),
            other => panic!("expected FreeformSlot, got {:?}", other),
        }
    }

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

    #[test]
    fn parses_expr_block_literal() {
        let src = r#"[[ "Eyes" ]]"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::ExprBlock(Expr::Literal(value)) => assert_eq!(value, "Eyes"),
            other => panic!("expected ExprBlock with Literal, got {:?}", other),
        }
    }

    #[test]
    fn parses_expr_block_with_pipeline() {
        let src = r#"[[ "Eyes" | some | assign("eyes") ]]"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::ExprBlock(Expr::Pipeline(base, ops)) => {
                match base.as_ref() {
                    Expr::Literal(value) => assert_eq!(value, "Eyes"),
                    other => panic!("expected Literal base, got {:?}", other),
                }
                assert_eq!(ops.len(), 2);
                assert!(matches!(ops[0], Op::Some));
                match &ops[1] {
                    Op::Assign(name) => assert_eq!(name, "eyes"),
                    other => panic!("expected Assign op, got {:?}", other),
                }
            }
            other => panic!("expected ExprBlock with Pipeline, got {:?}", other),
        }
    }

    #[test]
    fn parses_expr_block_with_exclude_group() {
        let src = r#"[[ "Hair" | excludeGroup("blonde") ]]"#;
        let tmpl = parse_template(src).expect("should parse");

        assert_eq!(tmpl.nodes.len(), 1);
        let (node, _span) = &tmpl.nodes[0];
        match node {
            Node::ExprBlock(Expr::Pipeline(base, ops)) => {
                match base.as_ref() {
                    Expr::Literal(value) => assert_eq!(value, "Hair"),
                    other => panic!("expected Literal base, got {:?}", other),
                }
                assert_eq!(ops.len(), 1);
                match &ops[0] {
                    Op::ExcludeGroup(name) => assert_eq!(name, "blonde"),
                    other => panic!("expected ExcludeGroup op, got {:?}", other),
                }
            }
            other => panic!("expected ExprBlock with Pipeline, got {:?}", other),
        }
    }

    #[test]
    fn parses_mixed_template() {
        let src = r#"
        {Hair}
        [[ "Eyes" | some | assign("eyes") ]]
        {{ SceneDescription }}
        # this is a comment
        Plain text here
        "#;

        let tmpl = parse_template(src).expect("should parse");
        assert!(!tmpl.nodes.is_empty());

        // Verify we have the expected node types in order
        let node_types: Vec<&str> = tmpl
            .nodes
            .iter()
            .map(|(node, _)| match node {
                Node::Text(_) => "Text",
                Node::GroupRef(_) => "GroupRef",
                Node::ExprBlock(_) => "ExprBlock",
                Node::FreeformSlot(_) => "FreeformSlot",
                Node::Comment(_) => "Comment",
            })
            .collect();

        // Template starts with newline (Text), then GroupRef, etc.
        assert!(node_types.contains(&"GroupRef"));
        assert!(node_types.contains(&"ExprBlock"));
        assert!(node_types.contains(&"FreeformSlot"));
        assert!(node_types.contains(&"Comment"));
        assert!(node_types.contains(&"Text"));
    }
}
