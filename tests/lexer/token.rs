use cobol_compiler::lexer::{Keyword, TokenKind};

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
