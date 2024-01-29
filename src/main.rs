use clap::Parser as ClapParser;
use eyre::{eyre, Result};
use regex::{Captures, Regex};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

#[derive(ClapParser)]
struct NorgFmt {
    /// The path of the file to format.
    file: PathBuf,

    #[arg(long)]
    verify: bool,
}

fn rest(children: &Vec<NorgNode>, from: Option<usize>, to: Option<usize>) -> String {
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

// Possible transformations:
// - Regular decay: `*|hello|*` -> `*hello*`
// - Conversion with escape sequences: `*hello\* world*` -> `*|hello* world|*`
// - Keep regular escape sequences in check: `*\hello*` -> `*\hello*`
// - Mixed: `*\hello\* world*` -> `*|hello* world|*`
pub fn parse_markup(_: &Node, children: Vec<NorgNode>, _: &String) -> Result<String> {
    let should_make_free_form = children.iter().any(|val| {
        val.kind == "escape_sequence"
            && val
                .content
                .chars()
                .nth_back(0)
                .unwrap()
                .is_ascii_punctuation()
    });

    let is_free_form = children
        .get(1)
        .ok_or(eyre!("malformed attached modifier input"))?
        .kind
        == "free_form_open";

    let char = children
        .get(0)
        .ok_or(eyre!("markup has no opening modifier"))?
        .content
        .clone();

    // TODO: Don't recompile this regex every time, please
    let regex = Regex::new(r"\\([[:punct:]])")?;

    if should_make_free_form && !is_free_form {
        Ok(char.to_owned()
            + "|"
            + &regex.replace_all(
                &rest(&children, Some(1), Some(children.len() - 1)),
                |cap: &Captures| cap[1].to_string(),
            )
            + "|"
            + &char)
    } else if !should_make_free_form && is_free_form {
        Ok(char.to_owned() + &rest(&children, Some(2), Some(children.len() - 2)) + &char)
    } else {
        Ok(char.to_owned() + &rest(&children, Some(1), None))
    }
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
            parse_markup(node, children, source)?
        }
        "escape_sequence" => parse_escape_sequence(node, children, source)?,
        _ if node.child_count() == 0 => node.utf8_text(source.as_bytes())?.to_string(),
        _ => rest(&children, None, None),
    };

    Ok(ret)
}

fn parse_escape_sequence(node: &Node<'_>, _: Vec<NorgNode>, source: &String) -> Result<String> {
    let escaped_char = node
        .utf8_text(source.as_bytes())?
        .chars()
        .nth_back(0)
        .unwrap();

    if escaped_char.is_ascii_punctuation() {
        Ok(node.utf8_text(source.as_bytes())?.to_string())
    } else {
        Ok(escaped_char.to_string())
    }
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
