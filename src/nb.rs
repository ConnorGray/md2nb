use pulldown_cmark::HeadingLevel;

use wolfram_expr::{Expr, Symbol};

use crate::ast::*;

pub fn node_to_cells(node: Node) -> Vec<Expr> {
    match node {
        Node::Heading(level, text) => {
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
        Node::Paragraph(text) => vec![Expr::normal(
            Symbol::new("System`Cell"),
            vec![text_to_text_data(text), Expr::from("Text")],
        )],
        Node::List(items) => {
            let mut list_cells = Vec::new();

            for item in items {
                list_cells.extend(list_item_to_cells(item));
            }

            list_cells
        },
        Node::CodeBlock(_, code_text) => {
            let code_text = code_text.unwrap();

            vec![Expr::normal(
                Symbol::new("System`Cell"),
                vec![Expr::string(code_text), Expr::string("Program")],
            )]
        },
    }
}

fn list_item_to_cells(ListItem(mut nodes): ListItem) -> Vec<Expr> {
    if nodes.len() != 1 {
        todo!("handle list items with more than one node");
    }

    let node = nodes.pop().unwrap();

    match node {
        Node::Paragraph(text) => {
            vec![Expr::normal(
                Symbol::new("System`Cell"),
                vec![text_to_text_data(text), Expr::from("Item")],
            )]
        },
        Node::List(_) => todo!("handle nested markdown lists"),
        Node::Heading(_, _) => todo!("handle markdown headings inside list items"),
        Node::CodeBlock(_, _) => todo!("handle markdown code block inside list item"),
    }
}

/// Returns a `TextData[{...}]` expression.
fn text_to_text_data(text: Vec<TextNode>) -> Expr {
    Expr::normal(Symbol::new("System`TextData"), vec![text_to_boxes(text)])
}

// Returns a `RowBox[{...}]` expression.
fn text_to_boxes(text: Vec<TextNode>) -> Expr {
    let mut row = Vec::new();

    for text in text {
        match text {
            TextNode::Text(text, styles) => {
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
            _ => todo!(),
        }
    }

    Expr::normal(
        Symbol::new("System`RowBox"),
        vec![Expr::normal(Symbol::new("System`List"), row)],
    )
}
