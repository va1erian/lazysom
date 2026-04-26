use std::fs;

fn main() {
    let mut content = fs::read_to_string("tests/integration_test.rs").unwrap();
    content = content.replace("        | arr serialized deserialized |\n", "        | arr serialized deserialized |\n        ");
    content = content.replace("        | arr1 arr2 serialized deserialized |\n", "        | arr1 arr2 serialized deserialized |\n        ");
    content = content.replace("        | arr str serialized deserialized |\n", "        | arr str serialized deserialized |\n        ");
    fs::write("tests/integration_test.rs", content).unwrap();
}
