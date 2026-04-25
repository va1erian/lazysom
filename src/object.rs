use std::collections::HashMap;
use derivative::Derivative;
use num_bigint::BigInt;
use gc::{Gc, GcCell, Trace, Finalize, custom_trace, unsafe_empty_trace};

/// Shared, mutable, garbage-collected reference — replaces Rc<RefCell<T>>.
pub type SomRef<T> = Gc<GcCell<T>>;

/// Convenience constructor that mirrors `Rc::new(RefCell::new(v))`.
#[inline]
pub fn som_ref<T: Trace + Finalize + 'static>(v: T) -> SomRef<T> {
    Gc::new(GcCell::new(v))
}

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Finalize)]
pub enum Value {
    Integer(BigInt),
    Double(f64),
    String(SomRef<std::string::String>),
    Symbol(std::string::String),
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

unsafe impl Trace for Value {
    custom_trace!(this, {
        match this {
            Value::String(s)        => mark(s),
            Value::Object(o)        => mark(o),
            Value::Class(c)         => mark(c),
            Value::Array(a)         => mark(a),
            Value::Method(m)        => mark(m),
            Value::Block(b)         => mark(b),
            Value::CompiledBlock(cb)=> mark(cb),
            // Scalars carry no GC pointers
            Value::Integer(_) | Value::Double(_) | Value::Symbol(_)
            | Value::Boolean(_)     | Value::Nil => {}
        }
    });
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Double(a), Value::Double(b)) => a == b,
            (Value::String(a), Value::String(b)) => Gc::ptr_eq(a, b),
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => Gc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => Gc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => Gc::ptr_eq(a, b),
            (Value::Method(a), Value::Method(b)) => Gc::ptr_eq(a, b),
            (Value::Block(a), Value::Block(b)) => Gc::ptr_eq(a, b),
            (Value::CompiledBlock(a), Value::CompiledBlock(b)) => Gc::ptr_eq(a, b),
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// SomObject
// ---------------------------------------------------------------------------

#[derive(Finalize)]
pub struct SomObject {
    pub class: SomRef<SomClass>,
    pub fields: Vec<Value>,
}

unsafe impl Trace for SomObject {
    custom_trace!(this, {
        mark(&this.class);
        for f in &this.fields { mark(f); }
    });
}

impl PartialEq for SomObject {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(&self.class, &other.class) && self.fields == other.fields
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

// ---------------------------------------------------------------------------
// SomClass
// ---------------------------------------------------------------------------

#[derive(Finalize)]
pub struct SomClass {
    pub name: std::string::String,
    pub class: Option<SomRef<SomClass>>,        // Metaclass
    pub super_class: Option<SomRef<SomClass>>,
    pub instance_fields: Vec<std::string::String>,
    pub fields: Vec<Value>,                      // Class-side fields
    pub methods: HashMap<std::string::String, SomRef<SomMethod>>,
    pub method_order: Vec<std::string::String>,
}

unsafe impl Trace for SomClass {
    custom_trace!(this, {
        if let Some(c) = &this.class      { mark(c); }
        if let Some(sc) = &this.super_class { mark(sc); }
        for f in &this.fields             { mark(f); }
        for m in this.methods.values()    { mark(m); }
    });
}

impl PartialEq for SomClass {
    fn eq(&self, other: &Self) -> bool {
        let class_eq = match (&self.class, &other.class) {
            (Some(a), Some(b)) => Gc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let super_class_eq = match (&self.super_class, &other.super_class) {
            (Some(a), Some(b)) => Gc::ptr_eq(a, b),
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

// ---------------------------------------------------------------------------
// SomMethod
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Finalize)]
pub struct SomMethod {
    pub name: std::string::String,
    pub signature: std::string::String,
    pub holder: SomRef<SomClass>,
    pub parameters: Vec<std::string::String>,
    pub body: MethodBody,
}

unsafe impl Trace for SomMethod {
    custom_trace!(this, {
        mark(&this.holder);
        mark(&this.body);
    });
}

impl SomMethod {
    pub fn is_primitive(&self) -> bool {
        matches!(self.body, MethodBody::Primitive(_))
    }
}

#[derive(Debug, Clone, Finalize)]
pub enum MethodBody {
    Ast(crate::ast::Block),
    Primitive(fn(&Value, Vec<Value>, &crate::universe::Universe, &crate::interpreter::Interpreter) -> anyhow::Result<crate::interpreter::ReturnValue>),
}

unsafe impl Trace for MethodBody {
    // Neither the AST (pure data) nor function pointers contain GC refs.
    unsafe_empty_trace!();
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

// ---------------------------------------------------------------------------
// CompiledBlockInstance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Finalize)]
pub struct CompiledBlockInstance {
    pub block: crate::bytecode::CompiledBlock,
    pub context: Option<SomRef<crate::bytecode_interpreter::Frame>>,
}

unsafe impl Trace for CompiledBlockInstance {
    custom_trace!(this, {
        if let Some(ctx) = &this.context { mark(ctx); }
    });
}

impl PartialEq for CompiledBlockInstance {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block
            && match (&self.context, &other.context) {
                (Some(a), Some(b)) => Gc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

// ---------------------------------------------------------------------------
// SomBlock
// ---------------------------------------------------------------------------

#[derive(Debug, Derivative, Finalize)]
#[derivative(PartialEq)]
pub struct SomBlock {
    pub body: crate::ast::Block,
    pub context: Option<SomRef<Activation>>,
}

unsafe impl Trace for SomBlock {
    custom_trace!(this, {
        if let Some(ctx) = &this.context { mark(ctx); }
    });
}

// ---------------------------------------------------------------------------
// Activation  (call-frame / lexical context)
// ---------------------------------------------------------------------------

#[derive(Debug, Derivative, Finalize)]
#[derivative(PartialEq)]
pub struct Activation {
    pub holder: Option<SomRef<SomClass>>,
    pub self_val: Value,
    pub args: HashMap<std::string::String, Value>,
    pub locals: HashMap<std::string::String, Value>,
    pub parent: Option<SomRef<Activation>>,
    pub is_active: bool,
}

unsafe impl Trace for Activation {
    custom_trace!(this, {
        if let Some(h) = &this.holder  { mark(h); }
        mark(&this.self_val);
        for v in this.args.values()    { mark(v); }
        for v in this.locals.values()  { mark(v); }
        if let Some(p) = &this.parent  { mark(p); }
    });
}

// ---------------------------------------------------------------------------
// Value helpers
// ---------------------------------------------------------------------------

impl Value {
    pub fn new_string(s: std::string::String) -> Self {
        Value::String(som_ref(s))
    }

    #[allow(dead_code)]
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
}
