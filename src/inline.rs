use crate::{rest, Config, NorgNode};
use eyre::{eyre, Result};
use itertools::Itertools;
use regex::{Captures, Regex};
use tree_sitter::Node;

// Possible transformations:
// - Regular decay: `*|hello|*` -> `*hello*`
// - Conversion with escape sequences: `*hello\* world*` -> `*|hello* world|*`
// - Keep regular escape sequences in check: `*\hello*` -> `*\hello*`
// - Mixed: `*\hello\* world*` -> `*|hello* world|*`
pub fn markup(_: &Node, children: Vec<NorgNode>, _: &String) -> Result<String> {
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

pub fn link_scope(_node: &Node, children: Vec<NorgNode>, _source: &str) -> Result<String> {
    // NOTE(vhyrro): Please there has to be a better way of doing this.
    let regex = Regex::new(r"\s+")?;

    let output = format!(
        "{} {}",
        children
            .get(0)
            .ok_or(eyre!("no scope provided for link"))?
            .content,
        regex.replace_all(
            &children
                .get(1)
                .ok_or(eyre!("no title provided for link"))?
                .content,
            " "
        )
    );

    Ok(output.trim().to_string())
}

pub fn anchor(node: &Node, _children: Vec<NorgNode>, source: &String) -> Result<String> {
    // NOTE(vhyrro): Please there has to be a better way of doing this.
    let regex = Regex::new(r"\s+")?;

    Ok(regex
        .replace_all(node.utf8_text(source.as_bytes())?.trim(), " ")
        .to_string())
}

pub fn uri(node: &Node, _children: Vec<NorgNode>, source: &str) -> Result<String> {
    Ok(node
        .utf8_text(source.as_bytes())?
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect())
}

pub fn escape_sequence(node: &Node<'_>, _: Vec<NorgNode>, source: &String) -> Result<String> {
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

pub fn paragraph(
    _node: &Node,
    children: Vec<NorgNode>,
    _source: &str,
    config: &Config,
) -> Result<String> {
    let whitespace_regex = Regex::new(r"\s+")?;
    let mergables = ["{"];

    Ok(whitespace_regex
        .split(&rest(&children, None, None))
        .map(String::from)
        .coalesce(|first, second| {
            if mergables
                .iter()
                .any(|possibility| first.starts_with(possibility))
            {
                Ok(first.to_string() + " " + &second)
            } else {
                Err((first, second))
            }
        })
        .fold::<Vec<String>, _>(vec![String::default()], |mut lines, word| {
            let current_line = lines.last_mut().unwrap();
            let new_len = word.len();

            // This odd-looking less than operation is intentional, as we are also taking into
            // account the space that will be inserted.
            if current_line.len() + new_len < config.line_length {
                current_line.push_str(&(" ".to_string() + &word));
            } else {
                *current_line = current_line.trim().to_string();
                lines.push(word.to_string());
            }

            lines
        })
        .join("\n").trim().to_string())
}

#[cfg(test)]
mod tests {
    use crate::Config;
    use tree_sitter::{Parser, Tree};

    use super::*;

    fn convert_to_tree(input: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_norg::language()).unwrap();

        parser.parse(input, None).unwrap()
    }

    #[test]
    fn escape_sequences() {
        let sources = vec![r"\t", r"\ ", r"\\", r"\*", r"\/", "\\\n"];
        let results = vec!["t", " ", r"\\", r"\*", r"\/", "\n"];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            assert_eq!(
                escape_sequence(&root, Vec::default(), &source.to_string()).unwrap(),
                result
            );
        }
    }

    #[test]
    fn markup() {
        let sources = vec![
            "*|test|*",
            r"*hello\* world*",
            r"*\test*",
            r"*\hello\* world*",
        ];
        let results = vec!["*test*", "*|hello* world|*", "*test*", r"*|hello* world|*"];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            let parsed = crate::parse(&root, &source.to_string(), &Config::default()).unwrap();

            assert_eq!(parsed, result);
        }
    }

    #[test]
    fn links() {
        let sources = vec![
            "{*    long link}",
            r"{*  long link with    space }",
            r"[ unnecessary space ]",
            r"[ unnecessary     extra	space ]",
            r"{#   link with space}[ and a description]",
            r"{/ file .txt}[ and a description]",
            r"{@ Tue    5th May}[]",
            r"{https://there should be no space here.com}",
        ];
        let results = vec![
            r"{* long link}",
            r"{* long link with space}",
            r"[unnecessary space]",
            r"[unnecessary extra space]",
            r"{# link with space}[and a description]",
            r"{/ file .txt}[and a description]",
            r"{@ Tue 5th May}[]",
            r"{https://thereshouldbenospacehere.com}",
        ];

        for (source, result) in sources.into_iter().zip(results) {
            let tree = convert_to_tree(source);
            let root = tree.root_node();

            let parsed = crate::parse(&root, &source.to_string(), &Config::default()).unwrap();

            assert_eq!(parsed, result);
        }
    }
}
