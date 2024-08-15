use chumsky::Parser as _;
use clap::Parser as ClapParser;
use converter::format;
use eyre::Result;
use rust_norg::parse;
use std::path::PathBuf;

mod converter;

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

    let ast = parse(&content).unwrap();

    let (formatted_output, errors) = format().parse_recovery(ast);

    if let Some(formatted_output) = formatted_output {
        print!("{}", formatted_output.join(""));
    }

    Ok(())
}
