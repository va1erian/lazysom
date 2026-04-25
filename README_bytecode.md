# SOM Bytecode and VM Image Documentation

This document describes the structure of the compiled bytecode image format, the bytecodes themselves, and a brief guide on how to implement an interpreter for it.

## 1. Image Format
The AST of a SOM program is compiled down into an `Image` which is serialized using the standard `bincode` crate (v1.3.3).

If you plan to write an interpreter for retro computers (e.g. C or Assembly), please be aware of `bincode`'s memory format.
* All integers (`u8`, `u16`, `u32`) are little-endian.
* Lengths of dynamic arrays (`Vec<T>`, `String`) are encoded as 8-byte little-endian unsigned integers (`u64`).

For a C or Assembly interpreter, it is usually much simpler to write a short deserialization script in Rust that loads this `image.bin` with `bincode` and dumps it out byte-for-byte in a custom layout without 64-bit lengths or dynamic sizes.

### Image Structure
```rust
pub struct Image {
    pub string_pool: Vec<String>,
    pub classes: Vec<CompiledClass>,
}
```

The image starts with an 8-byte length representing the number of strings in the `string_pool`. Following are the strings (8-byte length followed by UTF-8 bytes).
Then follows the 8-byte length of the `classes` array, and then the serialized `CompiledClass` objects.

Each `CompiledClass` contains the class definition:
* `name`: `u32` index to the string pool.
* `super_class`: `Option<u32>` (1 byte `tag` = 1 if some, followed by the `u32` index, or 0 if none).
* `instance_fields`: A `Vec<u32>` of instance field names in order.
* `class_fields`: A `Vec<u32>` of class field names.
* `instance_methods`: A `Vec<CompiledMethod>`.
* `class_methods`: A `Vec<CompiledMethod>` for the metaclass.

### Compiled Methods and Blocks
```rust
pub struct CompiledMethod {
    pub name: u32,
    pub is_primitive: bool,
    pub bytecodes: Vec<u8>,
    pub constants: Vec<Constant>,
    pub blocks: Vec<CompiledBlock>,
    pub num_locals: u16,
    pub num_args: u16,
    pub signature: u32,
}
```

Constants are stored in a constants array (`Vec<Constant>`). When bytecodes need to reference a symbol, string, integer or array, they use an index to lookup the corresponding constant.

**Note:** `is_primitive` is serialized as a 1 byte boolean (0 = false, 1 = true).

`Constant` serialization starts with a 4-byte little-endian tag (bincode enum tag, though sometimes 1-byte, `bincode 1.x` uses 4-byte tags for enums):
- Tag 0: Integer (u32 index to string pool)
- Tag 1: Double (8-byte IEEE 754 f64)
- Tag 2: String (u32 index to string pool)
- Tag 3: Symbol (u32 index to string pool)
- Tag 4: Array (Vec<Constant>)

Blocks (`CompiledBlock`) are conceptually anonymous methods without a name or signature but share the same attributes otherwise.

## 2. Bytecode Instructions

The VM relies on a stack-based instruction set. The instruction sequence is a flat `Vec<u8>`.

### Halting
* `Halt` (0): Stop VM execution.

### Stack Operations
* `Dup` (1): Duplicates the top value on the stack.
* `Pop` (12): Pops and discards the top value on the stack.

### Contextual Reads/Writes (Variables, arguments, fields)
Variables are resolved dynamically during compilation based on lexical context levels. `0` refers to the innermost enclosing scope (the current block or method itself). Levels > 0 reference progressively outer contexts.

* `PushLocal` (2) `<level: u8> <idx: u8>`: Pushes a local variable onto the stack.
* `PushArgument` (3) `<level: u8> <idx: u8>`: Pushes an argument onto the stack.
* `PushField` (4) `<sym_idx: u16>`: Looks up the symbol at `sym_idx` in constants and fetches the corresponding object field to push onto the stack.
* `PushBlock` (5) `<idx: u16>`: Fetches a `CompiledBlock` and creates a closure/activation from it.
* `PushConstant` (6) `<idx: u16>`: Fetches a constant from the `constants` array and pushes it to the stack.
* `PushGlobal` (7) `<sym_idx: u16>`: Looks up the symbol at `sym_idx` in constants, and fetches it from the global environment.
* `PushSelf` (8) `<level: u8>`: Pushes `self` onto the stack.
* `PushNil` (9), `PushTrue` (10), `PushFalse` (11): Convenience bytecodes to push standard singletons.

The equivalent operations to pop from the stack and store into a variable exist:
* `PopLocal` (13) `<level: u8> <idx: u8>`
* `PopArgument` (14) `<level: u8> <idx: u8>`
* `PopField` (15) `<sym_idx: u16>`

### Message Sends
* `Send` (16) `<sym_idx: u16>`: Sends a message. The selector is resolved via `constants[sym_idx]`. It pops arguments then pops the receiver from the stack.
* `SuperSend` (17) `<sym_idx: u16>`: Same as `Send`, but the method lookup begins at the superclass of the class containing the method.

### Control Flow
* `ReturnLocal` (18): Pops the top of the stack and returns it from the current block or method.
* `ReturnNonLocal` (19): Pops the top of the stack and returns it from the enclosing *method* context (escaping any enclosing blocks).

## 3. Writing an Interpreter

1. **Load Image:** Deserialize the `.bin` using bincode. This yields the `Image` structure.
2. **Environment:** Initialize global variables like `system`, `nil`, `true` and `false`. The core classes should be instantiated.
3. **Execution Context:** The VM runs inside an execution loop relying on frames (activations). Each time a method or block is invoked, a new Frame is created.
    - A Frame contains an operand stack, local variables, arguments, the instruction pointer (`ip`), and the enclosing `context` (if it's a block).
4. **Dispatch:** When a `Send` instruction happens, pop `N` arguments, then pop the receiver. Lookup the method in the receiver's class or superclasses. Execute the method by creating a Frame and running the inner instruction loop.
5. **Primitive Mapping:** Since the Image doesn't contain Rust closures, primitive methods are flagged with `is_primitive = true`. The interpreter must map these primitive methods to native environment implementations.

## Primitives Needed by the VM

When executing an image, the VM expects the following primitives (mapped as `Class>>methodName`) to be implemented natively by the VM environment in order for the standard SOM classes and examples to function correctly:

* `Object>>==`
* `Object>>hashcode`
* `Object>>objectSize`
* `Object>>perform:`
* `Object>>perform:inSuperclass:`
* `Object>>perform:withArguments:`
* `Object>>perform:withArguments:inSuperclass:`
* `Object>>instVarAt:`
* `Object>>instVarAt:put:`
* `Object>>instVarNamed:`
* `Object>>class`
* `Class>>new`
* `Class>>name`
* `Class>>superclass`
* `Class>>fields`
* `Class>>methods`
* `Metaclass>>new`
* `String>>concatenate:`
* `String>>asSymbol`
* `String>>length`
* `String>>=`
* `String>>substringFrom:to:`
* `String>>hashcode`
* `String>>isWhiteSpace`
* `String>>isLetters`
* `String>>isDigits`
* `Array>>at:`
* `Array>>at:put:`
* `Array>>length`
* `Block>>restart`
* `Integer>>+`
* `Integer>>-`
* `Integer>>*`
* `Integer>>/`
* `Integer>>//`
* `Integer>>%`
* `Integer>>&`
* `Integer>>=`
* `Integer>><`
* `Integer>><=`
* `Integer>>>`
* `Integer>>>=`
* `Integer>>asString`
* `Integer>>as32BitSignedValue`
* `Integer>>as32BitUnsignedValue`
* `Integer>>fromString:`
* `Double>>+`
* `Double>>-`
* `Double>>*`
* `Double>>/`
* `Double>>%`
* `Double>>=`
* `Double>><`
* `Double>><=`
* `Double>>>`
* `Double>>>=`
* `Double>>asString`
* `Double>>asInteger`
* `Double>>cos`
* `Double>>sin`
* `Double>>round`
* `Double>>PositiveInfinity`
* `Double>>fromString:`
* `Method>>signature`
* `Method>>holder`
* `Method>>invokeOn:with:`
* `System>>printString:`
* `System>>printNewline`
* `System>>errorPrint:`
* `System>>errorPrintln:`
* `System>>time`
* `System>>ticks`
* `System>>fullGC`
* `System>>global:`
* `System>>global:put:`
* `System>>load:`
* `System>>exit:`
