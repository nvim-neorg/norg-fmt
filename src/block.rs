use crate::{rest, Config, NorgNode};
use eyre::{eyre, Result};
use regex::Regex;
use tree_sitter::Node;

pub fn heading(
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

pub fn stars(node: &Node, _: Vec<NorgNode>, source: &String) -> Result<String> {
    Ok(node.utf8_text(source.as_bytes())?.trim().to_string())
}

pub fn title(_: &Node, children: Vec<NorgNode>, _: &str) -> Result<String> {
    let regex = Regex::new(r"[ \t\v]+")?;

    Ok(regex
        .replace_all(&rest(&children, None, None), " ")
        .to_string())
}

pub fn nestable_modifier(_node: &Node, children: Vec<NorgNode>, _source: &str) -> Result<String> {
    let prefix = &children
        .get(0)
        .ok_or(eyre!("nestable modifier has no prefix"))?
        .content
        .trim();

    let rest = rest(&children, Some(1), None);
    let mut split = rest.split_inclusive(['\n', '\r']);
    let first = split
        .next()
        .ok_or(eyre!("no content within nestable modifier"))?;
    let to_indent = split
        .map(|str| {
            if str.trim().is_empty() {
                str.into()
            } else {
                " ".repeat(prefix.len() + 1) + str
            }
        })
        .collect::<String>();

    Ok(prefix.to_string() + " " + first + &to_indent)
}

pub fn rangeable_modifier(_node: &Node, children: Vec<NorgNode>, _source: &str) -> Result<String> {
    let prefix = &children
        .get(0)
        .ok_or(eyre!("range-able detached modifier has no opening char"))?
        .content;
    let title = &children
        .get(1)
        .ok_or(eyre!("range-able detached modifier has no title"))?
        .content;

    let last_node = children
        .last()
        .ok_or(eyre!("range-able detached modifier has no opening char"))?;

    let has_closing_modifier = matches!(last_node.kind.as_str(), "^^" | "::" | "$$");

    let content = rest(
        &children,
        Some(2),
        if has_closing_modifier {
            Some(children.len() - 1)
        } else {
            None
        },
    )
    .split_inclusive(['\n', '\r'])
    .map(|str| " ".repeat(prefix.len() + 1) + str)
    .collect::<String>();

    if has_closing_modifier {
        Ok(prefix.to_owned() + " " + title.trim() + &content + &last_node.content)
    } else {
        Ok(prefix.to_owned() + " " + title.trim() + &content)
    }
}

pub fn ranged_tag(_node: &Node, children: Vec<NorgNode>, _source: &str) -> Result<String> {
    let head = rest(&children, None, Some(2));

    let parameters = children
        .iter()
        .skip(2)
        .take_while(|node| node.kind == "identifier")
        .map(|node| node.content.clone())
        .collect::<Vec<String>>();
    let content = rest(
        &children,
        Some(2 + parameters.len()),
        Some(children.len() - 1),
    );
    let end = &children.last().unwrap().content;

    Ok((head + " " + &parameters.join(" ")).trim_end().to_string() + "\n" + &content + end)
}

pub fn carryover_tag(_node: &Node, children: Vec<NorgNode>, _source: &str) -> Result<String> {
    let head = rest(&children, None, Some(2));
    let parameters = children
        .into_iter()
        .skip(2)
        .map(|node| node.content)
        .collect::<Vec<String>>()
        .join(" ");

    Ok((head + " " + &parameters).trim_end().to_string() + "\n")
}

#[cfg(test)]
mod tests {
    use tree_sitter::{Parser, Tree};

    use super::*;

    fn convert_to_tree(input: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_norg::language()).unwrap();

        parser.parse(input, None).unwrap()
    }

    #[test]
    fn headings() {
        let sources = vec![
            "* Heading",
            "  * Heading ",
            "*     Heading",
            "*  Heading with    several words",
            "* A heading with an obnoxiously long title that will definitely overflow the 80 character limit for a line."
        ];
        let results = vec![
            "* Heading",
            "* Heading",
            "* Heading",
            "* Heading with several words",
            "* A heading with an obnoxiously long title that will definitely overflow the 80 character limit for a line.",
        ];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            let parsed = crate::parse(&root, &source.to_string(), &Config::default()).unwrap();

            assert_eq!(parsed.trim(), result);
        }
    }

    #[test]
    fn nested_headings() {
        let sources = vec![
            "* Heading\nsome text below",
            "  * Heading\n   *** Another heading\n some text below the heading\n* A third heading\n    and some text below.",
        ];
        let results = vec![
            "* Heading\n  some text below",
            "* Heading\n*** Another heading\n    some text below the heading\n* A third heading\n  and some text below.",
        ];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            let parsed = crate::parse(&root, &source.to_string(), &Config::default()).unwrap();

            assert_eq!(parsed.trim(), result);
        }
    }

    #[test]
    fn nestable_modifiers() {
        let sources = vec![
            "- Text",
            "-  A    large amount of text that will surely surpass the eighty character limit if we try hard enough.",
            "- Text \n - Text",
            "- Text\n\n- A different list",
            "- A super duper large amount of text that will not only surely surpass the eighty character limit, but one that will extend beyond and span the distance of two lines instead.",

            "~ Text",
            "~  A    large amount of text that will surely surpass the eighty character limit if we try hard enough.",
            "~ Text \n - Text",
            "~ Text\n\n- A different list",
            "~ A super duper large amount of text that will not only surely surpass the eighty character limit, but one that will extend beyond and span the distance of two lines instead.",

            "> Text",
            ">  A    large amount of text that will surely surpass the eighty character limit if we try hard enough.",
            "> Text \n - Text",
            "> Text\n\n- A different list",
            "> A super duper large amount of text that will not only surely surpass the eighty character limit, but one that will extend beyond and span the distance of two lines instead.",
        ];
        let results = vec![
            "- Text",
            "- A large amount of text that will surely surpass the eighty character limit if\n  we try hard enough.",
            "- Text\n- Text",
            "- Text\n\n- A different list",
            "- A super duper large amount of text that will not only surely surpass the eighty\n  character limit, but one that will extend beyond and span the distance of two\n  lines instead.",

            "~ Text",
            "~ A large amount of text that will surely surpass the eighty character limit if\n  we try hard enough.",
            "~ Text\n- Text",
            "~ Text\n\n- A different list",
            "~ A super duper large amount of text that will not only surely surpass the eighty\n  character limit, but one that will extend beyond and span the distance of two\n  lines instead.",

            "> Text",
            "> A large amount of text that will surely surpass the eighty character limit if\n  we try hard enough.",
            "> Text\n- Text",
            "> Text\n\n- A different list",
            "> A super duper large amount of text that will not only surely surpass the eighty\n  character limit, but one that will extend beyond and span the distance of two\n  lines instead.",
        ];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            let parsed = crate::parse(&root, &source.to_string(), &Config::default()).unwrap();

            assert_eq!(parsed.trim(), result);
        }
    }
}
