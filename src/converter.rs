use chumsky::{select, Parser};
use itertools::Itertools as _;
use regex::Regex;
use rust_norg::{LinkTarget, NorgASTFlat, ParagraphSegment};

fn format_link_target(input: LinkTarget) -> String {
    match input {
        LinkTarget::Heading { level, title } => {
            format!("{} {}", "*".repeat(level.into()), format_paragraph(title))
        }
        LinkTarget::Footnote(title) => format!("^ {}", format_paragraph(title)),
        LinkTarget::Definition(title) => format!("$ {}", format_paragraph(title)),
        LinkTarget::Generic(title) => format!("# {}", format_paragraph(title)),
        LinkTarget::Wiki(title) => format!("? {}", format_paragraph(title)),
        LinkTarget::Extendable(title) => format!("= {}", format_paragraph(title)),
        LinkTarget::Path(path) => format!("/ {path}"),
        LinkTarget::Url(url) => url,
        LinkTarget::Timestamp(timestamp) => format!("@ {timestamp}"),
    }
}

fn format_link(
    filepath: Option<String>,
    targets: Vec<LinkTarget>,
    description: Option<Vec<ParagraphSegment>>,
) -> String {
    let filepath = filepath.unwrap_or_default();
    let targets = targets.into_iter().map(format_link_target).join(" : ");

    if let Some(description) = description.map(format_paragraph) {
        format!("{{{filepath}{targets}}}[{description}]")
    } else {
        format!("{{{filepath}{targets}}}")
    }
}

fn format_paragraph_segment(input: ParagraphSegment) -> String {
    use ParagraphSegment::*;

    match input {
        Token(token) => token.to_string(),
        //AttachedModifierCandidate { modifier_type, content, closer } => todo!(),
        AttachedModifier {
            modifier_type,
            content,
        } => format!(
            "{modifier_type}{}{modifier_type}",
            format_paragraph(content)
        ),
        Link {
            filepath,
            targets,
            description,
        } => format_link(filepath, targets, description),
        AnchorDefinition { content, target } => {
            let content = format_paragraph(content);

            match *target {
                Link {
                    filepath,
                    targets,
                    description,
                } => {
                    let link = format_link(filepath, targets, description);

                    format!("[{content}]{link}")
                }
                _ => unreachable!(),
            }
        }
        Anchor {
            content,
            description,
        } => {
            let content = format_paragraph(content);

            if let Some(description) = description.map(format_paragraph) {
                format!("[{content}][{description}]")
            } else {
                format!("[{content}]")
            }
        }
        InlineLinkTarget(content) => format!("<{}>", format_paragraph(content)),
        _ => unreachable!(),
    }
}

fn reflow_paragraph(input: Vec<String>) -> String {
    let whitespace_regex = Regex::new(r"\s+").unwrap();
    let mergables = ["{", "[", "<"];

    whitespace_regex
        .split(&input.join(""))
        .map_into()
        .coalesce(|first: String, second: String| {
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
            if current_line.len() + new_len < 80 {
                current_line.push_str(&(" ".to_string() + &word));
            } else {
                *current_line = current_line.trim().to_string();
                lines.push(word.to_string());
            }

            lines
        })
        .join("\n")
        .trim()
        .to_string()
}

fn format_paragraph(input: Vec<ParagraphSegment>) -> String {
    reflow_paragraph(input.into_iter().map(format_paragraph_segment).collect())
}

pub fn format() -> impl Parser<NorgASTFlat, Vec<String>, Error = chumsky::error::Simple<NorgASTFlat>>
{
    use NorgASTFlat::*;

    let formatter = select! {
        // TODO: Format attached modifier extensions.
        // TODO: Find way to appropriately propagate error messages.
        Heading { level, title, extensions: _ } => {
            format!("{} {}\n", "*".repeat(level.into()), title.into_iter().map_into::<String>().collect::<String>())
        },
        NestableDetachedModifier { modifier_type, level, content, extensions: _ } => {
            let content = format().parse(vec![*content]).unwrap().join("").replace("\n", &format!("\n{}", " ".repeat(level as usize + 1)));

            format!("{} {content}", modifier_type.to_string().repeat(level.into()))
        },
        RangeableDetachedModifier { modifier_type, title, content, extensions: _ } => {
            let is_single_line = content.len() == 1 && matches!(content[0], Paragraph(_));

            if is_single_line {
                format!("{modifier_type} {}\n{}", title.into_iter().map_into::<String>().collect::<String>(), format().parse(content).unwrap().join(""))
            } else {
                format!("{modifier_type}{modifier_type} {}\n{}\n$$\n", title.into_iter().map_into::<String>().collect::<String>(), format().parse(content).unwrap().join(""))
            }
        },
        CarryoverTag { tag_type, name, parameters, next_object } =>  {
            let tag_type = match tag_type {
                rust_norg::CarryoverTag::Attribute => "+",
                rust_norg::CarryoverTag::Macro => "#",
            };
            let name = name.join(".");
            let parameters = parameters.join(" ");
            let next_object = format().parse(vec![*next_object]).unwrap().join("");

            format!("{tag_type}{name} {parameters}\n{next_object}")
        },
        InfirmTag { name, parameters } => {
            let name = name.join(".");
            let parameters = parameters.join(" ");

            format!(".{name} {parameters}")
        },
        VerbatimRangedTag { name, parameters, content } => {
            let name = name.join(".");
            let parameters = parameters.join(" ");

            // TODO: Make `content` respect indentation
            format!("@{name} {parameters}\n{content}@end\n")
        },
        RangedTag { name, parameters, content } => {
            let name = name.join(".");
            let parameters = parameters.join(" ");
            let content = format().parse(content).unwrap().join("");

            format!("|{name} {parameters}\n{content}|end\n")
        },
        Paragraph(content) => format_paragraph(content) + "\n",
    };

    formatter.repeated().at_least(1)
}
