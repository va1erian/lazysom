use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Opcode {
    Halt,
    Dup,
    PushLocal(u8, u8),    // context_level, index
    PushArgument(u8, u8), // context_level, index
    PushField(u16),       // index into constants (symbol) for dynamic lookup, or u8 if static
    PushBlock(u16),       // index into blocks
    PushConstant(u16),    // index into constants
    PushGlobal(u16),      // index into constants (symbol)
    PushSelf(u8),         // context_level
    PushNil,
    PushTrue,
    PushFalse,
    Pop,
    PopLocal(u8, u8),
    PopArgument(u8, u8),
    PopField(u16),
    Send(u16),            // index into constants (selector)
    SuperSend(u16),       // index into constants (selector)
    ReturnLocal,
    ReturnNonLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Constant {
    Integer(String),
    Double(f64),
    String(String),
    Symbol(String),
    Array(Vec<Constant>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledBlock {
    pub bytecodes: Vec<Opcode>,
    pub constants: Vec<Constant>,
    pub blocks: Vec<CompiledBlock>,
    pub num_locals: usize,
    pub num_args: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompiledClass {
    pub name: String,
    pub super_class: Option<String>,
    pub instance_fields: Vec<String>,
    pub class_fields: Vec<String>,
    pub instance_methods: std::collections::HashMap<String, CompiledMethod>,
    pub class_methods: std::collections::HashMap<String, CompiledMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    pub classes: std::collections::HashMap<String, CompiledClass>,
}
