use pulldown_cmark::HeadingLevel;

use wolfram_expr::{Expr, Symbol};

use crate::ast::{Block, ListItem, TextSpan, TextStyle};

struct State {
    list_depth: u8,
}

pub fn block_to_cells(block: Block) -> Vec<Expr> {
    let mut state = State { list_depth: 0 };

    block_to_cells_(&mut state, block)
}

fn block_to_cells_(state: &mut State, block: Block) -> Vec<Expr> {
    match block {
        Block::Heading(level, text) => {
            let style = match level {
                HeadingLevel::H1 => "Title",
                HeadingLevel::H2 => "Chapter",
                HeadingLevel::H3 => "Section",
                HeadingLevel::H4 => "Subsection",
                HeadingLevel::H5 => "Subsubsection",
                HeadingLevel::H6 => "Subsubsubsection",
            };

            vec![Expr::normal(
                Symbol::new("System`Cell"),
                vec![text_to_text_data(text), Expr::from(style)],
            )]
        },
        Block::Paragraph(text) => vec![Expr::normal(
            Symbol::new("System`Cell"),
            vec![text_to_text_data(text), Expr::from("Text")],
        )],
        Block::List(items) => {
            let mut list_cells = Vec::new();

            state.list_depth += 1;

            for item in items {
                list_cells.extend(list_item_to_cells(state, item));
            }

            state.list_depth -= 1;

            list_cells
        },
        Block::CodeBlock(_, code_text) => {
            vec![Expr::normal(
                Symbol::new("System`Cell"),
                vec![Expr::string(code_text), Expr::string("Program")],
            )]
        },
        Block::Rule => todo!("handle markdown Rule"),
    }
}

fn list_item_to_cells(state: &mut State, ListItem(blocks): ListItem) -> Vec<Expr> {
    let mut cells = vec![];

    for block in blocks {
        match block {
            Block::Paragraph(text) => {
                let style = match state.list_depth {
                    0 => panic!(),
                    1 => "Item",
                    2 => "Subitem",
                    3 => "Subsubitem",
                    _ => todo!("return list depth error"),
                };

                cells.push(Expr::normal(
                    Symbol::new("System`Cell"),
                    vec![text_to_text_data(text), Expr::from(style)],
                ));
            },
            Block::List(items) => {
                let mut list_cells = Vec::new();

                state.list_depth += 1;

                for item in items {
                    list_cells.extend(list_item_to_cells(state, item));
                }

                state.list_depth -= 1;

                cells.extend(list_cells);
            },
            Block::Heading(_, _) => todo!("handle markdown headings inside list items"),
            Block::CodeBlock(_, _) => {
                todo!("handle markdown code block inside list item")
            },
            Block::Rule => todo!("handle markdown rule inside list item"),
        }
    }

    cells
}

/// Returns a `TextData[{...}]` expression.
fn text_to_text_data(text: Vec<TextSpan>) -> Expr {
    Expr::normal(Symbol::new("System`TextData"), vec![text_to_boxes(text)])
}

// Returns a `RowBox[{...}]` expression.
fn text_to_boxes(text: Vec<TextSpan>) -> Expr {
    let mut row = Vec::new();

    for span in text {
        match span {
            TextSpan::Text(text, styles) => {
                let mut style_rules: Vec<Expr> = Vec::new();

                for style in styles {
                    let (lhs, rhs) = match style {
                        TextStyle::Emphasis => {
                            (Symbol::new("System`FontSlant"), "Italic")
                        },
                        TextStyle::Strong => (Symbol::new("System`FontWeight"), "Bold"),
                        TextStyle::Strikethrough => todo!("strikethrough text"),
                    };

                    style_rules.push(Expr::normal(
                        Symbol::new("System`Rule"),
                        vec![Expr::from(lhs), Expr::string(rhs)],
                    ));
                }

                let expr = if style_rules.is_empty() {
                    Expr::string(text)
                } else {
                    style_rules.insert(0, Expr::string(text));

                    Expr::normal(Symbol::new("System`StyleBox"), style_rules)
                };

                row.push(expr);
            },
            TextSpan::Code(code) => row.push(Expr::normal(
                Symbol::new("System`StyleBox"),
                vec![Expr::string(code), Expr::string("Code")],
            )),
            TextSpan::Link { label, destination } => row.push(Expr::normal(
                Symbol::new("System`ButtonBox"),
                vec![
                    text_to_boxes(label),
                    Expr::normal(
                        Symbol::new("System`Rule"),
                        vec![
                            Expr::from(Symbol::new("System`BaseStyle")),
                            Expr::string("Hyperlink"),
                        ],
                    ),
                    Expr::normal(
                        Symbol::new("System`Rule"),
                        vec![
                            Expr::from(Symbol::new("System`ButtonData")),
                            Expr::normal(
                                Symbol::new("System`List"),
                                vec![
                                    Expr::normal(
                                        Symbol::new("System`URL"),
                                        vec![Expr::string(destination.clone())],
                                    ),
                                    Expr::from(Symbol::new("System`None")),
                                ],
                            ),
                        ],
                    ),
                    Expr::normal(
                        Symbol::new("System`Rule"),
                        vec![
                            Expr::from(Symbol::new("System`ButtonNote")),
                            Expr::string(destination),
                        ],
                    ),
                ],
            )),
            TextSpan::SoftBreak => row.push(Expr::string(" ")),
            TextSpan::HardBreak => todo!("handle {span:?}"),
        }
    }

    Expr::normal(
        Symbol::new("System`RowBox"),
        vec![Expr::normal(Symbol::new("System`List"), row)],
    )
}
