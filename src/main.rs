use clap::Parser as ClapParser;
use eyre::{eyre, Result};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

#[derive(ClapParser)]
struct NorgFmt {
    /// The path of the file to format.
    file: PathBuf,
}

pub fn parse_heading(node: &Node, source: &mut String) -> Result<()> {
    let depth = node
        .named_child(0)
        .ok_or(eyre!("heading has no stars"))?
        .utf8_text(source.as_bytes())?;

    let title = node.named_child(1).ok_or(eyre!("heading has no title"))?;

    let result = format!(
        "{} {}",
        "*".repeat(depth.len()),
        title.utf8_text(source.as_bytes())?
    );

    source.replace_range(node.start_byte()..title.end_byte(), &result);

    Ok(())
}

fn main() -> Result<()> {
    let cli = NorgFmt::parse();

    let file = cli.file;
    let mut content = String::from_utf8(std::fs::read(file)?)?;

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_norg::language())?;

    let tree = parser.parse(&content, None).unwrap();

    let mut cursor = tree.walk();

    let mut node = cursor.node();

    while cursor.goto_next_sibling() || cursor.goto_first_child() {
        match node.kind() {
            "heading" => parse_heading(&node, &mut content),
            _ => {
                node = cursor.node();
                continue;
            }
        }?;

        node = cursor.node();
    }

    println!("{}", content);

    Ok(())
}
