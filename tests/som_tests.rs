use std::path::PathBuf;
use lazysom::run_som;

#[test]
fn test_recursion() {
    let classpath_extra = vec![PathBuf::from("tests")];
    // Run RecursionTest directly
    let result = run_som(classpath_extra, "tests/RecursionTest.som", vec!["RecursionTest".to_string()]);
    assert!(result.is_ok(), "RecursionTest failed: {:?}", result.err());
}
