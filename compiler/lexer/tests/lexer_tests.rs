use klik_lexer::{Lexer, TokenKind};

#[test]
fn lexes_identifiers_numbers_strings_keywords_ops_and_punctuation() {
    let source = r#"fn main() { let value = 42 return "ok" if else x + - * / = , ; ( ) { } }"#;
    let tokens = Lexer::new(source, "<test>").tokenize().expect("tokenize");

    assert!(matches!(tokens[0].kind, TokenKind::Fn));
    assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    assert!(matches!(tokens[2].kind, TokenKind::LeftParen));
    assert!(matches!(tokens[3].kind, TokenKind::RightParen));
    assert!(matches!(tokens[4].kind, TokenKind::LeftBrace));
    assert!(matches!(tokens[5].kind, TokenKind::Let));
    assert!(matches!(tokens[6].kind, TokenKind::Identifier(_)));
    assert!(matches!(tokens[7].kind, TokenKind::Eq));
    assert!(matches!(tokens[8].kind, TokenKind::IntLiteral(42)));
    assert!(matches!(tokens[9].kind, TokenKind::Return));
    assert!(matches!(&tokens[10].kind, TokenKind::StringLiteral(s) if s == "ok"));
    assert!(matches!(tokens[11].kind, TokenKind::If));
    assert!(matches!(tokens[12].kind, TokenKind::Else));
    assert!(matches!(tokens[13].kind, TokenKind::Identifier(_)));
    assert!(matches!(tokens[14].kind, TokenKind::Plus));
    assert!(matches!(tokens[15].kind, TokenKind::Minus));
    assert!(matches!(tokens[16].kind, TokenKind::Star));
    assert!(matches!(tokens[17].kind, TokenKind::Slash));
    assert!(matches!(tokens[18].kind, TokenKind::Eq));
    assert!(matches!(tokens[19].kind, TokenKind::Comma));
    assert!(matches!(tokens[20].kind, TokenKind::Semicolon));
    assert!(matches!(tokens[21].kind, TokenKind::LeftParen));
    assert!(matches!(tokens[22].kind, TokenKind::RightParen));
    assert!(matches!(tokens[23].kind, TokenKind::LeftBrace));
    assert!(matches!(tokens[24].kind, TokenKind::RightBrace));
}
