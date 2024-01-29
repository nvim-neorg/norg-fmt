use clap::Parser as ClapParser;
use eyre::{eyre, Result};
use regex::{Captures, Regex};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

mod inline;

#[derive(ClapParser)]
struct NorgFmt {
    /// The path of the file to format.
    file: PathBuf,

    #[arg(long)]
    verify: bool,
}

pub fn rest(children: &Vec<NorgNode>, from: Option<usize>, to: Option<usize>) -> String {
    children
        .iter()
        .take(to.unwrap_or(children.len()))
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

    Ok(heading_header + &rest(&children, Some(2), None))
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
        "bold" | "italic" | "underline" | "strikethrough" | "spoiler" | "superscript"
        | "subscript" | "verbatim" | "inline_comment" | "math" | "inline_macro" => {
            inline::markup(node, children, source)?
        }
        "escape_sequence" => inline::escape_sequence(node, children, source)?,
        _ if node.child_count() == 0 => node.utf8_text(source.as_bytes())?.to_string(),
        _ => rest(&children, None, None),
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

    if cli.verify {
        println!("AST verification is not implemented yet!");
    }

    Ok(())
}
