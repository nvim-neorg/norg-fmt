// TODO: Add loads of tests

use clap::Parser as ClapParser;
use eyre::{eyre, Result};
use regex::Regex;
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

mod inline;

#[derive(ClapParser)]
struct NorgFmt {
    /// The path of the file to format.
    file: PathBuf,

    /// (todo) Verify the output of the AST after the formatting.
    #[arg(long)]
    verify: bool,

    /// If true will add an extra newline after a heading title to separate the content.
    #[arg(long)]
    newline_after_headings: bool,

    /// If true will not forcefully give heading titles zero indentation.
    #[arg(long)]
    indent_headings: bool,

    /// Determines the maximum length of a paragraph's line. Default: 80.
    #[arg(long)]
    line_length: Option<usize>,
}

pub fn rest(children: &Vec<NorgNode>, from: Option<usize>, to: Option<usize>) -> String {
    children
        .iter()
        .take(to.unwrap_or(children.len()))
        .skip(from.unwrap_or(0))
        .fold(String::default(), |str, val| str + &val.content)
}

pub fn parse_heading(
    _node: &Node,
    children: Vec<NorgNode>,
    _source: &str,
    config: &Config,
) -> Result<String> {
    let stars = children
        .get(0)
        .ok_or(eyre!("heading has no stars"))?
        .clone()
        .content;

    let heading_header = format!(
        "{} {}{}",
        stars,
        children
            .get(1)
            .ok_or(eyre!("heading has no title"))?
            .content,
        if config.newline_after_headings {
            "\n"
        } else {
            ""
        }
    );

    // TODO: Handle hard line breaks (`\\n`)
    let r = Regex::new(r"[\r\n]+")?;

    let children = children
        .into_iter()
        .map(|node| {
            if !config.indent_headings && node.kind == "heading" {
                node
            } else {
                let matches = r.find_iter(&node.content).collect::<Vec<_>>();

                NorgNode {
                    kind: node.kind,
                    content: r
                        .split(&node.content)
                        .enumerate()
                        .filter(|(_, str)| !str.is_empty())
                        .map(|(i, str)| (i, str.to_string()))
                        .map(|(i, str)| {
                            " ".repeat(stars.len() + 1)
                                + &str
                                + matches.get(i).map(|m| m.as_str()).unwrap_or("\n")
                        })
                        .collect::<String>(),
                }
            }
        })
        .collect();

    Ok(heading_header + &rest(&children, Some(2), None))
}

pub fn parse_stars(node: &Node, _: Vec<NorgNode>, source: &String) -> Result<String> {
    Ok(node.utf8_text(source.as_bytes())?.trim().to_string())
}

pub fn parse_title(node: &Node, _: Vec<NorgNode>, source: &String) -> Result<String> {
    Ok(node.utf8_text(source.as_bytes())?.trim().to_string())
}

pub fn parse_nestable_modifier(
    _node: &Node,
    children: Vec<NorgNode>,
    _source: &str,
) -> Result<String> {
    // Seriously find out how to remove all of these regexes please
    let regex = Regex::new(r"[\n\r]")?;

    let prefix = &children
        .get(0)
        .ok_or(eyre!("nestable modifier has no prefix"))?
        .content;

    let content = rest(&children, Some(1), None);

    let mut split = regex.split(&content);

    let first_line: String = split.nth(0).unwrap().to_string();

    // FIXME: This only creates a single newline afterwards which is really bad as it will connect
    // disjoint lists together.
    Ok(prefix.to_owned()
        + " "
        + &first_line
        + &split
            .map(|str| " ".repeat(prefix.len() + 1) + str)
            .collect::<String>())
}

#[derive(Debug, Clone)]
pub struct NorgNode {
    // field: Option<String>,
    pub kind: String,
    pub content: String,
}

pub fn parse(node: &Node, source: &String, config: &Config) -> Result<String> {
    let mut children = vec![];

    for child in node.children(&mut node.walk()) {
        let content = parse(&child, source, config)?;

        children.push(NorgNode {
            kind: child.kind().to_string(),
            content,
        })
    }

    Ok(match node.kind() {
        "heading" => parse_heading(node, children, source, config)?,
        "heading_stars" => parse_stars(node, children, source)?,
        "title" => parse_title(node, children, source)?,
        "unordered_list_item" | "ordered_list_item" | "quote_list_item" => {
            parse_nestable_modifier(node, children, source)?
        }
        "bold" | "italic" | "underline" | "strikethrough" | "spoiler" | "superscript"
        | "subscript" | "verbatim" | "inline_comment" | "math" | "inline_macro" => {
            inline::markup(node, children, source)?
        }
        "escape_sequence" => inline::escape_sequence(node, children, source)?,
        "uri" => inline::uri(node, children, source)?,
        "description" => inline::anchor(node, children, source)?,
        "paragraph" => inline::paragraph(node, children, source, config)?,
        kind if kind.starts_with("link_scope_") || kind.starts_with("link_target_") => {
            inline::link_scope(node, children, source)?
        }
        _ if node.child_count() == 0 => node.utf8_text(source.as_bytes())?.to_string(),
        _ => rest(&children, None, None),
    }
    .to_string())
}

pub struct Config {
    newline_after_headings: bool,
    indent_headings: bool,
    line_length: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            newline_after_headings: false,
            indent_headings: false,
            line_length: 80,
        }
    }
}

fn main() -> Result<()> {
    let cli = NorgFmt::parse();

    let config = Config {
        newline_after_headings: cli.newline_after_headings,
        indent_headings: cli.indent_headings,
        line_length: cli.line_length.unwrap_or(80),
    };

    let file = cli.file;
    let content = String::from_utf8(std::fs::read(file)?)?;

    let mut parser = Parser::new();
    parser.set_language(tree_sitter_norg::language())?;

    let tree = parser.parse(&content, None).unwrap();

    // println!("{}", tree.root_node().to_sexp());
    println!("{}", parse(&tree.root_node(), &content, &config)?.trim());

    if cli.verify {
        println!("AST verification is not implemented yet!");
    }

    Ok(())
}
