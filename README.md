# (WIP) Opinionated Formatter for Norg Files

This project serves as a proof of concept formatter for Norg files using the latest V3 treesitter parsing engine.

The formatting style is currently very opinionated and was devised by the creators of Norg (hey that's me!).
In the future we hope to provide a plethora of flags to tune the document to your style.

> [!NOTE]
> Not all of the syntax is supported by the formatter due to temporary limitations in the V3 parser.
> The capabilities of this project will be extended as the parser nears full completion.

# Current Capabilities

- Formatting of headings and proper indentation of children
- Consistent formatting of links and anchors
- Removal of extraneous escape sequences
- Automatic conversion of markup to free-form markup and vice versa if there
  are escape characters (e.g. `$Hello \\LaTeX!$` => `$|Hello \LaTeX!|$`)
- Smart formatting of paragraphs to a specific line length while preserving proper link structures.

# Usage

```sh
norg-fmt <file> <options>
```

Currently `norg-fmt` is capable of formatting one file at a time.

Available options may be viewed by running `norg-fmt --help`. The formatter will print to stdout, so feel
free to pipe the output anywhere you might need.
