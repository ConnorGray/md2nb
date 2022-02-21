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
///
/// *CommonMark Spec:* [blocks](https://spec.commonmark.org/0.30/#blocks),
/// [container blocks](https://spec.commonmark.org/0.30/#container-blocks)
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Text),
    List(Vec<ListItem>),
    Heading(HeadingLevel, Text),
    /// An indented or fenced code block.
    ///
    /// *CommonMark Spec:* [indented code blocks](https://spec.commonmark.org/0.30/#indented-code-blocks),
    /// [fenced code blocks](https://spec.commonmark.org/0.30/#fenced-code-blocks)
    CodeBlock {
        /// If this `CodeBlock` is a fenced code block, this is its info string.
        ///
        /// *CommonMark Spec:* [info string](https://spec.commonmark.org/0.30/#info-string)
        info_string: Option<String>,
        code: String,
    },
    /// *CommonMark Spec:* [block quotes](https://spec.commonmark.org/0.30/#block-quotes)
    BlockQuote(Vec<Block>),
    Table {
        headers: Vec<Text>,
        rows: Vec<Vec<Text>>,
    },
    /// *CommonMark Spec: [thematic breaks](https://spec.commonmark.org/0.30/#thematic-breaks)
    Rule,
}

/// A sequence of [`TextSpan`]s that make up a block of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text(pub Vec<TextSpan>);

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem(pub Vec<Block>);

/// A piece of textual Markdown content.
#[derive(Debug, Clone, PartialEq)]
pub enum TextSpan {
    Text(String, HashSet<TextStyle>),
    Code(String),
    Link { label: Text, destination: String },
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
            Event::Rule => false,
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
            Tag::BlockQuote => false,
            Tag::Table(_) => false,
            Tag::TableHead | Tag::TableRow => unreachable!(),
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
                complete.push(Block::Paragraph(Text(mem::replace(
                    &mut text_spans,
                    vec![],
                ))));
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
                        text_spans.push(TextSpan::from_link(
                            link_type,
                            text,
                            destination.to_string(),
                            label.to_string(),
                        ))
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
                        let code_text = text_to_string(text_spans);

                        complete.push(Block::CodeBlock {
                            info_string: fence_label,
                            code: code_text,
                        })
                    },
                    Tag::BlockQuote => {
                        let blocks = events_to_blocks(events);
                        complete.push(Block::BlockQuote(blocks))
                    },
                    // TODO: Support table column alignments.
                    Tag::Table(_alignments) => {
                        let mut events = events.into_iter();
                        let header_events = match events.next().unwrap() {
                            UnflattenedEvent::Event(_) => panic!(),
                            UnflattenedEvent::Nested { tag, events } => {
                                assert!(tag == Tag::TableHead);
                                events
                            },
                        };

                        let mut headers = Vec::new();

                        for table_cell in header_events {
                            let table_cell_text = unwrap_text(
                                unwrap_table_cell(table_cell),
                                HashSet::new(),
                            );

                            headers.push(table_cell_text);
                        }

                        let mut rows = Vec::new();

                        for row_events in events {
                            let row_events = match row_events {
                                UnflattenedEvent::Event(_) => panic!(),
                                UnflattenedEvent::Nested { tag, events } => {
                                    assert!(tag == Tag::TableRow);
                                    events
                                },
                            };

                            let mut row = Vec::new();

                            for table_cell in row_events {
                                let table_cell_text = unwrap_text(
                                    unwrap_table_cell(table_cell),
                                    HashSet::new(),
                                );

                                row.push(table_cell_text);
                            }

                            rows.push(row);
                        }

                        complete.push(Block::Table { headers, rows })
                    },
                    _ => todo!("handle: {tag:?}"),
                }
            },
        }
    }

    if !text_spans.is_empty() {
        complete.push(Block::paragraph(text_spans));
    }

    complete
}

fn unwrap_text(events: Vec<UnflattenedEvent>, mut styles: HashSet<TextStyle>) -> Text {
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
                    styles.remove(&TextStyle::Emphasis);
                },
                Tag::Strong => {
                    styles.insert(TextStyle::Strong);
                    text_spans.extend(unwrap_text(events, styles.clone()));
                    styles.remove(&TextStyle::Strong);
                },
                Tag::Strikethrough => {
                    styles.insert(TextStyle::Strikethrough);
                    text_spans.extend(unwrap_text(events, styles.clone()));
                    styles.remove(&TextStyle::Strikethrough);
                },
                Tag::Paragraph => {
                    // If this is a separate paragraph, insert two hardbreaks
                    // (two newlines). Don't insert hardbreaks if there isn't any existing
                    // text content, to avoid having leading empty lines.
                    if !text_spans.is_empty() {
                        // TODO: Replace this with a new TextSpan::ParagraphBreak?
                        //       A HardBreak is just a newline.
                        text_spans.push(TextSpan::HardBreak);
                        text_spans.push(TextSpan::HardBreak);
                    }
                    text_spans.extend(unwrap_text(events, styles.clone()))
                },
                Tag::Link(link_type, destination, label) => {
                    let text = unwrap_text(events, HashSet::new());
                    text_spans.push(TextSpan::from_link(
                        link_type,
                        text,
                        destination.to_string(),
                        label.to_string(),
                    ))
                },
                _ => todo!("handle {tag:?}"),
            },
        }
    }

    Text(text_spans)
}

fn unwrap_table_cell(event: UnflattenedEvent) -> Vec<UnflattenedEvent> {
    match event {
        UnflattenedEvent::Event(_) => panic!(),
        UnflattenedEvent::Nested { tag, events } => {
            assert_eq!(tag, Tag::TableCell, "expected to get Tag::TableCell");
            events
        },
    }
}

fn text_to_string(Text(text_spans): Text) -> String {
    let mut string = String::new();

    for span in text_spans {
        match span {
            TextSpan::Text(text, styles) => {
                if !styles.is_empty() {
                    todo!("support text style(s) `{styles:?}` in string {text:?}");
                }

                string.push_str(&text);
            },
            TextSpan::SoftBreak => {
                string.push_str(" ");
            },
            TextSpan::HardBreak => {
                string.push_str("\n");
            },
            _ => todo!("handle span: {span:?}"),
        }
    }

    string
}

//======================================
// Impls
//======================================

impl TextSpan {
    fn from_link(
        link_type: LinkType,
        text: Text,
        destination: String,
        label: String,
    ) -> TextSpan {
        if !label.is_empty() {
            eprintln!("warning: link label is ignored: {label:?}");
        }

        match link_type {
            LinkType::Inline => (),
            _ => todo!("support non-inline link type: {link_type:?} (destination: {destination})"),
        }

        TextSpan::Link {
            label: text,
            destination,
        }
    }
}

impl Block {
    fn paragraph(text: Vec<TextSpan>) -> Block {
        Block::Paragraph(Text(text))
    }
}

impl IntoIterator for Text {
    type Item = TextSpan;
    type IntoIter = std::vec::IntoIter<TextSpan>;

    fn into_iter(self) -> Self::IntoIter {
        let Text(vec) = self;
        vec.into_iter()
    }
}

//======================================
// Tests
//======================================

#[test]
fn tests() {
    use pretty_assertions::assert_eq;

    assert_eq!(
        parse_markdown_to_ast("hello"),
        vec![Block::paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::new()
        )])]
    );

    //--------------
    // Styled text
    //--------------

    assert_eq!(
        parse_markdown_to_ast("*hello*"),
        vec![Block::paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Emphasis])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("**hello**"),
        vec![Block::paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strong])
        )])]
    );

    assert_eq!(
        parse_markdown_to_ast("~~hello~~"),
        vec![Block::paragraph(vec![TextSpan::Text(
            "hello".into(),
            HashSet::from_iter(vec![TextStyle::Strikethrough])
        )])]
    );

    //--------------
    // Lists
    //--------------

    assert_eq!(
        parse_markdown_to_ast("* hello"),
        vec![Block::List(vec![ListItem(vec![Block::paragraph(vec![
            TextSpan::Text("hello".into(), HashSet::new())
        ])])])]
    );

    // List items with styled text

    assert_eq!(
        parse_markdown_to_ast("* *hello*"),
        vec![Block::List(vec![ListItem(vec![Block::paragraph(vec![
            TextSpan::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Emphasis])
            )
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* **hello**"),
        vec![Block::List(vec![ListItem(vec![Block::paragraph(vec![
            TextSpan::Text("hello".into(), HashSet::from_iter(vec![TextStyle::Strong]))
        ])])])]
    );

    assert_eq!(
        parse_markdown_to_ast("* ~~hello~~"),
        vec![Block::List(vec![ListItem(vec![Block::paragraph(vec![
            TextSpan::Text(
                "hello".into(),
                HashSet::from_iter(vec![TextStyle::Strikethrough])
            )
        ])])])]
    );
}

#[test]
fn test_structure() {
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            * hello

              world
            "
        )),
        vec![Block::List(vec![ListItem(vec![
            Block::paragraph(vec![TextSpan::Text("hello".into(), Default::default())]),
            Block::paragraph(vec![TextSpan::Text("world".into(), Default::default())])
        ])])]
    );

    #[rustfmt::skip]
    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            # Example

            * A
              - A.A

                hello world

                * *A.A.A*
            "
        )),
        vec![
            Block::Heading(
                HeadingLevel::H1,
                Text(vec![TextSpan::Text("Example".into(), Default::default())])
            ),
            Block::List(vec![
                ListItem(vec![
                    Block::paragraph(vec![TextSpan::Text("A".into(), Default::default())]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.A".into(), Default::default())]),
                            Block::paragraph(vec![TextSpan::Text("hello world".into(), Default::default())]),
                            Block::List(vec![
                                ListItem(vec![
                                    Block::paragraph(vec![
                                        TextSpan::Text(
                                            "A.A.A".into(),
                                            HashSet::from_iter([TextStyle::Emphasis])
                                        )
                                    ])
                                ])
                            ])
                        ])
                    ])
                ])
            ])
        ]
    );

    #[rustfmt::skip]
    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            * A
              - A.A
                * A.A.A
              - A.B
              - A.C
            "
        )),
        vec![
            Block::List(vec![
                ListItem(vec![
                    Block::paragraph(vec![TextSpan::Text("A".into(), Default::default())]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.A".into(), Default::default())]),
                            Block::List(vec![ListItem(vec![
                                Block::paragraph(vec![TextSpan::Text("A.A.A".into(), Default::default())]),
                            ])])
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.B".into(), Default::default())]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.C".into(), Default::default())]),
                        ])
                    ])
                ])
            ])
        ]
    );

    #[rustfmt::skip]
    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            # Example

            * A
              - A.A
              - A.B
              * A.C
            "
        )),
        vec![
            Block::Heading(
                HeadingLevel::H1,
                Text(vec![TextSpan::Text("Example".into(), Default::default())])
            ),
            Block::List(vec![
                ListItem(vec![
                    Block::paragraph(vec![TextSpan::Text("A".into(), Default::default())]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.A".into(), Default::default())]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.B".into(), Default::default())]),
                        ]),
                    ]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.C".into(), Default::default())])
                        ])
                    ]),
                ]),
            ])
        ]
    );

    #[rustfmt::skip]
    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            * A
              - A.A
              - A.B

                separate paragraph

              - A.C
            "
        )),
        vec![
            Block::List(vec![
                ListItem(vec![
                    Block::paragraph(vec![TextSpan::Text("A".into(), Default::default())]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.A".into(), Default::default())]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.B".into(), Default::default())]),
                            Block::paragraph(vec![TextSpan::Text("separate paragraph".into(), Default::default())]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.C".into(), Default::default())]),
                        ])
                    ])
                ])
            ])
        ]
    );

    #[rustfmt::skip]
    assert_eq!(
        parse_markdown_to_ast(indoc!(
            "
            # Example

            * A
              - A.A
                * A.A.A
                  **soft break**

              - A.B

                separate paragraph

              - A.C
            "
        )),
        vec![
            Block::Heading(
                HeadingLevel::H1,
                Text(vec![TextSpan::Text("Example".into(), Default::default())])
            ),
            Block::List(vec![
                ListItem(vec![
                    Block::paragraph(vec![TextSpan::Text("A".into(), Default::default())]),
                    Block::List(vec![
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.A".into(), Default::default())]),
                            Block::List(vec![
                                ListItem(vec![
                                    Block::paragraph(vec![
                                        TextSpan::Text(
                                            "A.A.A".into(),
                                            Default::default(),
                                        ),
                                        TextSpan::SoftBreak,
                                        TextSpan::Text(
                                            "soft break".into(),
                                            HashSet::from_iter([TextStyle::Strong])
                                        )
                                    ]),
                                ])
                            ]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.B".into(), Default::default())]),
                            Block::paragraph(vec![TextSpan::Text("separate paragraph".into(), Default::default())]),
                        ]),
                        ListItem(vec![
                            Block::paragraph(vec![TextSpan::Text("A.C".into(), Default::default())]),
                        ]),
                    ])
                ])
            ])
        ]
    );
}
