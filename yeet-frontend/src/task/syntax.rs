use std::path::Path;

use syntect::{
    easy::HighlightLines,
    highlighting::Theme,
    parsing::{SyntaxReference, SyntaxSet},
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use tokio::fs::{self};

pub async fn highlight(syntaxes: &SyntaxSet, theme: &Theme, path: &Path) -> Option<String> {
    match fs::read_to_string(path).await {
        Ok(content) => {
            let syntax = resolve_syntax(syntaxes, &content, path).await;
            if let Some(syntax) = syntax {
                tracing::debug!("syntax: {:?}", syntax.name);

                let mut highlighter = HighlightLines::new(syntax, theme);
                let mut result = String::new();
                for line in LinesWithEndings::from(&content) {
                    let highlighted = match highlighter.highlight_line(line, &syntaxes) {
                        Ok(ranges) => &as_24_bit_terminal_escaped(&ranges[..], false),
                        Err(err) => {
                            tracing::error!("unable to highlight line: {:?}", err);
                            line
                        }
                    };
                    result.push_str(highlighted);
                }

                Some(result)
            } else {
                tracing::debug!("unable to resolve syntax for: {:?}", path);
                Some(content)
            }
        }
        Err(err) => {
            tracing::error!("reading file failed: {:?} {:?}", path, err);
            None
        }
    }
}

async fn resolve_syntax<'a>(
    syntaxes: &'a SyntaxSet,
    content: &str,
    path: &Path,
) -> Option<&'a SyntaxReference> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let syntax = syntaxes.find_syntax_by_extension(&name);
    if syntax.is_some() {
        return syntax;
    }

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy())
        .unwrap_or_default();
    let syntax = syntaxes.find_syntax_by_extension(&ext);
    if syntax.is_some() {
        return syntax;
    }

    syntaxes.find_syntax_by_first_line(&content)
}