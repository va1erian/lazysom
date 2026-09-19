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
- **Primitives (`src/primitives/`)**: Implements built-in methods for core classes, split into modules: `core` (Object/Class/Block basics), `numbers`, `collections` (Array/String), `system` (I/O, classpath), `async_io` (async/network primitives), `gui`, `debugger`, and shared `helpers`.
- **GUI (`src/gui.rs`)**: A minimal Smalltalk-style IDE (Browser/Workspace/debugger) built on `eframe`/`egui`, launched with `--gui`.
- **Serialization (`src/serialize.rs`)**: Object graph serialization support (e.g. JSON/MessagePack) exposed to SOM code via primitives.
- **Background runner (`src/vm_runner.rs`)**: Runs IDE snippets on a background thread with pause/resume/step commands for the debugger.
- **Bytecode compiler/VM (`src/compiler.rs`, `src/bytecode.rs`, `src/bytecode_interpreter.rs`)**: An experimental AST-to-bytecode compiler and stack-based VM used by `--compile-image`/`--run-image`. See the note under Usage — it is currently broken and not the interpreter's normal execution path.
- **Threading**: The interpreter runs in a separate thread with an increased stack size (128 MB) to accommodate deep recursions typical in SOM programs.

## Usage

### Prerequisites

- Rust (latest stable)
- SOM standard library (included as a submodule in `SOM/`)

After cloning (or when working in a git worktree), initialize the submodule:

```bash
git submodule update --init
```

The `SOM/` submodule provides the SOM standard library and `TestSuite`. Without it, everything fails at startup with `Class Object not found in classpath`.

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

### GUI IDE

`lazysom` includes a minimal Smalltalk-style IDE (Browser and Workspace) built using `eframe` and `egui`.

To launch the IDE, use the `--gui` flag and include the `Tools` directory in the classpath:

```bash
cargo run --release -- --classpath Tools --gui LazyIde
```

### Bytecode image (experimental, currently broken)

`--compile-image <path>` and `--run-image <path>` compile a class to a bytecode `Image` and execute it on a separate bytecode VM (see `README_bytecode.md`). This path is experimental and currently broken: e.g. `(3 + 4) println` prints `nil`, `self`-sends fail, and blocks panic. It will be replaced per the roadmap (see below) — do not rely on it.

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

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) for the project's direction, including plans to replace the experimental bytecode VM.
