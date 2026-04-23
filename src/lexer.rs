use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r#""[^"]*""#)] // Comments
pub enum Token {
    #[token("primitive")]
    Primitive,

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*")]
    Identifier,

    #[token("=")]
    Equal,

    #[regex(r"----(-*)")]
    Separator,

    #[token("(")]
    NewTerm,

    #[token(")")]
    EndTerm,

    #[token("|")]
    Or,

    #[token(",")]
    Comma,

    #[token("-")]
    Minus,

    #[token("~")]
    Not,

    #[token("&")]
    And,

    #[token("*")]
    Star,

    #[token("/")]
    Div,

    #[token("\\")]
    Mod,

    #[token("+")]
    Plus,

    #[token(">")]
    More,

    #[token("<")]
    Less,

    #[token("@")]
    At,

    #[token("%")]
    Per,

    #[regex(r"[~&|*/\\+>=<@%!-]+", priority = 1)]
    OperatorSequence,

    #[token(":")]
    Colon,

    #[token("[")]
    NewBlock,

    #[token("]")]
    EndBlock,

    #[token("#")]
    Pound,

    #[token("^")]
    Exit,

    #[token(".")]
    Period,

    #[token(":=")]
    Assign,

    #[regex(r"[0-9]+")]
    Integer,

    #[regex(r"[0-9]+\.[0-9]+")]
    Double,

    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*:")]
    Keyword,

    #[regex(r"'", lex_string)]
    STString,
}

fn lex_string(lex: &mut logos::Lexer<Token>) -> logos::Filter<()> {
    let mut remainder = lex.remainder();
    let mut len = 0;
    let mut escaped = false;

    while let Some(c) = remainder.chars().next() {
        len += c.len_utf8();
        remainder = &remainder[c.len_utf8()..];
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '\'' {
            lex.bump(len);
            return logos::Filter::Emit(());
        }
    }
    logos::Filter::Skip
}
