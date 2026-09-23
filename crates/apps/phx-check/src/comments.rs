/// One line of a comment, doc comments included, with the source line it sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub line: usize,
    pub text: String,
}

/// Extracts the comments of Rust source, which `syn` drops, skipping string and character literals so that
/// `"// x"` is not taken for one.
pub fn comments(source: &str) -> Vec<Comment> {
    let chars: Vec<char> = source.chars().collect();
    let mut lexer = Lexer { chars: &chars, at: 0, line: 1, found: Vec::new() };
    lexer.run();
    lexer.found
}

#[derive(Debug)]
struct Lexer<'a> {
    chars: &'a [char],
    at: usize,
    line: usize,
    found: Vec<Comment>,
}

impl Lexer<'_> {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek(0)?;
        self.at += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(0) {
            match c {
                '/' if self.peek(1) == Some('/') => self.line_comment(),
                '/' if self.peek(1) == Some('*') => self.block_comment(),
                '"' => self.string(),
                'r' if self.starts_raw_string() => self.raw_string(),
                '\'' => self.quote(),
                _ if is_ident(c) => self.ident(),
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn line_comment(&mut self) {
        self.at += 2;
        let mut text = String::new();
        while let Some(c) = self.peek(0) {
            if c == '\n' {
                break;
            }
            text.push(c);
            self.at += 1;
        }
        self.found.push(Comment { line: self.line, text });
    }

    fn block_comment(&mut self) {
        self.at += 2;
        let mut depth = 1_usize;
        let mut text = String::new();
        let mut line = self.line;
        while depth > 0 {
            match (self.peek(0), self.peek(1)) {
                (None, _) => break,
                (Some('/'), Some('*')) => {
                    depth += 1;
                    self.at += 2;
                    text.push_str("/*");
                }
                (Some('*'), Some('/')) => {
                    depth -= 1;
                    self.at += 2;
                    if depth > 0 {
                        text.push_str("*/");
                    }
                }
                (Some('\n'), _) => {
                    self.found.push(Comment { line, text: std::mem::take(&mut text) });
                    self.bump();
                    line = self.line;
                }
                (Some(c), _) => {
                    text.push(c);
                    self.at += 1;
                }
            }
        }
        self.found.push(Comment { line, text });
    }

    fn string(&mut self) {
        self.bump();
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '"' => return,
                _ => {}
            }
        }
    }

    /// An `r` opens a raw string only as a token's start, `r"` or `r#…"`; `br` reaches here through the `b` ident
    /// path, so identifiers are consumed whole to keep this true.
    fn starts_raw_string(&self) -> bool {
        let mut ahead = 1;
        while self.peek(ahead) == Some('#') {
            ahead += 1;
        }
        self.peek(ahead) == Some('"')
    }

    fn raw_string(&mut self) {
        self.bump();
        let mut hashes = 0_usize;
        while self.peek(0) == Some('#') {
            hashes += 1;
            self.bump();
        }
        self.bump();
        while let Some(c) = self.bump() {
            if c == '"' {
                let mut closing = 0_usize;
                while closing < hashes && self.peek(0) == Some('#') {
                    closing += 1;
                    self.bump();
                }
                if closing == hashes {
                    return;
                }
            }
        }
    }

    /// A quote opens a character literal (`'a'`, `'\n'`, `'\u{1F600}'`) or a lifetime (`'a`), which never closes.
    fn quote(&mut self) {
        self.bump();
        match (self.peek(0), self.peek(1)) {
            (Some('\\'), _) => {
                self.bump();
                self.bump();
                while let Some(c) = self.bump() {
                    if c == '\'' {
                        return;
                    }
                }
            }
            (Some(_), Some('\'')) => {
                self.bump();
                self.bump();
            }
            _ => {}
        }
    }

    /// Consumes an identifier, and the string after a `b`, `br` or `c` prefix, so a prefix is never mistaken for
    /// the start of a raw string.
    fn ident(&mut self) {
        let start = self.at;
        while self.peek(0).is_some_and(is_ident) {
            self.bump();
        }
        let word: String = self.chars.get(start..self.at).map(|w| w.iter().collect()).unwrap_or_default();
        let prefix = matches!(word.as_str(), "b" | "c" | "br" | "cr");
        if prefix && matches!(self.peek(0), Some('"' | '#')) {
            if word.ends_with('r') {
                self.at -= 1;
                self.raw_string();
            } else {
                self.string();
            }
        } else if word == "b" && self.peek(0) == Some('\'') {
            self.quote();
        }
    }
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::{Comment, comments};

    fn texts(source: &str) -> Vec<String> {
        comments(source).into_iter().map(|c| c.text).collect()
    }

    #[test]
    fn comment_lexer_skips_strings() {
        assert!(comments(r#"let s = "// REP.8";"#).is_empty());
        assert!(comments("let s = r#\"/* x */ // y\"#;").is_empty());
        assert!(comments(r#"let s = b"// x"; let c = '"'; let d = b'"';"#).is_empty());
        assert_eq!(texts(r"let q = '\''; let p = '\\'; // real"), vec![" real"]);
    }

    #[test]
    fn comment_lexer_finds_line_block_and_doc_comments() {
        let source = "/// doc\nfn f<'a>(x: &'a u8) -> char { '/' } // tail\n/* one\n two /* nested */ */";
        assert_eq!(texts(source), vec!["/ doc", " tail", " one", " two /* nested */ "]);
        assert_eq!(comments(source).get(3), Some(&Comment { line: 4, text: " two /* nested */ ".to_owned() }));
    }
}
