use std::borrow::Cow;

use rustyline::config::Configurer;
use rustyline::highlight::Highlighter;
use rustyline::{ColorMode, Editor};

use rustyline_derive::{Completer, Helper, Hinter, Validator};
use secrecy::{Secret, SecretString};

use crate::Result;

#[derive(Completer, Helper, Hinter, Validator)]
struct MaskingHighlighter {
    masking: bool,
}

impl Highlighter for MaskingHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        use unicode_width::UnicodeWidthStr;
        if self.masking {
            Cow::Owned("*".repeat(line.width()))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        self.masking
    }
}

/// Read a passphrase from stdin prompt.
pub fn read_password(prompt: Option<&str>) -> Result<SecretString> {
    let h = MaskingHighlighter { masking: true };
    let mut rl = Editor::new()?;
    rl.set_helper(Some(h));
    rl.set_color_mode(ColorMode::Forced);
    rl.set_auto_add_history(false);

    // NOTE: trim any trailing newline is a quick hack
    // NOTE: for pasting
    let passwd = rl
        .readline(prompt.unwrap_or("Password: "))?
        .trim_end_matches('\n')
        .to_string();

    Ok(Secret::new(passwd))
}
