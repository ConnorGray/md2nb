//! Parse a Markdown input string into a sequence of Markdown abstract syntax tree
//! [`Block`]s.
//!
//! This module compensates for the fact that the `pulldown-cmark` crate is focused on
//! efficient incremental output (pull parsing), and consequently doesn't provide it's own
//! AST types.

mod unflatten;


use std::{collections::HashSet, mem};

use pulldown_cmark::{self as md, Event, HeadingLevel, LinkType, Tag};

use self::unflatten::UnflattenedEvent;

//======================================
// AST Representation
//======================================

/// A piece of structural Markdown content.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<TextSpan>),
    List(Vec<ListItem>),
    Heading(HeadingLevel, Vec<TextSpan>),
    CodeBlock(Option<String>, String),
    Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem(pub Vec<Block>);

/// A piece of textual Markdown content.
#[derive(Debug, Clone, PartialEq)]
pub enum TextSpan {
    Text(String, HashSet<TextStyle>),
    Code(String),
    Link {
        label: Vec<TextSpan>,
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

pub(crate) fn parse_markdown_to_ast(input: &str) -> Vec<Block> {
    /* For Markdown parsing debugging.
    {
        let mut options = md::Options::empty();
        options.insert(md::Options::ENABLE_STRIKETHROUGH);
        let parser = md::Parser::new_ext(input, options);

        let events: Vec<_> = parser.into_iter().collect();

        println!("==== All events =====\n");
        for event in &events {
            println!("{event:?}");
        }
        println!("\n=====================\n");

        println!("==== Unflattened events =====\n");
        for event in unflatten::parse_markdown_to_unflattened_events(input) {
            println!("{event:#?}")
        }
        println!("=============================\n");
    }
    */

    let events = unflatten::parse_markdown_to_unflattened_events(input);

    events_to_blocks(events)
}

/// Returns `true` if `event` contains content that can be added "inline" with text
/// content.
///
/// `event`'s that cannot be added inline will start a new [`Block`].
fn is_inline(event: &UnflattenedEvent) -> bool {
    match event {
        UnflattenedEvent::Event(event) => match event {
            Event::Start(_) | Event::End(_) => unreachable!(),
            Event::Text(_) => true,
            Event::Code(_) => true,
            Event::SoftBreak => true,
            Event::HardBreak => true,
            // TODO: HTML could cause break to next block?
            Event::Html(_) => false,
            Event::Rule => true,
            Event::TaskListMarker(_) => false,
            Event::FootnoteReference(_) => true,
        },
        UnflattenedEvent::Nested { tag, events: _ } => match tag {
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough => true,
            Tag::Heading(_, _, _) => false,
            Tag::Paragraph => false,
            Tag::List(_) => false,
            Tag::Item => false,
            Tag::CodeBlock(_) => false,
            _ => todo!("handle tag: {tag:?}"),
        },
    }
}

fn events_to_blocks(events: Vec<UnflattenedEvent>) -> Vec<Block> {
    let mut complete: Vec<Block> = vec![];

    let mut text_spans: Vec<TextSpan> = vec![];

    for event in events {
        // println!("event: {:?}", event);

        if !is_inline(&event) {
            if !text_spans.is_empty() {
                complete.push(Block::Paragraph(mem::replace(&mut text_spans, vec![])));
            }
        }

        match event {
            UnflattenedEvent::Event(event) => match event {
                Event::Start(_) | Event::End(_) => {
                    panic!("illegal Event::{{Start, End}} in UnflattenedEvent::Event")
                },
                Event::Text(text) => {
                    text_spans.push(TextSpan::Text(text.to_string(), HashSet::new()))
                },
                Event::Code(code) => text_spans.push(TextSpan::Code(code.to_string())),
                Event::SoftBreak => text_spans.push(TextSpan::SoftBreak),
                Event::HardBreak => text_spans.push(TextSpan::HardBreak),
                Event::Html(_) => eprintln!("warning: skipping inline HTML"),
                Event::Rule => complete.push(Block::Rule),
                Event::TaskListMarker(_) | Event::FootnoteReference(_) => {
                    todo!("handle: {event:?}")
                },
            },
            UnflattenedEvent::Nested { tag, events } => {
                match tag {
                    //
                    // Inline content
                    //
                    Tag::Emphasis => {
                        text_spans.extend(unwrap_text(
                            events,
                            HashSet::from_iter([TextStyle::Emphasis]),
                        ));
                    },
                    Tag::Strong => {
                        text_spans.extend(unwrap_text(
                            events,
                            HashSet::from_iter([TextStyle::Strong]),
                        ));
                    },
                    Tag::Strikethrough => {
                        text_spans.extend(unwrap_text(
                            events,
                            HashSet::from_iter([TextStyle::Strikethrough]),
                        ));
                    },

                    Tag::Link(link_type, destination, label) => {
                        let text = unwrap_text(events, HashSet::new());

                        if !label.is_empty() {
                            eprintln!("warning: link label is ignored: {label:?}");
                        }

                        match link_type {
                            LinkType::Inline => (),
                            _ => todo!("support non-inline link type: {link_type:?} (destination: {destination})"),
                        }

                        text_spans.push(TextSpan::Link {
                            label: text,
                            destination: destination.to_string(),
                        })
                    },

                    //
                    // Block content
                    //

                    // TODO: Use the two Heading fields that are ignored here?
                    Tag::Heading(level, _, _) => {
                        complete.push(Block::Heading(
                            level,
                            unwrap_text(events, Default::default()),
                        ));
                    },
                    Tag::Paragraph => {
                        text_spans.extend(unwrap_text(events, Default::default()))
                    },
                    Tag::List(_) => {
                        let mut items: Vec<ListItem> = Vec::new();

                        for event in events {
                            if let UnflattenedEvent::Nested {
                                tag: Tag::Item,
                                events: item_events,
                            } = event
                            {
                                let item_blocks = events_to_blocks(item_events);
                                items.push(ListItem(item_blocks));
                            } else {
                                todo!("handle list element: {event:?}");
                            }
                        }

                        complete.push(Block::List(items));
                    },
                    Tag::Item => {
                        complete.extend(events_to_blocks(events));
                    },
                    Tag::CodeBlock(kind) => {
                        let fence_label = match kind {
                            md::CodeBlockKind::Indented => None,
                            md::CodeBlockKind::Fenced(label) => Some(label.to_string()),
                        };

                        let text_spans = unwrap_text(events, Default::default());

                        let mut code_text = String::new();

                        for span in text_spans {
                            match span {
                                TextSpan::Text(text, styles) => {
                                    assert!(styles.is_empty());
                                    code_text.push_str(&text);
                                },
                                _ => todo!("handle span: {span:?}"),
                            }
                        }

                        complete.push(Block::CodeBlock(fence_label, code_text))
                    },
                    _ => todo!("handle: {tag:?}"),
                }
            },
        }
    }

    if !text_spans.is_empty() {
        complete.push(Block::Paragraph(text_spans));
    }

    complete
}

fn unwrap_text(
    events: Vec<UnflattenedEvent>,
    mut styles: HashSet<TextStyle>,
) -> Vec<TextSpan> {
    let mut text_spans: Vec<TextSpan> = vec![];

    for event in events {
        match event {
            UnflattenedEvent::Event(event) => match event {
                Event::Start(_) | Event::End(_) => unreachable!(),
                Event::Text(text) => {
                    text_spans.push(TextSpan::Text(text.to_string(), styles.clone()))
                },
                Event::Code(code) => text_spans.push(TextSpan::Code(code.to_string())),
                Event::SoftBreak => text_spans.push(TextSpan::SoftBreak),
                Event::HardBreak => text_spans.push(TextSpan::HardBreak),
                Event::Html(_) => eprintln!("warning: skipping inline HTML"),
                Event::TaskListMarker(_) | Event::Rule | Event::FootnoteReference(_) => {
                    todo!("handle: {event:?}")
                },
            },
            UnflattenedEvent::Nested { tag, events } => match tag {
                Tag::Emphasis => {
                    styles.insert(TextStyle::Emphasis);
                    text_spans.extend(unwrap_text(events, styles.clone()));
                },
                Tag::Strong => {
                    styles.insert(TextStyle::Strong);
                    text_spans.extend(unwrap_text(events, styles.clone()));
                },
                Tag::Strikethrough => {
                    styles.insert(TextStyle::Strikethrough);
                    text_spans.extend(unwrap_text(events, styles.clone()));
                },
                Tag::Link(link_type, destination, label) => {
                    let text = unwrap_text(events, HashSet::new());

                    if !label.is_empty() {
                        eprintln!("warning: link label is ignored: {label:?}");
                    }

                    match link_type {
                        LinkType::Inline => (),
                        _ => todo!("support non-inline link type: {link_type:?} (destination: {destination})"),
                    }

                    text_spans.push(TextSpan::Link {
                        label: text,
                        destination: destination.to_string(),
                    })
                },
                _ => todo!("handle {tag:?}"),
            },
        }
    }

    text_spans
}

//======================================
// Tests
//======================================

#[test]
fn tests() {
    use pretty_assertions::assert_eq;

    assert_eq!(
        parse_markdown_to_ast("hello"),
        vec![Block::Paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::new()
        )])]
    );

    //--------------
    // Styled text
    //--------------

    assert_eq!(
        parse_markdown_to_ast("*hello*"),
        vec![Block::Paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Emphasis])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("**hello**"),
        vec![Block::Paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strong])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("~~hello~~"),
        vec![Block::Paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strikethrough])
        )])]
    );

    //--------------
    // Lists
    //--------------

    assert_eq!(
        parse_markdown_to_ast("* hello"),
        vec![Block::List(vec![ListItem(vec![Block::Paragraph(vec![
            TextSpan::Text("hello".into(), HashSet::new())
        ])])])]
    );

    // List items with styled text

    assert_eq!(
        parse_markdown_to_ast("* *hello*"),
        vec![Block::List(vec![ListItem(vec![Block::Paragraph(vec![
            TextSpan::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Emphasis])
            )
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* **hello**"),
        vec![Block::List(vec![ListItem(vec![Block::Paragraph(vec![
            TextSpan::Text("hello".into(), HashSet::from_iter(vec![TextStyle::Strong]))
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* ~~hello~~"),
        vec![Block::List(vec![ListItem(vec![Block::Paragraph(vec![
            TextSpan::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Strikethrough])
            )
        ])])])]
    );
}
