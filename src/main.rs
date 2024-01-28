use clap::Parser as ClapParser;
use eyre::{eyre, Result};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

#[derive(ClapParser)]
struct NorgFmt {
    /// The path of the file to format.
    file: PathBuf,
}

fn rest(children: Vec<NorgNode>, from: Option<usize>) -> String {
    children
        .into_iter()
        .skip(from.unwrap_or(0))
        .fold(String::default(), |str, val| str + &val.content)
}

pub fn parse_heading(_node: &Node, children: Vec<NorgNode>, _: &String) -> Result<String> {
    let heading_header = format!(
        "{} {}",
        children
            .get(0)
            .ok_or(eyre!("heading has no stars"))?
            .content,
        children
            .get(1)
            .ok_or(eyre!("heading has no title"))?
            .content,
    );

    Ok(heading_header + &rest(children, Some(2)))
}

pub fn parse_stars(node: &Node, _: Vec<NorgNode>, source: &String) -> Result<String> {
    Ok(node.utf8_text(source.as_bytes())?.trim().to_string())
}

pub fn parse_title(node: &Node, _: Vec<NorgNode>, source: &String) -> Result<String> {
    Ok(node.utf8_text(source.as_bytes())?.trim().to_string())
}

pub struct NorgNode {
    // field: Option<String>,
    pub kind: String,
    pub content: String,
}

pub fn parse(node: &Node, source: &String) -> Result<String> {
    // println!("'{}'", node.kind());
    let mut children = vec![];

    for child in node.children(&mut node.walk()) {
        let content = parse(&child, source)?;

        children.push(NorgNode {
            kind: child.kind().to_string(),
            content,
        })
    }

    let ret = match node.kind() {
        "heading" => parse_heading(node, children, source)?,
        "heading_stars" => parse_stars(node, children, source)?,
        "title" => parse_title(node, children, source)?,
        "document" => rest(children, None),
        _ => node.utf8_text(source.as_bytes())?.to_string(),
    };

    Ok(ret)
}

fn main() -> Result<()> {
    let cli = NorgFmt::parse();

    let file = cli.file;
    let content = String::from_utf8(std::fs::read(file)?)?;

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_norg::language())?;

    let tree = parser.parse(&content, None).unwrap();

    println!("{}", tree.root_node().to_sexp());
    println!("{}", parse(&tree.root_node(), &content)?);

    Ok(())
}
