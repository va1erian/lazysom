This is a rust implementation of the SOM language in rust.
Always use the latest rust version.
Never use nightly rust features unless the user explicitly asks for it.
The SOM directory contains SOM standard classes, tests and examples. 
Never edit files in the SOM directory.
Ensure tests always pass before merging any changes.
Ensure there are no warnings after "cargo check"
check if you are in a windows environment before running any commands.

GUIDELINES FOR DEVELOPING IDE TOOLS & SOM CODE:
- **Conditionals syntax:** Always use `ifTrue:ifFalse:` for dual-branch conditionals in SOM. The selector `ifFalse:ifTrue:` is NOT supported by the standard boolean classes and will cause runtime errors.
- **Harness Verification:** Always run `cargo test` after modifying any IDE tools in the `Tools/` directory. The test `test_ide_classes_compilation` in `tests/integration_test.rs` automatically compiles these files to check for syntax/compilation issues.
