use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use derivative::Derivative;
use num_bigint::BigInt;

pub type SomRef<T> = Rc<RefCell<T>>;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(BigInt),
    Double(f64),
    String(SomRef<String>),
    Symbol(String),
    Boolean(bool),
    Nil,
    Object(SomRef<SomObject>),
    Class(SomRef<SomClass>),
    Array(SomRef<Vec<Value>>),
    #[allow(dead_code)]
    Method(SomRef<SomMethod>),
    Block(SomRef<SomBlock>),
    CompiledBlock(SomRef<CompiledBlockInstance>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Double(a), Value::Double(b)) => a == b,
            (Value::String(a), Value::String(b)) => Rc::ptr_eq(a, b),
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => Rc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Method(a), Value::Method(b)) => Rc::ptr_eq(a, b),
            (Value::Block(a), Value::Block(b)) => Rc::ptr_eq(a, b),
            (Value::CompiledBlock(a), Value::CompiledBlock(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

pub struct SomObject {
    pub class: SomRef<SomClass>,
    pub fields: Vec<Value>,
}

impl PartialEq for SomObject {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.class, &other.class) && self.fields == other.fields
    }
}

impl std::fmt::Debug for SomObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SomObject")
            .field("class", &self.class.borrow().name)
            .field("fields_count", &self.fields.len())
            .finish()
    }
}

pub struct SomClass {
    pub name: String,
    pub class: Option<SomRef<SomClass>>, // Metaclass
    pub super_class: Option<SomRef<SomClass>>,
    pub instance_fields: Vec<String>,
    pub fields: Vec<Value>, // Class fields
    pub methods: HashMap<String, SomRef<SomMethod>>,
    pub method_order: Vec<String>,
}

impl PartialEq for SomClass {
    fn eq(&self, other: &Self) -> bool {
        let class_eq = match (&self.class, &other.class) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let super_class_eq = match (&self.super_class, &other.super_class) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        self.name == other.name && class_eq && super_class_eq
    }
}

impl std::fmt::Debug for SomClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SomClass")
            .field("name", &self.name)
            .field("super_class", &self.super_class.as_ref().map(|s| s.borrow().name.clone()))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct SomMethod {
    pub name: String,
    pub signature: String,
    pub holder: SomRef<SomClass>,
    pub parameters: Vec<String>,
    pub body: MethodBody,
}

impl SomMethod {
    pub fn is_primitive(&self) -> bool {
        matches!(self.body, MethodBody::Primitive(_))
    }
}

#[derive(Debug, Clone)]
pub enum MethodBody {
    Ast(crate::ast::Block),
    Primitive(fn(&Value, Vec<Value>, &crate::universe::Universe, &crate::interpreter::Interpreter) -> anyhow::Result<crate::interpreter::ReturnValue>),
}

impl PartialEq for SomMethod {
    fn eq(&self, other: &Self) -> bool {
        self.signature == other.signature && self.name == other.name
    }
}

impl PartialEq for MethodBody {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MethodBody::Ast(a), MethodBody::Ast(b)) => a == b,
            (MethodBody::Primitive(_), MethodBody::Primitive(_)) => true, 
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledBlockInstance {
    pub block: crate::bytecode::CompiledBlock,
    pub context: Option<SomRef<crate::bytecode_interpreter::Frame>>,
}

#[derive(Debug, Derivative)]
#[derivative(PartialEq)]
pub struct SomBlock {
    pub body: crate::ast::Block,
    pub context: Option<SomRef<Activation>>, // Lexical context
}

#[derive(Debug, Derivative)]
#[derivative(PartialEq)]
pub struct Activation {
    pub holder: Option<SomRef<SomClass>>,
    pub self_val: Value,
    pub args: HashMap<String, Value>,
    pub locals: HashMap<String, Value>,
    pub parent: Option<SomRef<Activation>>,
    pub is_active: bool,
}

impl Value {
    pub fn new_string(s: String) -> Self {
        Value::String(Rc::new(RefCell::new(s)))
    }

    #[allow(dead_code)]
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
}
