# LazySOM Studio — roadmap from a SOM interpreter to a Visual Basic–style IDE

Goal: a RAD environment in the spirit of VB6 / PureBasic where you pick a project type
(library, console, form app, web app), draw your UI or API, write SOM code with real
editor support, press F5 to debug, and press "Build" to get a small self-contained
`.exe` / ELF binary. The Smalltalk image/browser metaphor goes away: projects are
files on disk, the IDE is a normal desktop application.

---

## 0. Where we are today (baseline, verified 2026-09-19 on commit `13a8f17`)

### 0.1 What was actually run

Release build on Windows 11, Rust 1.98, with the `SOM` submodule contents present
(they are **not** checked out in fresh worktrees; without them 8 of the 9 integration
tests fail with `Class Object not found in classpath`, so CI needs `submodules: recursive`
as it already has, and local worktrees need `git submodule update --init`).

| Check | Result |
|---|---|
| `cargo build --release`, `cargo check` | ✅ builds, 0 warnings |
| `cargo test --release` | ✅ 21/21 (12 GC stress + 9 integration) |
| SOM TestSuite on AST interpreter | ✅ 221/221 after the block-scoping fix (`c5c7980`); before it, `HashTest>>testHashtable` failed because block activations resolved fields before the method's arguments |
| `tests/Test*.som`, `Async/AsyncIOTest` | ✅ run and print expected output |
| Bytecode VM (`--compile-image` / `--run-image`) | ❌ **non-functional**: `(3 + 4) println` prints `nil`; `self fib: 25` fails with "Method fib: not found in image for Integer" (wrong receiver on self-sends); `#(1 2 3) do: [...]` panics (`bytecode_interpreter.rs:459` index out of bounds); TestHarness image runs and prints nothing. Every image is ~240 KB regardless of program. |
| `3 fooBar` (doesNotUnderstand) | ❌ prints `ERROR: Method fooBar not found in class Integer` and then **keeps running with nil** (exit code 0). Cause: `System>>signalError:` is declared `primitive` in SOM but not implemented, and unimplemented primitives silently return `nil`. |
| `(Array new: 3) at: 10` | ❌ silently returns `nil` |
| `(Array new: 3) at: 10 put: 5` | ❌ Rust **panic** in `collections.rs:147` → with `panic = "abort"` in the release profile the process dies. |
| `1/0` | ⚠️ stops the program with `Error: Division by zero`, exit 1, no stack trace, no line |
| Recursion `self down: 900` | ❌ `Recursion limit exceeded in dispatch` — the 1000 limit counts *all* nested dispatches, so real method depth is only a few hundred |
| Speed, `fib: 25` (AST) | ~690 ms (≈ 250k method activations) — roughly 100× slower than a production Smalltalk; acceptable for an IDE prototype, not for "real apps" |
| `--gui LazyIde` | ✅ window starts and renders for 10 s with no errors, ~240 MB RSS. Interactive behaviour (browser, debugger stepping) **not verified** here — needs a manual pass. |

Declared-but-unimplemented primitives (all silently return `nil` today):
`System>>signalError:`, `System>>errorPrint:`, `System>>errorPrintln:`,
`System>>printStackTrace`, `Object>>inspect`, `Object>>instVarNamed:`,
`Object>>perform:withArguments:inSuperclass:`, `Method>>invokeOn:with:`,
`Primitive>>invokeOn:with:`, `Tools/File.som` `readText:` / `writeText:to:`.

### 0.2 Architecture facts that shape the plan

| Area | Current state | Problem for the goal |
|---|---|---|
| Engines | AST interpreter (`interpreter.rs`) is the only one that works. The bytecode VM (`compiler.rs` + `bytecode_interpreter.rs`) is a broken sketch (see above); its compiler also `panic!`s on nested array literals and unknown assignment targets. Primitive signature is hard-wired to the AST `Interpreter`. | Self-contained exes need a compiled engine, so the VM is effectively a **rewrite**, with the AST interpreter as the reference implementation. |
| Primitives | A method declared `= primitive` with no Rust implementation becomes a stub returning `nil` (`universe.rs`, `assemble_method`). Primitives index `Vec`s directly and can panic. | Silent wrong results and hard aborts. Missing primitives must fail loudly at load time; primitives must never panic. |
| Errors | `doesNotUnderstand:` → `halt` → SOM's `Object>>doesNotUnderstand:` → `error:` → unimplemented `system signalError:` → returns nil and continues. Rust-side errors are `anyhow` strings that unwind to `main`. No exceptions in the language. | Worst of both worlds: logic errors are swallowed, runtime errors kill the program without a trace. |
| Source positions | Parser reports byte offsets only; AST has no spans. | No line numbers in errors, no breakpoints by line, no editor diagnostics. |
| Parser | Stops at the first error. | Useless for an editor (needs error recovery for completion on half-typed code). |
| Recursion | Rust-stack recursive evaluation, hard limit of 1000 nested dispatches (a few hundred method levels). | Real programs hit this. |
| Values | Every integer is a `BigInt`; symbols are owned `String`s; activations are `HashMap`s. | Slow (see fib timing). |
| Name resolution | Flat global namespace, classes found by `ClassName.som` on a classpath. | No modules, no libraries, no name clash protection. |
| System class | ~20 IDE primitives injected into `System` by hand-written boilerplate in `universe.rs`. | Not scalable; mixes IDE plumbing with language runtime. |
| GUI | SOM code drives egui immediate mode every frame via raw pointers (`NativeHandle(usize)` cast to `&egui::Ui`). IDE itself is SOM (`Tools/LazyIde.som`, `Browser`, `Workspace`, `Debugger`, `HelpPanel`). | Unsafe, fragile, and the IDE is only as reliable as the interpreter it is debugging. |
| Runner | `vm_runner.rs`: global singleton, runs snippets on a background thread with a pause/step channel. | Good seed for a debugger, but in-process: a crash in user code crashes the IDE. |
| Async | tokio runtime + callback-based `AsyncFile` / `AsyncTcpSocket`. | Useful building block for the Net/Web library. |

Constraints to keep (from `AGENTS.md`): stable Rust only, never edit `SOM/`, the standard
SOM TestSuite must keep passing, `cargo check` must be warning-free. Therefore every
language extension must be a **strict superset** of SOM, and our own core additions live
outside `SOM/` (see class extensions, §2.4).

---

## 1. Architecture target

### 1.1 Key decisions

1. **One engine: a new bytecode VM.** Self-contained executables need a compiled image, so
   a bytecode VM becomes the only engine. The current one doesn't work (§0.1), so it is
   rewritten, not fixed: new instruction set with line tables, non-recursive (explicit frame
   stack, which removes the depth limit and makes pause/step/unwind trivial). The working
   AST interpreter stays as the **reference implementation** for differential testing
   until the VM passes everything, then it is deleted.
2. **The IDE is written in Rust (egui), not in SOM.** This is what "drop the Smalltalk-style
   UI" means in practice: no class browser, no workspace-as-image. The IDE can still be
   extended with SOM plugins later, but its core must never depend on the code under test.
3. **User programs run out of process.** F5 launches the project with the same runtime
   that a built exe uses, connected to the IDE over a debug protocol (JSON over a local
   socket / stdio, shaped like DAP). A user crash or infinite loop never takes the IDE down,
   and "works in IDE" == "works as exe".
4. **Executables = prebuilt runtime stub + appended image.** Building never needs a Rust
   toolchain on the user's machine and cross-building Windows↔Linux is a file copy.
5. **Files are the source of truth.** A project is a directory with a manifest; forms and
   APIs are declarative files plus code-behind classes. Everything diffs cleanly in git.

### 1.2 Crate layout (cargo workspace)

```
crates/
  som-syntax     lexer, error-tolerant parser, AST with spans, formatter
  som-compiler   bytecode compiler, linker, module resolver, image format (+ source maps)
  som-vm         VM, GC, exceptions, core primitives, debug hooks, primitive registry
  som-stdlib     native primitives for the standard library (feature-gated: io, net, db, ...)
  som-forms      retained widget tree + egui renderer for form apps
  som-web        HTTP server runtime (hyper/axum on the existing tokio dependency)
  som-runtime    the stub binaries: console / forms / web (feature combos)
  som-langserv   language services: index, completion, diagnostics, go-to-def, hover
  som-ide        the IDE (egui + egui_dock)
  lazysom        CLI: new / build / run / test / package / repl / lsp
lib/             SOM sources of the standard library (Core, Collections, IO, ...)
templates/       project templates (library, console, forms, web)
SOM/             untouched upstream submodule (TestSuite + Smalltalk classes)
```

`som-langserv` also exposes an LSP server (`lazysom lsp`) almost for free, so VS Code
users get highlighting/completion too.

### 1.3 Project model

`MyApp.somproj` (TOML):

```toml
[project]
name    = "InvoiceTool"
kind    = "forms"            # library | console | forms | web
version = "1.0.0"
main    = "App"              # class with class-side main: args  (console/forms/web)
icon    = "res/app.ico"

[dependencies]
Json    = { path = "../libs/Json" }
Sqlite  = { lib  = "Sqlite", version = "1.2" }         # from the local library store
Charts  = { git  = "https://github.com/…/charts", tag = "v0.3" }

[build]
targets = ["windows-x64", "linux-x64"]
console = false               # forms: hide console window on Windows
```

```
InvoiceTool/
  InvoiceTool.somproj
  src/App.som
  src/MainForm.form          # designer file (declarative)
  src/MainForm.som           # code-behind (event handlers)
  src/Invoice.som
  res/app.ico
  test/InvoiceTest.som
  build/                     # outputs, gitignored
```

---

## 2. Language: from SOM to an industrial-grade dialect

All additions are opt-in syntax or new classes; plain SOM files keep working unchanged.

### 2.1 Exceptions (highest priority)

Pharo-style, because it is proven and fits blocks naturally:

```smalltalk
[ file := File openRead: path. ^ file readAll ]
    on: FileNotFound do: [:e | ^ nil ]
    
[ conn send: data ] ensure: [ conn close ].

[ x foo ] on: MessageNotUnderstood do: [:e | e return: 0 ].
```

- Class hierarchy (in `lib/Core`): `Exception` → `Error` (`MessageNotUnderstood`,
  `ZeroDivide`, `IndexOutOfBounds`, `KeyNotFound`, `InvalidArgument`, `IOError` →
  `FileNotFound`…, `NetworkError`, `ParseError`), `Warning`, `Notification`.
- API: `signal`, `signal:`, `messageText`, `return:`, `retry`, `resume:` (resumable only),
  `pass`, `outer`, `signalerContext`, `stackTrace`; blocks gain `on:do:`, `ensure:`,
  `ifCurtailed:`; `Error signal: 'msg'` and `self error: 'msg'` both raise.
- VM: handler search walks frames marked by `on:do:` (no Rust unwinding; the VM owns its
  frame stack). `ensure:` blocks run during unwinding, including non-local returns.
- **Every** primitive failure becomes a typed SOM exception instead of `anyhow` text
  (`at:` out of range → `IndexOutOfBounds`, division by zero → `ZeroDivide`,
  bad primitive argument → `InvalidArgument`).
- `doesNotUnderstand:` signals a resumable `MessageNotUnderstood`; it no longer calls
  `system exit:`. `escapedBlock:` and `unknownGlobal:` become `BlockContextEscaped` /
  `UndefinedGlobal` errors.

### 2.2 Unhandled-error policy (never die silently)

A per-app `UnhandledErrorHandler`, default depending on project kind:

| Context | Default behaviour |
|---|---|
| Under the IDE | VM pauses at the signalling frame; debugger opens on the exact line with locals. User can fix the method, restart the frame, or continue. |
| Console exe | Print exception class, message and a **symbolicated stack trace with file:line**, exit code 1. |
| Forms exe | Modal error dialog (message, details expander with stack trace, "Copy", "Continue", "Quit"). Event loop survives: the failing event handler is aborted, the app keeps running. |
| Web exe | Request gets HTTP 500 (JSON body in dev mode with trace), error logged, server keeps serving. |

User override: `Application onUnhandledError: [:e | ... ]`. Also a crash log file option.

### 2.3 Source positions and diagnostics

- Spans on every AST node; the compiler emits a line table per method (bytecode offset →
  line/column), stored in the image (strippable for release builds; keep file:line at least).
- Parser error recovery (sync on `.`, `]`, `)`, method boundaries) producing multiple
  diagnostics with ranges; used by both compiler and editor.
- Compile-time warnings: unused locals/args, assignment to undeclared variable (today a
  bytecode-compiler panic), unknown global (after module resolution), unreachable code
  after `^`, shadowing, `ifFalse:ifTrue:` on standard booleans (the AGENTS.md footgun).

### 2.4 Module system

Requirements: easy to create and import libraries, no global name clashes, deterministic
builds, and still "one class per file" to stay SOM-like.

**Module = a directory (or a `.somlib` archive) with a manifest.** A project is a module;
every dependency is a module. Class names are unique *within* a module.

File-level syntax (new, optional header before the class definition):

```smalltalk
import Collections.                       "brings in the module's exported classes"
import Net.Http (Client, Request).        "selective"
import Json as J.                         "qualified only: J::Parser"

MainForm = Form (
    | client |
    load = ( client := Client new. ^ J::Parser parse: '{}' )
)
```

- Resolution order for a bare name: current module → explicitly imported names → `Core`
  (implicitly imported: Object, String, Integer, Array, Exception, ...). Ambiguity is a
  compile error, fixed with `as` aliasing or `Module::Class` qualification.
- `::` is a new lexer token; qualified names only in expression/superclass position, so
  existing SOM files are unaffected.
- Exports: all classes are public by default; a class can be marked internal with a
  `<internal>` pragma after its name. Methods can be marked `<private>` (enforced for sends
  not to `self`/`super`, warnings first, errors later).
- Resolution happens at **link time** in `som-compiler`, producing fully-qualified globals
  in the image; runtime lookups by bare name go through the module table, so `System
  global:` keeps working for reflective code.
- **Class extensions** (needed because `SOM/` is read-only, and handy for libraries):
  ```smalltalk
  extend String (
      trimmed = ( ... )
      asJson  = ( ^ Json::Writer write: self )
  )
  ```
  Extensions are scoped: visible only in modules that import the module defining them
  (avoids the classic monkey-patching conflicts). `Core`'s own extensions of the SOM
  classes are always visible.
- Circular imports between modules are an error; within a module anything goes.

### 2.5 Smaller language additions

- Dynamic array literal `{ a. b. c }` and dictionary literal `#{ #name -> 'x'. #age -> 3 }`.
- String escapes, multiline strings, string interpolation via `'Hello {name}' format: ...`
  (library, not syntax).
- `Character` class; proper `Symbol` interning.
- Class-side `main: args` convention as the program entry point (replaces instance `run`/`run:`).
- Optional type hints for tooling only (no runtime checks), as pragmas:
  `<returns: String>` on methods and `| total <Integer> |` for locals/fields.
  Used by completion and hover; ignored by the VM.
- Properties sugar for form/UI classes (generated accessors for declared fields) — via a
  `<property>` pragma on fields, so designers can enumerate them.

### 2.6 VM hardening and performance

- Non-recursive VM with explicit frame stack; configurable max stack (default ~100k frames);
  `StackOverflow` exception instead of a crash.
- `Value::SmallInt(i64)` fast path with overflow promotion to `BigInt`.
- Interned symbols (`u32` ids), slot-indexed locals/args (no `HashMap` per activation).
- Inline caches at send sites; method cache invalidated on method install (keeps live
  edit-and-continue possible).
- Primitive registry keyed by `(class, selector)` with a declarative table / proc-macro,
  replacing the manual `MethodDef` injection blocks in `universe.rs`.
- Remove `NativeHandle(usize)` raw pointer casts: native resources become `Value::Native(
  Gc<dyn NativeObject>)` with type checks and finalizers (file handles, sockets, windows).
- Threads: the `gc` crate's `Gc` is `!Send`, so concurrency = **isolates** (one VM per OS
  thread, message passing with copied/serialized values; the existing `serialize.rs` is the
  starting point). Exposed as `Worker spawn: [...]`, `Channel`, and `Future`/`Promise`
  integrated with the async IO event loop.
- Interrupt/pause hooks at back-edges and sends, so the debugger can break into infinite loops.

---

## 3. Standard library

Style: **PureBasic's breadth and "just works" pragmatism** (one obvious way to read a file,
download a URL, show a dialog), organised like the **Java class library** (clear packages,
consistent collection interfaces, streams, exceptions per package). Implemented in SOM on
top of a small set of native primitives in `som-stdlib`. Each package is a module, so apps
only link what they import (keeps executables small).

| Module | Contents | Inspiration |
|---|---|---|
| `Core` | Object additions, Exception hierarchy, Boolean/Number/String/Character/Symbol extensions, `Comparable`, `Assert` | java.lang |
| `Collections` | `List` (growable), `LinkedList`, `Map`/`HashMap`, `SortedMap`, `Set`, `Deque`, `Stack`, `Queue`, `Interval`; common protocol `do: collect: select: reject: detect:ifNone: inject:into: sortBy: groupBy:`; iterators | java.util |
| `Text` | `StringBuilder`, `Regex` (wrap `regex` crate), split/join/trim/pad, `Format` (numbers, dates, `'{0} of {1}'`), encoding (UTF-8/16, Base64, hex) | java.util.regex, java.text, PureBasic String lib |
| `IO` | `File`, `Directory`, `Path` (normalize, join, extension…), `FileStream`, `Reader`/`Writer`, `BufferedReader` (readLine), `BinaryReader`/`Writer`, temp files, file watching | java.io / java.nio.file, PureBasic File/FileSystem |
| `Time` | `Date`, `Time`, `DateTime`, `Duration`, `Stopwatch`, `Timer` (periodic callbacks), time zones (UTC + local) | java.time |
| `Math` | `Math` functions, `Random` (seedable), `Decimal` (money), `BigInteger` already native | java.lang.Math, java.math |
| `System` | `Environment` (vars, args, OS, paths like AppData/Home), `Process` (run, pipes, exit code), `Clipboard`, `Console` (colors, readLine, key input), `Log` | PureBasic Process/Console, java.lang.System |
| `Net` | `TcpClient`/`TcpServer`, `UdpSocket`, `HttpClient` (get/post/json/download with progress), `Url`, `Mail` (SMTP send) | java.net, PureBasic Network/HTTP |
| `Json` / `Csv` / `Xml` / `Ini` | parse/stringify, mapping to/from SOM objects via declared fields | PureBasic JSON/XML/Preference |
| `Data` | `Sqlite` (bundled via `rusqlite` feature), a small `Database` protocol for other drivers later, prepared statements, result sets | java.sql, PureBasic Database |
| `Crypto` | hashes (SHA-256/MD5/CRC32), HMAC, secure random, AES-GCM | PureBasic Cipher, java.security |
| `Compression` | zip read/write, gzip | java.util.zip |
| `Concurrent` | `Worker`, `Channel`, `Future`, `Promise`, event loop integration (absorbs today's `Async/`) | java.util.concurrent |
| `Test` | xUnit (`TestCase`, assertions, runner with file:line failures) — compatible with the SOM TestSuite style | JUnit |
| `Forms` | see §4.2 | VB6 intrinsic controls, PureBasic Gadgets |
| `Web` | see §4.3 | — |
| `Graphics` | `Image` load/save/scale (the `image` crate), `Canvas` 2D drawing, colors | PureBasic 2DDrawing/Image |

Documentation: doc comments (`"! ..."` convention) extracted into the library index,
shown in hover/completion and an in-IDE help browser (F1 on a selector).

---

## 4. Project kinds and runtimes

### 4.1 Console apps & libraries

- Console: `App class >> main: args` → exit code from the returned integer (or 0).
- Library: no entry point; builds to a `.somlib` (manifest + compiled modules + optional
  sources + symbol/doc index). Test runner integrated (`lazysom test`, IDE Test Explorer).
- Templates: "Console App", "Class Library", "Unit Test Project".

### 4.2 Form apps (Visual Basic model)

**Declarative form file + code-behind**, like VB6's `.frm` split cleanly in two:

`MainForm.form` (TOML or a SOM-literal format; designer-owned, humans may edit):
```toml
[form]
class = "MainForm"
title = "Invoices"
size  = [640, 420]
startPosition = "center"

[[control]]
type = "Button"
name = "saveButton"
text = "&Save"
bounds = [520, 370, 100, 28]
anchor = ["right", "bottom"]
events = { click = "saveButtonClick:" }

[[control]]
type = "DataGrid"
name = "invoicesGrid"
bounds = [10, 10, 610, 350]
anchor = ["left", "top", "right", "bottom"]
```

`MainForm.som` (user-owned code-behind):
```smalltalk
MainForm = Form (
    load = ( invoicesGrid rows: Invoice all )
    saveButtonClick: sender = ( self save. MessageBox info: 'Saved' )
)
```
- The compiler generates a hidden `MainForm_Designer` superclass from the `.form` file
  (fields for every control, `initializeComponents`, event wiring), so the user class
  just subclasses it — the VB/WinForms partial-class trick without partial classes.
- **Runtime (`som-forms`)**: retained widget tree owned by Rust, rendered with egui at
  absolute positions (`egui::Area`/`put` at rects) with VB.NET-style `anchor` and `dock`
  layout, plus optional flow/grid containers. Events are queued and dispatched to SOM on
  the VM thread between frames — no more SOM code running *inside* egui closures with raw
  pointers. Controls are real SOM objects with properties (`text`, `enabled`, `visible`,
  `bounds`, `font`, `foreColor`, `backColor`, `tabIndex`, `tag`).
- Control set v1: Form, Label, Button, TextBox (single/multi, password), CheckBox,
  RadioButton, ComboBox, ListBox, GroupBox/Panel, TabControl, PictureBox, ProgressBar,
  Slider, NumericUpDown, DatePicker, DataGrid, TreeView, MenuBar/ContextMenu, ToolBar,
  StatusBar, Timer (non-visual), Canvas (custom painting, absorbs `EguiPainter`).
- Standard dialogs: `MessageBox`, `InputBox`, open/save file dialog, folder picker,
  color picker (`rfd` crate for native file dialogs).
- Multiple forms, modal `showDialog` returning a result, `Application run: MainForm`.

**Form designer (IDE)**: toolbox → drag onto a design surface that renders the *same*
`som-forms` controls (true WYSIWYG), select/move/resize handles with snap-to-grid and
alignment guides, multi-select align/distribute, tab-order mode, anchors editor,
Properties window (typed editors: text, number, color, font, enum, bounds, image
resource), Events tab: double-click an event → creates/jumps to the handler method in
code-behind (VB's signature feature). Undo/redo on the form model. Designer ↔ file is a
pure serialisation of the form model, so hand edits round-trip.

### 4.3 Web apps (API designer)

**Declarative API file + controller classes:**

`Api.api` (TOML; designer-owned):
```toml
[api]
title    = "Invoice API"
basePath = "/api/v1"

[[model]]
name   = "Invoice"
fields = { id = "Integer", customer = "String", total = "Decimal", paid = "Boolean?" }

[[route]]
method  = "GET"
path    = "/invoices/{id}"
handler = "InvoiceController>>show:"
params  = { id = "Integer" }
returns = { 200 = "Invoice", 404 = "Error" }

[[route]]
method  = "POST"
path    = "/invoices"
handler = "InvoiceController>>create:"
body    = "Invoice"
returns = { 201 = "Invoice" }
```

```smalltalk
InvoiceController = Controller (
    show: request = (
        ^ (Invoice find: (request param: #id))
            ifNil: [ Response notFound ]
            ifNotNil: [:inv | Response ok: inv ]
    )
)
```
- Runtime (`som-web`): hyper/axum on tokio; router generated from the `.api` file;
  path/query/body parameters validated and converted **before** reaching SOM (400 on
  mismatch); models mapped to/from JSON by field names; static files from `wwwroot/`;
  middleware chain in SOM (`Middleware` class: logging, auth, CORS); simple HTML
  templating for server-rendered pages.
- Concurrency: a pool of VM isolates (one per worker thread), each with the app loaded;
  shared state goes through `Data`/`Sqlite` or explicit channels. Keeps the VM single-
  threaded and the GC simple.
- **API designer (IDE)**: table of routes (method, path, handler, status codes) with a
  detail pane, model editor (fields/types), "Generate handler" jumps into the controller,
  **OpenAPI 3 export/import**, and a built-in "Try it" request panel (like Postman) that
  hits the running dev server. Hot reload: saving a controller method reinstalls it in
  the running isolates without restart.

---

## 5. The IDE

### 5.1 Shell and layout (VB6 / modern VS hybrid)

- Menu bar (File, Edit, View, Project, Build, Debug, Tools, Help) + toolbar
  (New, Open, Save, Run ▶ F5, Pause, Stop, Step Into F11 / Over F10 / Out Shift-F11, Build).
- Dockable panels (`egui_dock`), layout persisted per user:
  - **Project Explorer** — tree: Forms, APIs, Classes, Resources, References
    (right-click "Add Reference…" = VB References dialog: pick library from store,
    folder, `.somlib`, or git URL).
  - **Editor tabs** — code files, form designers, API designers.
  - **Toolbox** (visible with a form designer) and **Properties** window.
  - **Outline** — classes/methods of the current file.
  - **Output** (build + program stdout/stderr), **Problems** (diagnostics, click-to-jump).
  - **Immediate window** — a REPL against the running/paused program (replaces the
    Workspace; `? expr` prints like VB).
  - Debug: **Call Stack**, **Locals**, **Watch**, **Breakpoints**, object inspector.
- Start page: recent projects, "New Project" wizard with the four templates.
- Settings dialog (theme, font, tab size, keymap) stored in the user config dir
  (not `Tools/ide_config.txt` in the repo).

### 5.2 Code editor

- File-based editing of whole class files (no per-method browser).
- Syntax highlighting from `som-syntax`'s lexer (incremental re-lex of the changed lines),
  semantic highlighting from the language index (fields vs locals vs globals vs unknown).
- Gutter: line numbers, breakpoints (click), current-line arrow, error/warning markers;
  squiggles with hover tooltips; code folding per method.
- Completion (Ctrl+Space and automatic after a receiver + space or after `:`):
  - variables in scope, fields, globals/classes from the module and its imports;
  - selectors: of the receiver's inferred class when known (literals, `X new`, type hints,
    field assignments seen in the class, return-type pragmas), otherwise all known
    selectors ranked by frequency/proximity;
  - keyword-message snippets with tab stops (`at: ‸ put: ‸`), `ifTrue:ifFalse:` snippet;
  - import suggestions for unresolved class names ("Add `import Net.Http`").
- Signature help for keyword messages, hover docs, go to definition (F12), find
  implementors / senders, rename symbol (class, method, field, local), find/replace with
  regex, multi-cursor (later), format document.
- Implementation path: start with `egui::TextEdit` + custom `layouter` for highlighting and
  an overlay for gutter/popups; replace with a dedicated editor widget over a `ropey`
  buffer once files get large or features (folding, multi-cursor) need it. Budget time for
  this — the editor is the product.

### 5.3 Build & run

- Build pipeline: parse → resolve modules → compile → link (tree-shake unused modules and
  classes, strip docs/sources for release) → write image → append to runtime stub.
- Configurations: Debug (full line tables, assertions) / Release (stripped).
- F5: build incrementally (only changed files recompiled; per-module cache in `build/`),
  launch runtime with `--debug-port`, attach (see §5.4).

### 5.4 Debugger

The existing `vm_runner.rs` pause/step channel and `serialize_stack` are the seed; they
move into the VM (`som-vm` debug hooks) and a wire protocol so the same debugger works for
every project kind and for built debug executables.

**Architecture**
- `som-vm` exposes a `DebugHook` checked at sends, back-edges and statement boundaries
  (statement boundaries come from the line table). Zero cost when no debugger is attached
  (a single flag test).
- Every runtime stub accepts `--debug <port>` (or `LAZYSOM_DEBUG=port`); the IDE connects
  over a local TCP socket with a JSON protocol modelled on DAP (initialize, setBreakpoints,
  configurationDone, continue, next, stepIn, stepOut, pause, stackTrace, scopes, variables,
  evaluate, setVariable, exceptionInfo, plus our `replaceMethod` / `restartFrame`).
  Keeping it DAP-shaped means `lazysom dap` can later serve VS Code for free.
- Debug builds keep full line tables, local-variable names and source hashes; the IDE
  refuses to place breakpoints when the source changed since the build (offers rebuild).
- Web apps: the debugger attaches to all isolates; hitting a breakpoint pauses only the
  isolate serving that request, the others keep serving (or "pause all" option).
- Forms apps: while paused, the window stops processing events but keeps being repainted
  by the runtime (no "Not responding"), and the IDE is brought to front.

**Features (v1)**
- Breakpoints: line, conditional (`count > 10`), hit count, logpoints (print a message
  without stopping), method breakpoints (`Invoice>>total`), enable/disable, persisted per
  project.
- Exception breakpoints: break on unhandled (default on), on any `Error`, or on specific
  classes (`MessageNotUnderstood`); "just my code" option to skip library frames.
- Stepping: into, over, out, run to cursor, set next statement (within a method); block
  bodies step naturally (step-into on `do:` enters the block, not the collection code,
  when "just my code" is on).
- Pause at any time, including inside infinite loops (back-edge hook).
- Call Stack panel with file:line per frame, blocks shown as `[] in Invoice>>total`,
  library frames collapsed; clicking a frame switches locals and the editor.
- Locals / Watch / Inspector: args, temps, `self` fields, outer-block variables; lazy
  expansion of objects, collections shown by element (`List` size 3: …), strings with
  full-text viewer; edit values in place.
- Immediate window evaluates in the selected frame's scope (`? total * 2`,
  `items add: x`), with completion.
- **Edit and continue**: save while paused → method recompiled and reinstalled in the live
  VM (inline caches invalidated), current frame restarted if it was the edited method.
  Shape changes (adding fields) require restart, clearly reported.
- Output panel captures stdout/stderr of the debuggee with clickable `File.som:42` links;
  stack traces printed by crashed standalone exes are also clickable when pasted.
- Debugger visualisation for forms: "highlight control" from the Locals view.

**Later**
- Data breakpoints (break when field X of object Y changes), reverse stepping via
  recorded frames, remote debugging of an exe on another machine (same protocol over the
  network, token-protected), CPU/allocation profiler reusing the same hooks.

### 5.5 What is removed

- `Tools/LazyIde.som`, `Browser.som`, `Workspace.som`, `HelpPanel.som`, `Debugger.som`,
  `EguiContext/EguiUi/EguiPainter.som`, `Tools/ide_config.txt`.
- `src/gui.rs` (`SomGuiApp` driving `renderFrameOn:`), the `--gui` flag, the global
  `VM_RUNNER` singleton and the `System>>bgTask*` / `registerGui*` / `evaluateAsync:`
  primitives, the raw-pointer egui primitives in `primitives/gui.rs`.
- The AST interpreter, once the VM reaches parity (keep it until then as the reference
  implementation for differential testing).

---

## 6. Self-contained executables

- `som-runtime` builds three stubs per target: `console`, `forms` (egui + glow/wgpu), `web`
  (hyper). Features are compiled in only where needed; `Data`/`Crypto`/`Compression` are
  optional features that produce additional stub variants or are enabled in all stubs if
  size allows (decide with measurements).
- Stub layout: `[runtime binary][image bytes][trailer: magic "LSOMIMG1", image offset,
  image length, flags, checksum]`. At startup the stub reads its own executable, finds the
  trailer, and boots the image. No temp files, no DLLs.
- Image format v2: versioned header, module table, string/symbol pool, classes, methods,
  line tables, resource blobs (icons, images, embedded files accessible via
  `Resources at: 'logo.png'`). Replace bincode's 8-byte lengths with a documented compact
  format (keeps the retro-VM story in `README_bytecode.md` alive).
- Windows: separate `windows` subsystem stub for forms apps (no console window); icon and
  version info written into the PE resource section at build time (`editpe`-style crate or
  a small resource writer). Linux: static musl build for console/web stubs; forms stub
  links dynamically only against system GL/X11/Wayland libs.
- Stubs for all supported targets ship with the IDE, so a Windows IDE can build Linux
  binaries (and vice versa) with no toolchain.
- Size targets (release, stripped, `opt-level="z"`, LTO — already configured):
  console ≤ 3 MB, web ≤ 6 MB, forms ≤ 10 MB. Track in CI.
- Later: code signing hook, macOS target, installer generation (zip / `.msi` via WiX).

---

## 7. Library workflow ("easy to create and import")

- **Create**: File → New Project → Class Library. Build produces `build/Name-1.0.0.somlib`.
- **Use locally**: Project Explorer → References → Add → pick a folder or `.somlib`.
- **Share**: `lazysom package` → `.somlib`; `lazysom publish` to a git repo (tags as
  versions). A central registry is explicitly out of scope for v1; the design (name +
  semver + source) leaves room for one.
- Store: `~/.lazysom/libs/<name>/<version>/`; `MyApp.somlock` pins resolved versions
  and checksums.
- A `.somlib` carries its symbol + doc index, so completion works for binary-only
  libraries.
- Native extensions (Rust crates adding primitives) are out of scope for v1 because they
  break "one stub, append image". v2 option: an "extended runtime" build via `cargo`
  when the user has a toolchain.

---

## 8. Phased delivery

Each phase ends in something usable and keeps TestSuite + `cargo test` green.

### Phase 0 — Make the reference interpreter honest (days, not weeks)
The AST interpreter is the oracle for the new VM, so its silent failures get fixed first:
- Loading a class whose `primitive` method has no Rust implementation → load error
  listing the missing primitives (replaces the silent `nil` stub).
- Implement `System>>signalError:` (stop with message + interpreter stack of
  `Class>>selector` frames, exit 1), `errorPrint:`, `errorPrintln:`, `printStackTrace`,
  `instVarNamed:`, `perform:withArguments:inSuperclass:`, `invokeOn:with:`.
- Audit all primitives for panics (`at:put:` bounds, argument type assumptions) →
  return errors instead. Add a test that runs every primitive with bad arguments.
- Raise the dispatch depth limit to count method activations, not every nested dispatch,
  and make it configurable.
- Add a Windows job to CI (Windows is a shipping target; CI only runs on Linux today).
- Mark the bytecode VM path (`--compile-image`/`--run-image`) as experimental in the
  README until Phase 1 replaces it.
- **Exit**: every probe in §0.1 either works or stops with a clear error and non-zero exit.

### Phase 1 — Foundations (engine consolidation)
- Split into workspace crates; declarative primitive registry.
- Spans in AST, error-tolerant parser, diagnostics with line/col.
- Bytecode VM: non-recursive, SmallInt, interned symbols, slot locals, line tables,
  fix compiler panics; primitives decoupled from the AST interpreter.
- `DebugHook` in the VM (sends, back-edges, statement boundaries) so the debugger is
  designed in from the start rather than retrofitted.
- Differential test harness: run TestSuite + `tests/*.som` on both engines.
- **Exit**: VM passes the full SOM TestSuite; errors print `File.som:42` stack traces.

### Phase 2 — Industrial language core
- Exceptions (`on:do:`, `ensure:`, `ifCurtailed:`, resumable/non-resumable, retry/pass).
- Typed primitive errors; `doesNotUnderstand:` → `MessageNotUnderstood`; unhandled-error
  policy per app kind.
- Module system + `import` + `::` + class extensions + `.somproj` parsing + linker.
- `Core`, `Collections`, `Text`, `IO`, `Time`, `System`, `Test` modules.
- CLI: `lazysom new|build|run|test` for console & library projects; runtime stub +
  appended image for Windows and Linux.
- AST interpreter removed.
- **Exit**: a console app with a library dependency builds into a single exe on both OSes;
  an uncaught error prints a symbolicated trace and exits 1.

### Phase 3 — IDE shell, editor, debugger
- Rust/egui IDE: project explorer, tabs, output, problems, settings, new-project wizard.
- `som-langserv`: index, diagnostics, highlighting, completion, go-to-def, hover, rename.
- VM debug hooks (added in Phase 1 alongside line tables) + DAP-shaped debug protocol in
  every runtime stub; out-of-process run.
- Debugger v1 as in §5.4: breakpoints (line/conditional/logpoint/exception), stepping,
  pause, call stack, locals/watch/inspector, immediate window in frame scope,
  edit-and-continue.
- Delete the SOM-based Tools IDE and `gui.rs`.
- **Exit**: create, edit, debug and build a console app entirely inside the IDE.

### Phase 4 — Form apps
- `som-forms` runtime (retained tree, anchors/dock, events, dialogs, v1 control set).
- `.form` format + generated designer superclass.
- Form designer: toolbox, WYSIWYG surface, properties/events window, double-click-to-handler.
- `Graphics` module, Windows-subsystem stub, icons/resources.
- **Exit**: the classic "address book with SQLite" sample is built in < 30 min from scratch,
  shipped as a single exe for Windows and Linux.

### Phase 5 — Web apps
- `som-web` runtime (router, validation, JSON mapping, static files, middleware, isolate pool).
- `.api` format, API designer, OpenAPI export/import, "Try it" panel, hot reload.
- `Net`, `Json`, `Data`, `Crypto`, `Concurrent` modules.
- **Exit**: a CRUD JSON API with SQLite runs from a single exe and passes an OpenAPI
  contract test.

### Phase 6 — Polish & ecosystem
- Library packaging/publishing from the IDE, lockfiles, git sources.
- Performance pass (inline caches, profiler view in IDE), more controls (TreeView,
  charts), documentation browser, sample gallery, LSP for external editors,
  installers and code signing hooks.

Rough ordering rationale: exceptions and line numbers (Phases 1–2) make everything after
them debuggable; the IDE comes before forms because the form designer is an IDE feature;
web comes last because it reuses isolates, JSON and the module system.

---

## 9. Risks and open questions

- **Editor quality in egui.** `TextEdit` is not a code editor; a custom widget may be needed
  earlier than hoped. Mitigation: isolate the editor behind a trait; evaluate
  `egui_code_editor` / writing our own in Phase 3's first weeks.
- **`gc` crate limits.** `Gc` is `!Send`, collection pauses are unbounded, cycle collection
  cost unknown at scale. Isolates sidestep threading; a custom GC could come later behind
  the `som-vm` API.
- **Forms fidelity on egui.** Absolute positioning and native look are not egui's strengths;
  accept a consistent custom look (like PureBasic's cross-platform gadgets) rather than
  native widgets. Revisit native backends only if users demand it.
- **Superset constraint.** New syntax (`import`, `::`, `extend`, `{ }`, `#{ }`) must not
  change the meaning of any valid SOM program — each addition gets a TestSuite run plus
  parser golden tests.
- **Decisions to confirm before Phase 2:**
  1. Form/API file format: TOML (proposed; readable, good Rust support) vs. SOM literal syntax.
  2. Exception semantics: full Pharo (resumable, `outer`, `retry`) vs. a simpler
     try/catch/finally subset first.
  3. Extension visibility: scoped to importers (proposed) vs. global.
  4. Whether the IDE should eventually be scriptable in SOM (plugins), which affects how
     much of `som-langserv` is exposed to SOM.
