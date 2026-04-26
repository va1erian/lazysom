use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::object::Value;

/// The Acyclic Intermediate Representation of a SOM Object Graph
#[derive(Serialize, Deserialize, Debug)]
pub enum SerializedValue {
    Nil,
    Boolean(bool),
    // BigInts are safely serialized as base-10 strings to prevent precision loss in JSON
    Integer(String),
    Double(f64),
    Symbol(String),

    /// A pointer to an object defined elsewhere in the payload
    Ref(u32),

    /// Heap-allocated definitions
    DefString { id: u32, value: String },
    DefArray { id: u32, elements: Vec<SerializedValue> },
    DefObject {
        id: u32,
        class_name: String,
        fields: Vec<SerializedValue>
    },

    // Extensibility: Closures/Compiled blocks are notoriously hard to serialize.
    // For now, we serialize them as null or skip them, but the IR allows expansion.
    Unsupported(String),
}

pub struct SomSerializer {
    /// Maps memory addresses of Gc pointers to unique IDs
    seen_pointers: HashMap<usize, u32>,
    next_id: u32,
}

impl SomSerializer {
    pub fn new() -> Self {
        Self {
            seen_pointers: HashMap::new(),
            next_id: 0,
        }
    }

    /// Converts a SOM Value into the serializable IR
    pub fn serialize_value(&mut self, value: &Value) -> SerializedValue {
        match value {
            Value::Nil => SerializedValue::Nil,
            Value::Boolean(b) => SerializedValue::Boolean(*b),
            Value::Integer(i) => SerializedValue::Integer(i.to_str_radix(10)),
            Value::Double(d) => SerializedValue::Double(*d),
            Value::Symbol(s) => SerializedValue::Symbol(s.clone()),
            Value::String(s) => {
                let ptr = &**s as *const _ as usize as usize;
                if let Some(&id) = self.seen_pointers.get(&ptr) {
                    SerializedValue::Ref(id)
                } else {
                    let id = self.get_next_id(ptr);
                    SerializedValue::DefString { id, value: s.borrow().clone() }
                }
            },
            Value::Array(arr) => {
                let ptr = &**arr as *const _ as usize as usize;
                if let Some(&id) = self.seen_pointers.get(&ptr) {
                    return SerializedValue::Ref(id);
                }
                let id = self.get_next_id(ptr);

                // Recursively serialize elements
                let elements = arr.borrow().iter()
                    .map(|v| self.serialize_value(v))
                    .collect();

                SerializedValue::DefArray { id, elements }
            },
            Value::Object(obj) => {
                let ptr = &**obj as *const _ as usize as usize;
                if let Some(&id) = self.seen_pointers.get(&ptr) {
                    return SerializedValue::Ref(id);
                }
                let id = self.get_next_id(ptr);

                let obj_ref = obj.borrow();
                let class_name = obj_ref.class.borrow().name.clone();
                let fields = obj_ref.fields.iter()
                    .map(|f| self.serialize_value(f))
                    .collect();

                SerializedValue::DefObject { id, class_name, fields }
            },
            Value::NativeHandle(_) | Value::Method(_) | Value::Block(_) | Value::CompiledBlock(_) | Value::Class(_) => {
                // Runtime-specific states shouldn't usually be serialized over the wire
                SerializedValue::Unsupported("Runtime Context".to_string())
            }
        }
    }

    fn get_next_id(&mut self, ptr: usize) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.seen_pointers.insert(ptr, id);
        id
    }
}

pub fn to_json(value: &Value) -> Result<String, serde_json::Error> {
    let mut serializer = SomSerializer::new();
    let ir = serializer.serialize_value(value);
    serde_json::to_string_pretty(&ir)
}

pub fn to_msgpack(value: &Value) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    let mut serializer = SomSerializer::new();
    let ir = serializer.serialize_value(value);
    rmp_serde::to_vec(&ir)
}

use crate::universe::Universe;
use anyhow::{Result, anyhow};

use crate::object::{SomObject, som_ref};

pub struct SomDeserializer<'a> {
    universe: &'a Universe,
    /// Maps IDs to partially constructed values
    resolved_pointers: HashMap<u32, Value>,
}

impl<'a> SomDeserializer<'a> {
    pub fn new(universe: &'a Universe) -> Self {
        Self {
            universe,
            resolved_pointers: HashMap::new(),
        }
    }

    pub fn deserialize(&mut self, ir: &SerializedValue) -> Result<Value> {
        // Pass 1: instantiate shells
        self.instantiate_shells(ir)?;
        // Pass 2: populate content
        self.populate_content(ir)
    }

    fn instantiate_shells(&mut self, ir: &SerializedValue) -> Result<()> {
        match ir {
            SerializedValue::DefString { id, value: _ } => {
                // To maintain graph identity, we create an empty string and later fill it,
                // OR since strings are immutable in Rust SOM, we can just create it now.
                let val = Value::new_string(String::new());
                self.resolved_pointers.insert(*id, val);
            }
            SerializedValue::DefArray { id, elements } => {
                let val = Value::Array(som_ref(vec![Value::Nil; elements.len()]));
                self.resolved_pointers.insert(*id, val);
                for el in elements {
                    self.instantiate_shells(el)?;
                }
            }
            SerializedValue::DefObject { id, class_name, fields } => {
                let class = self.universe.load_class(class_name)?;
                let val = Value::Object(som_ref(SomObject {
                    class,
                    fields: vec![Value::Nil; fields.len()],
                }));
                self.resolved_pointers.insert(*id, val);
                for f in fields {
                    self.instantiate_shells(f)?;
                }
            }
            SerializedValue::Ref(_) | SerializedValue::Nil | SerializedValue::Boolean(_)
            | SerializedValue::Integer(_) | SerializedValue::Double(_)
            | SerializedValue::Symbol(_) | SerializedValue::Unsupported(_) => {}
        }
        Ok(())
    }

    fn populate_content(&mut self, ir: &SerializedValue) -> Result<Value> {
        match ir {
            SerializedValue::Nil => Ok(Value::Nil),
            SerializedValue::Boolean(b) => Ok(Value::Boolean(*b)),
            SerializedValue::Integer(s) => {
                use num_bigint::BigInt;
                use std::str::FromStr;
                let num = BigInt::from_str(s)
                    .map_err(|e| anyhow!("Failed to parse integer from string: {}", e))?;
                Ok(Value::Integer(num))
            }
            SerializedValue::Double(d) => Ok(Value::Double(*d)),
            SerializedValue::Symbol(s) => Ok(Value::Symbol(s.clone())),
            SerializedValue::Unsupported(msg) => Ok(Value::new_string(format!("Unsupported: {}", msg))),
            SerializedValue::Ref(id) => {
                if let Some(val) = self.resolved_pointers.get(id) {
                    Ok(val.clone())
                } else {
                    Err(anyhow!("Unresolved reference ID: {}", id))
                }
            }
            SerializedValue::DefString { id, value } => {
                let val = self.resolved_pointers.get(id)
                    .ok_or_else(|| anyhow!("Missing shell for string ID: {}", id))?;
                if let Value::String(s) = val {
                    *s.borrow_mut() = value.clone();
                }
                Ok(val.clone())
            }
            SerializedValue::DefArray { id, elements } => {
                let val = self.resolved_pointers.get(id)
                    .ok_or_else(|| anyhow!("Missing shell for array ID: {}", id))?
                    .clone();
                let mut populated_elements = Vec::new();
                for el in elements {
                    populated_elements.push(self.populate_content(el)?);
                }
                if let Value::Array(arr) = &val {
                    *arr.borrow_mut() = populated_elements;
                }
                Ok(val)
            }
            SerializedValue::DefObject { id, class_name: _, fields } => {
                let val = self.resolved_pointers.get(id)
                    .ok_or_else(|| anyhow!("Missing shell for object ID: {}", id))?
                    .clone();
                let mut populated_fields = Vec::new();
                for f in fields {
                    populated_fields.push(self.populate_content(f)?);
                }
                if let Value::Object(obj) = &val {
                    obj.borrow_mut().fields = populated_fields;
                }
                Ok(val)
            }
        }
    }
}
