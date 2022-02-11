//! Parse a Markdown input string into a sequence of Markdown abstract syntax tree
//! [`Node`]s.
//!
//! This module compensates for the fact that the `pulldown-cmark` crate is focused on
//! efficient incremental output (pull parsing), and consequently doesn't provide it's own
//! AST types.

use std::collections::HashSet;

use pulldown_cmark::{self as md, Event, HeadingLevel, Tag};

pub(crate) fn parse_markdown_to_ast(input: &str) -> Vec<Node> {
    // Set up options and parser. Strikethroughs are not part of the CommonMark standard
    // and we therefore must enable it explicitly.
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    let mut parser = md::Parser::new_ext(input, options);

    let mut builder = AstBuilder::default();

    loop {
        let event = match parser.next() {
            Some(event) => event,
            None => break,
        };

        // println!("event: {:?}", event);

        match event {
            Event::Start(tag) => builder.start_tag(tag),
            Event::End(tag) => builder.end_tag(tag),
            Event::Text(text) => builder.add_inline_text(text.to_string()),
            Event::Html(_) => println!("warning: skipping inline HTML"),
            _ => todo!("handle: {event:?}"),
        }
    }

    builder.complete
}

//======================================
// AST Representation
//======================================

/// A piece of structural Markdown content.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Paragraph(Vec<TextNode>),
    List(Vec<ListItem>),
    Heading(HeadingLevel, Vec<TextNode>),
    CodeBlock(Option<String>, Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem(pub Vec<Node>);

/// A piece of textual Markdown content.
#[derive(Debug, Clone, PartialEq)]
pub enum TextNode {
    Text(String, HashSet<TextStyle>),
    Link {
        label: Vec<TextNode>,
        destination: String,
    },
    SoftBreak,
    HardBreak,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextStyle {
    Emphasis,
    Strong,
    Strikethrough,
}

//======================================
// AST Builder
//======================================

#[derive(Default)]
struct AstBuilder {
    complete: Vec<Node>,

    /// This will typically be very shallow.
    current: Vec<Node>,
    active_styles: HashSet<TextStyle>,
}

impl AstBuilder {
    fn add_inline_text(&mut self, text: String) {
        let text_node = TextNode::Text(text.clone(), self.active_styles.clone());

        match self.current.last_mut() {
            Some(Node::Paragraph(text_nodes)) => text_nodes.push(text_node),
            // FIXME: Handle:
            // * hello
            //
            //   this is more indented item text.
            Some(Node::List(items)) => {
                let ListItem(nodes) = items.last_mut().unwrap();

                match nodes.last_mut() {
                    None => nodes.push(Node::Paragraph(vec![text_node])),
                    Some(Node::Paragraph(text_nodes)) => {
                        text_nodes.push(text_node);
                    },
                    _ => todo!(),
                }
            },
            Some(Node::Heading(_, text_nodes)) => text_nodes.push(text_node),
            Some(Node::CodeBlock(_, code_text)) => {
                assert!(code_text.is_none());
                *code_text = Some(text);
            },
            None => todo!(),
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.start_paragraph(),
            // TODO: handle these other heading elements?
            Tag::Heading(level, _, _) => self.start_heading(level),
            Tag::Emphasis => {
                self.active_styles.insert(TextStyle::Emphasis);
            },
            Tag::Strong => {
                self.active_styles.insert(TextStyle::Strong);
            },
            Tag::Strikethrough => {
                self.active_styles.insert(TextStyle::Strikethrough);
            },
            // TODO: Handle numbered lists.
            Tag::List(_) => self.start_list(),
            Tag::Item => match self.current.last_mut().unwrap() {
                Node::List(items) => items.push(ListItem(vec![])),
                node @ Node::Paragraph(_)
                | node @ Node::Heading(_, _)
                | node @ Node::CodeBlock(_, _) => {
                    panic!("unexpected list item in {node:?}")
                },
            },
            Tag::CodeBlock(kind) => {
                let fence_label = match kind {
                    md::CodeBlockKind::Indented => None,
                    md::CodeBlockKind::Fenced(label) => Some(label.to_string()),
                };
                self.current.push(Node::CodeBlock(fence_label, None))
            },
            _ => todo!("handle: {tag:?}"),
        }
    }

    fn end_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.end_paragraph(),
            // TODO: handle these other heading elements?
            Tag::Heading(_, _, _) => self.end_heading(),
            Tag::Emphasis => {
                assert!(self.active_styles.remove(&TextStyle::Emphasis));
            },
            Tag::Strong => {
                assert!(self.active_styles.remove(&TextStyle::Strong));
            },
            Tag::Strikethrough => {
                assert!(self.active_styles.remove(&TextStyle::Strikethrough))
            },
            // TODO: Handle numbered lists.
            Tag::List(_) => self.end_list(),
            Tag::Item => {
                assert!(matches!(self.current.last(), Some(Node::List(_))));
            },
            Tag::CodeBlock(_) => self.end_code_block(),
            _ => todo!("handle: {tag:?}"),
        }
    }

    // Structural

    fn start_paragraph(&mut self) {
        start_paragraph(&mut self.current)
    }

    fn end_paragraph(&mut self) {
        let paragraph = match self.current.pop().unwrap() {
            node @ Node::Paragraph(_) => node,
            Node::List(_) | Node::Heading(_, _) | Node::CodeBlock(_, _) => {
                panic!("expected Node::Paragraph at end of paragraph")
            },
        };

        match self.current.last_mut() {
            None => self.complete.push(paragraph),
            _ => todo!(),
        }
    }

    fn start_list(&mut self) {
        match self.current.last_mut() {
            None => self.current.push(Node::List(vec![])),
            _ => todo!("start sub-list"),
        }
    }

    fn end_list(&mut self) {
        let items: Vec<ListItem> = match self.current.pop().unwrap() {
            Node::List(items) => items,
            Node::Paragraph(_) | Node::Heading(_, _) | Node::CodeBlock(_, _) => {
                panic!("expected Node::List at end of list")
            },
        };

        match self.current.last_mut() {
            None => self.complete.push(Node::List(items)),
            _ => todo!("end sub-list"),
        }
    }

    fn start_heading(&mut self, level: HeadingLevel) {
        match self.current.last_mut() {
            Some(Node::Paragraph(_)) => self.current.push(Node::Heading(level, vec![])),
            // FIXME: Fix and add test for this.
            Some(Node::List(_)) => unimplemented!("heading nested instead list"),
            Some(Node::Heading(_, _)) => panic!("heading nested inside heading"),
            Some(Node::CodeBlock(_, _)) => panic!("heading nested inside code block"),
            None => self.current.push(Node::Heading(level, vec![])),
        }
    }

    fn end_heading(&mut self) {
        let heading = match self.current.pop().unwrap() {
            node @ Node::Heading(_, _) => node,
            Node::List(_) | Node::Paragraph(_) | Node::CodeBlock(_, _) => {
                panic!("expected Node::Heading at end of heading")
            },
        };

        match self.current.last_mut() {
            None => self.complete.push(heading),
            _ => todo!("start nested heading"),
        }
    }

    fn end_code_block(&mut self) {
        let code_block = match self.current.pop().unwrap() {
            node @ Node::CodeBlock(_, _) => node,
            Node::List(_) | Node::Paragraph(_) | Node::Heading(_, _) => {
                panic!("expected Node::Heading at end of code block")
            },
        };

        match self.current.last_mut() {
            None => self.complete.push(code_block),
            _ => todo!("end nested code block"),
        }
    }
}

fn start_paragraph(nodes: &mut Vec<Node>) {
    match nodes.last_mut() {
        Some(Node::Paragraph(prev_paragraph)) => {
            debug_assert!(!prev_paragraph.is_empty());

            nodes.push(Node::Paragraph(vec![]));
        },
        Some(Node::List(list_items)) => {
            // debug_assert!(!matches!(inner_nodes.last(), Some(Node::Paragraph(_))));

            // TODO: Add test that this is not `nodes.push(..)`.
            // * hello
            //   - world
            //
            //     how are you
            //
            //   doing today?
            match list_items.last_mut() {
                // TODO: Test this control flow path.
                Some(ListItem(inner_nodes)) => {
                    inner_nodes.push(Node::Paragraph(vec![]));
                },
                // TODO: Test this control flow path.
                None => list_items.push(ListItem(vec![Node::Paragraph(vec![])])),
            }
        },
        Some(Node::Heading(_, _)) => nodes.push(Node::Paragraph(vec![])),
        Some(Node::CodeBlock(_, _)) => panic!("paragraph nested inside code block"),
        None => nodes.push(Node::Paragraph(vec![])),
    }
}

//======================================
// Tests
//======================================

#[test]
fn tests() {
    assert_eq!(
        parse_markdown_to_ast("hello"),
        vec![Node::Paragraph(vec![TextNode::Text(
            "hello".into(),
            HashSet::new()
        )])]
    );

    //--------------
    // Styled text
    //--------------

    assert_eq!(
        parse_markdown_to_ast("*hello*"),
        vec![Node::Paragraph(vec![TextNode::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Emphasis])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("**hello**"),
        vec![Node::Paragraph(vec![TextNode::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strong])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("~~hello~~"),
        vec![Node::Paragraph(vec![TextNode::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strikethrough])
        )])]
    );

    //--------------
    // Lists
    //--------------

    assert_eq!(
        parse_markdown_to_ast("* hello"),
        vec![Node::List(vec![ListItem(vec![Node::Paragraph(vec![
            TextNode::Text("hello".into(), HashSet::new())
        ])])])]
    );

    // List items with styled text

    assert_eq!(
        parse_markdown_to_ast("* *hello*"),
        vec![Node::List(vec![ListItem(vec![Node::Paragraph(vec![
            TextNode::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Emphasis])
            )
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* **hello**"),
        vec![Node::List(vec![ListItem(vec![Node::Paragraph(vec![
            TextNode::Text("hello".into(), HashSet::from_iter(vec![TextStyle::Strong]))
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* ~~hello~~"),
        vec![Node::List(vec![ListItem(vec![Node::Paragraph(vec![
            TextNode::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Strikethrough])
            )
        ])])])]
    );
}
