use std::fmt;

use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Identification,
    Environment,
    Data,
    Procedure,
    Division,
    Section,
    Display,
    Accept,
    Perform,
    Stop,
    Run,
    If,
    Else,
    Move,
}

impl Keyword {
    pub const ALL: [Keyword; 14] = [
        Keyword::Identification,
        Keyword::Environment,
        Keyword::Data,
        Keyword::Procedure,
        Keyword::Division,
        Keyword::Section,
        Keyword::Display,
        Keyword::Accept,
        Keyword::Perform,
        Keyword::Stop,
        Keyword::Run,
        Keyword::If,
        Keyword::Else,
        Keyword::Move,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::Identification => "IDENTIFICATION",
            Keyword::Environment => "ENVIRONMENT",
            Keyword::Data => "DATA",
            Keyword::Procedure => "PROCEDURE",
            Keyword::Division => "DIVISION",
            Keyword::Section => "SECTION",
            Keyword::Display => "DISPLAY",
            Keyword::Accept => "ACCEPT",
            Keyword::Perform => "PERFORM",
            Keyword::Stop => "STOP",
            Keyword::Run => "RUN",
            Keyword::If => "IF",
            Keyword::Else => "ELSE",
            Keyword::Move => "MOVE",
        }
    }

    /// Case-insensitive lookup.
    pub fn from_word(word: &str) -> Option<Keyword> {
        Keyword::ALL
            .into_iter()
            .find(|k| k.as_str().eq_ignore_ascii_case(word))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    /// `PIC` or `PICTURE`; the picture string follows as `PictureString`.
    Picture,
    /// Uppercased user-defined word.
    Identifier(String),
    /// Unsigned number as written, e.g. `123` or `45.67`.
    NumericLiteral(String),
    /// Contents of a quoted literal with the quotes removed and doubled quotes collapsed.
    StringLiteral(String),
    /// Uppercased picture characters, e.g. `S9(3)V99`.
    PictureString(String),
    Period,
    Comma,
    Semicolon,
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
    Power,
    Eq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Keyword(k) => write!(f, "KEYWORD {}", k.as_str()),
            TokenKind::Picture => write!(f, "PICTURE"),
            TokenKind::Identifier(s) => write!(f, "IDENTIFIER {s}"),
            TokenKind::NumericLiteral(s) => write!(f, "NUMBER {s}"),
            TokenKind::StringLiteral(s) => write!(f, "STRING {s:?}"),
            TokenKind::PictureString(s) => write!(f, "PICTURE-STRING {s}"),
            TokenKind::Period => write!(f, "'.'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Semicolon => write!(f, "';'"),
            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::Slash => write!(f, "'/'"),
            TokenKind::Power => write!(f, "'**'"),
            TokenKind::Eq => write!(f, "'='"),
            TokenKind::Lt => write!(f, "'<'"),
            TokenKind::Gt => write!(f, "'>'"),
            TokenKind::LtEq => write!(f, "'<='"),
            TokenKind::GtEq => write!(f, "'>='"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// Location in the original file(s).
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup_is_case_insensitive_and_complete() {
        for k in Keyword::ALL {
            assert_eq!(Keyword::from_word(k.as_str()), Some(k));
            assert_eq!(Keyword::from_word(&k.as_str().to_lowercase()), Some(k));
        }
        assert_eq!(Keyword::from_word("Display"), Some(Keyword::Display));
        assert_eq!(Keyword::from_word("PROGRAM-ID"), None);
        assert_eq!(Keyword::from_word("DISPLAYS"), None);
    }

    #[test]
    fn display_formats() {
        assert_eq!(
            TokenKind::Keyword(Keyword::Move).to_string(),
            "KEYWORD MOVE"
        );
        assert_eq!(
            TokenKind::StringLiteral("a\"b".into()).to_string(),
            "STRING \"a\\\"b\""
        );
        assert_eq!(TokenKind::Power.to_string(), "'**'");
        assert_eq!(
            TokenKind::NumericLiteral("1.5".into()).to_string(),
            "NUMBER 1.5"
        );
    }
}
