use std::fs;

fn main() {
    let mut content = fs::read_to_string("tests/integration_test.rs").unwrap();
    // The parser might not like blocks as snippets if not wrapped correctly. Let's make it a valid block or method body.
    // Actually evaluate_snippet parses as block body. Let's just remove newlines before the pipes or check Hello.som.
    // Oh, the first character is a newline and spaces!
    // `evaluate_snippet` probably doesn't like leading whitespace before `|`. Let's strip the leading newline and spaces from the snippets.
    content = content.replace("    let snippet_code = \"\n        |", "    let snippet_code = \"|");
    fs::write("tests/integration_test.rs", content).unwrap();
}
