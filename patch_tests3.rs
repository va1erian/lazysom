use std::fs;

fn main() {
    let mut content = fs::read_to_string("tests/integration_test.rs").unwrap();
    // evaluate_snippet calls parse_expression. Variable declarations `| ... |` are not expressions, they belong in blocks `[ |...| ... ]` or method bodies.
    // We can wrap the code in a block and then call `value` on it, e.g. `[ |arr| ... ] value`.
    content = content.replace("    let snippet_code = \"| arr serialized deserialized |\n        ", "    let snippet_code = \"[ | arr serialized deserialized |\n        ");
    content = content.replace("    let snippet_code = \"| arr1 arr2 serialized deserialized |\n        ", "    let snippet_code = \"[ | arr1 arr2 serialized deserialized |\n        ");
    content = content.replace("    let snippet_code = \"| arr str serialized deserialized |\n        ", "    let snippet_code = \"[ | arr str serialized deserialized |\n        ");
    content = content.replace("        deserialized\n    \";", "        deserialized ] value\n    \";");
    fs::write("tests/integration_test.rs", content).unwrap();
}
