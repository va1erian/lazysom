use std::fs;

fn main() {
    let mut content = fs::read_to_string("SOM/Smalltalk/System.som").unwrap();
    content = content.replace("\n)\n\n    serialize: object format: formatString = primitive\n    deserialize: data format: formatString = primitive\n", "");
    content = content.replace("    totalCompilationTime = ( ^ 0 \"Estimated total compilation time in milliseconds\" )", "    totalCompilationTime = ( ^ 0 \"Estimated total compilation time in milliseconds\" )\n\n    serialize: object format: formatString = primitive\n    deserialize: data format: formatString = primitive\n");
    fs::write("SOM/Smalltalk/System.som", content).unwrap();
}
