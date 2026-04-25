# SOM Bytecode and VM Image Documentation

This document describes the structure of the compiled bytecode image format, the bytecodes themselves, and a brief guide on how to implement an interpreter for it.

## 1. Image Format
The AST of a SOM program is compiled down into an `Image` which is then serialized using `bincode`.

```rust
pub struct Image {
    pub classes: std::collections::HashMap<String, CompiledClass>,
}
```

The Image is just a collection of `CompiledClass` objects indexed by their name.
Each `CompiledClass` contains the class definition:
* `name`: Name of the class
* `super_class`: Optional name of the superclass.
* `instance_fields`: A list of instance field names in order.
* `class_fields`: A list of class field names.
* `instance_methods`: A hashmap of method selectors to `CompiledMethod`.
* `class_methods`: A hashmap of method selectors to `CompiledMethod` for the metaclass.

### Compiled Methods and Blocks
```rust
pub struct CompiledMethod {
    pub name: String,
    pub is_primitive: bool,
    pub bytecodes: Vec<Opcode>,
    pub constants: Vec<Constant>,
    pub blocks: Vec<CompiledBlock>,
    pub num_locals: usize,
    pub num_args: usize,
    pub signature: String,
}
```

Constants are stored in a constants array (`Vec<Constant>`). When bytecodes need to reference a symbol, string, integer or array, they use an index to lookup the corresponding constant.
Blocks (`CompiledBlock`) are conceptually anonymous methods without a name or signature but share the same attributes otherwise.

## 2. Bytecode Instructions

The VM relies on a stack-based instruction set.

### Halting
* `Halt`: Stop VM execution.

### Stack Operations
* `Dup`: Duplicates the top value on the stack.
* `Pop`: Pops and discards the top value on the stack.

### Contextual Reads/Writes (Variables, arguments, fields)
Variables are resolved dynamically during compilation based on lexical context levels. `0` refers to the innermost enclosing scope (the current block or method itself). Levels > 0 reference progressively outer contexts.

* `PushLocal(level, idx)`: Pushes a local variable onto the stack.
* `PushArgument(level, idx)`: Pushes an argument onto the stack.
* `PushField(sym_idx)`: Looks up the symbol at `sym_idx` in constants and fetches the corresponding object field to push onto the stack.
* `PushGlobal(sym_idx)`: Looks up the symbol at `sym_idx` in constants, and fetches it from the global environment.
* `PushSelf(level)`: Pushes `self` onto the stack.

The equivalent operations to pop from the stack and store into a variable exist:
* `PopLocal(level, idx)`
* `PopArgument(level, idx)`
* `PopField(sym_idx)`

### Constants
* `PushConstant(idx)`: Fetches a constant from the `constants` array and pushes it to the stack.
* `PushNil`, `PushTrue`, `PushFalse`: Convenience bytecodes to push standard singletons.
* `PushBlock(idx)`: Fetches a `CompiledBlock` and creates a closure/activation from it.

### Message Sends
* `Send(sym_idx)`: Sends a message. The selector is resolved via `constants[sym_idx]`. It pops arguments then pops the receiver from the stack.
* `SuperSend(sym_idx)`: Same as `Send`, but the method lookup begins at the superclass of the class containing the method.

### Control Flow
* `ReturnLocal`: Pops the top of the stack and returns it from the current block or method.
* `ReturnNonLocal`: Pops the top of the stack and returns it from the enclosing *method* context (escaping any enclosing blocks).

## 3. Writing an Interpreter

1. **Load Image:** Deserialize the `.bin` using bincode. This yields the `Image` structure.
2. **Environment:** Initialize global variables like `system`, `nil`, `true` and `false`. The core classes should be instantiated.
3. **Execution Context:** The VM runs inside an execution loop relying on frames (activations). Each time a method or block is invoked, a new Frame is created.
    - A Frame contains an operand stack, local variables, arguments, the instruction pointer (`ip`), and the enclosing `context` (if it's a block).
4. **Dispatch:** When a `Send` instruction happens, pop `N` arguments, then pop the receiver. Lookup the method in the receiver's class or superclasses. Execute the method by creating a Frame and running the inner instruction loop.
5. **Primitive Mapping:** Since the Image doesn't contain Rust closures, primitive methods are flagged with `is_primitive = true`. The interpreter must map these primitive methods (`Object>>==`, `Integer>>+`, etc) to native Rust implementations.
