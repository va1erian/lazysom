# lazysom

A simple SOM (Simple Object Machine) interpreter written in Rust.

## Architecture

The project is structured as a classic interpreter:

- **Lexer (`src/lexer.rs`)**: Tokenizes SOM source code using the `logos` library.
- **Parser (`src/parser.rs`)**: A hand-written recursive descent parser that produces an Abstract Syntax Tree (AST).
- **AST (`src/ast.rs`)**: Defines the structure of SOM classes, methods, and expressions.
- **Objects (`src/object.rs`)**: Implements the SOM object model, including:
    - `Value`: An enum representing all SOM types (Nil, Boolean, Integer, Double, String, Symbol, Array, Object, Block, Method, Primitive).
    - `SomObject`: A generic object with a class and fields.
    - Memory management using `Rc<RefCell<...>>` for shared, mutable references.
- **Universe (`src/universe.rs`)**: Manages the global state, including:
    - Global variables and class loading.
    - Classpath management for finding `.som` files.
    - Bootstrapping core classes like `Object`, `Class`, `Metaclass`, and `System`.
- **Interpreter (`src/interpreter.rs`)**: Executes the AST nodes. It handles:
    - Method dispatch (lookup and execution).
    - Block activations and closures.
    - Non-local returns and restarts.
- **Primitives (`src/primitives.rs`)**: Implements built-in methods for core classes (e.g., arithmetic, array manipulation, system I/O).
- **Threading**: The interpreter runs in a separate thread with an increased stack size (128 MB) to accommodate deep recursions typical in SOM programs.

## Usage

### Prerequisites

- Rust (latest stable)
- SOM standard library (included as a submodule in `SOM/`)

### Building

```bash
cargo build --release
```

### Running a SOM program

To run a SOM program, provide the path to the main `.som` file:

```bash
cargo run -- Examples/Hello.som
```

### REPL

Running `lazysom` without arguments starts a minimal REPL:

```bash
cargo run
```

## Testing

`lazysom` is tested against the standard SOM test suite located in `SOM/TestSuite`.

To run the full test suite:

```bash
cargo run -- SOM/TestSuite/TestHarness.som
```

To run a specific test suite:

```bash
cargo run -- SOM/TestSuite/TestHarness.som Array
```
