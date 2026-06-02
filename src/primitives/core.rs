use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::{Result, anyhow};
use gc::Gc;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("Object>>perform:".to_string(), obj_perform);
    prims.insert("Object>>perform:withArguments:".to_string(), obj_perform);
    prims.insert("Object>>perform:inSuperclass:".to_string(), obj_perform_in_superclass);
    prims.insert("Object>>instVarAt:".to_string(), obj_inst_var_at);
    prims.insert("Object>>instVarAt:put:".to_string(), obj_inst_var_at_put);
    prims.insert("Object>>class".to_string(), obj_class);
    prims.insert("Object>>==".to_string(), obj_eq);
    prims.insert("Object>>hashcode".to_string(), obj_hashcode);
    prims.insert("Object>>objectSize".to_string(), obj_object_size);
    prims.insert("Object>>respondsTo:".to_string(), obj_responds_to);
    prims.insert("Object>>halt".to_string(), crate::primitives::debugger::dbg_halt);

    prims.insert("Class>>new".to_string(), class_new);
    prims.insert("Class>>name".to_string(), class_name);
    prims.insert("Class>>superclass".to_string(), class_superclass);
    prims.insert("Class>>fields".to_string(), class_fields);
    prims.insert("Class>>methods".to_string(), class_methods);
    prims.insert("Class>>hasMethod:".to_string(), class_has_method);
    prims.insert("Class>>selectors".to_string(), class_selectors);

    prims.insert("Method>>signature".to_string(), method_signature);
    prims.insert("Method>>holder".to_string(), method_holder);
    prims.insert("Method>>source".to_string(), method_source);
    prims.insert("Primitive>>signature".to_string(), method_signature);
    prims.insert("Primitive>>holder".to_string(), method_holder);
    prims.insert("Primitive>>source".to_string(), method_source);

    prims.insert("Nil>>asString".to_string(), nil_as_string);
    prims.insert("Symbol>>asString".to_string(), symbol_as_string);

    prims.insert("True>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifFalse:".to_string(), bool_if_false);
    prims.insert("True>>ifFalse:".to_string(), bool_if_false);
    prims.insert("Boolean>>ifTrue:ifFalse:".to_string(), bool_if_true_if_false);
    prims.insert("True>>not".to_string(), true_not);
    prims.insert("False>>not".to_string(), false_not);

    prims.insert("Block>>whileTrue:".to_string(), block_while_true);
    prims.insert("Block>>whileFalse:".to_string(), block_while_false);
    prims.insert("Block>>restart".to_string(), block_restart);
    prims.insert("Block>>value".to_string(), block_value);
    prims.insert("Block>>value:".to_string(), block_value);
    prims.insert("Block>>value:with:".to_string(), block_value);
}

fn obj_perform(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let selector = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let mut perform_args = Vec::new();
        if args.len() > 1 {
            if let Value::Array(arr) = &args[1] {
                perform_args.extend(arr.borrow().iter().cloned());
            } else {
                perform_args.push(args[1].clone());
            }
        }
        interpreter.dispatch_internal(self_val.clone(), &selector, perform_args)
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn obj_perform_in_superclass(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Some(arg_sel), Some(Value::Class(cls))) = (args.get(0), args.get(1)) {
        let selector = match arg_sel {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        let method = interpreter.lookup_method(cls.clone(), &selector)?;
        interpreter.run_method_internal(method, self_val.clone(), Vec::new())
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn obj_inst_var_at(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Object(obj), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
        let i = idx.to_usize().unwrap_or(0);
        if i > 0 && i <= obj.borrow().fields.len() {
            return Ok(ReturnValue::Value(obj.borrow().fields[i - 1].clone()));
        }
    } else if let (Value::Class(cls), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
        let i = idx.to_usize().unwrap_or(0);
        if i > 0 && i <= cls.borrow().fields.len() {
            return Ok(ReturnValue::Value(cls.borrow().fields[i - 1].clone()));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn obj_inst_var_at_put(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Object(obj), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
        let i = idx.to_usize().unwrap_or(0);
        if i > 0 && i <= obj.borrow().fields.len() {
            obj.borrow_mut().fields[i - 1] = val.clone();
            return Ok(ReturnValue::Value(val.clone()));
        }
    } else if let (Value::Class(cls), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
        let i = idx.to_usize().unwrap_or(0);
        if i > 0 && i <= cls.borrow().fields.len() {
            cls.borrow_mut().fields[i - 1] = val.clone();
            return Ok(ReturnValue::Value(val.clone()));
        }
    }
    Ok(ReturnValue::Value(Value::Nil))
}

fn obj_class(self_val: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let cls_name = match self_val {
        Value::Integer(_) => "Integer",
        Value::String(_) => "String",
        Value::Boolean(true) => "True",
        Value::Boolean(false) => "False",
        Value::Nil => "Nil",
        Value::Double(_) => "Double",
        Value::Object(obj) => {
            return Ok(ReturnValue::Value(Value::Class(obj.borrow().class.clone())));
        }
        Value::Array(_) => "Array",
        Value::Class(cls) => {
            return Ok(ReturnValue::Value(Value::Class(cls.borrow().class.as_ref().unwrap().clone())));
        }
        Value::Block(b) => {
            let params = b.borrow().body.parameters.len();
            match params {
                0 => "Block1",
                1 => "Block2",
                2 => "Block3",
                _ => "Block",
            }
        }
        Value::CompiledBlock(b) => {
            let params = b.borrow().block.num_args;
            match params {
                0 => "Block1",
                1 => "Block2",
                2 => "Block3",
                _ => "Block",
            }
        }
        Value::Symbol(_) => "Symbol",
        Value::Method(m) => {
            if m.borrow().is_primitive() { "Primitive" } else { "Method" }
        }
        Value::NativeHandle(_) => "Object",
    };
    Ok(ReturnValue::Value(Value::Class(universe.load_class(cls_name)?)))
}

fn obj_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(other) = args.get(0) {
        Ok(ReturnValue::Value(Value::Boolean(self_val == other)))
    } else {
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }
}

pub fn obj_hashcode(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let h = match self_val {
        Value::Integer(i) => (i % BigInt::from(0x7FFFFFFF)).to_i64().unwrap_or(0),
        Value::Double(d) => d.to_bits() as i64,
        Value::String(s) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            s.borrow().hash(&mut hasher);
            hasher.finish() as i64
        }
        Value::Symbol(s) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish() as i64
        }
        Value::Boolean(b) => if *b { 1 } else { 0 },
        Value::Nil => 0,
        Value::Object(obj) => Gc::as_ptr(obj) as i64,
        Value::Class(cls) => Gc::as_ptr(cls) as i64,
        Value::Array(arr) => Gc::as_ptr(arr) as i64,
        Value::Method(m) => Gc::as_ptr(m) as i64,
        Value::Block(b) => Gc::as_ptr(b) as i64,
        Value::CompiledBlock(b) => Gc::as_ptr(b) as i64,
        Value::NativeHandle(h) => *h as i64,
    };
    Ok(ReturnValue::Value(Value::Integer(BigInt::from(h & 0x7FFFFFFF))))
}

fn obj_object_size(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    Ok(ReturnValue::Value(Value::Integer(BigInt::from(16))))
}

fn obj_responds_to(self_val: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(arg) = args.get(0) {
        let selector = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        let mut cls_opt = match self_val {
            Value::Integer(_) => universe.get_global("Integer"),
            Value::Double(_) => universe.get_global("Double"),
            Value::String(_) => universe.get_global("String"),
            Value::Symbol(_) => universe.get_global("Symbol"),
            Value::Boolean(true) => universe.get_global("True"),
            Value::Boolean(false) => universe.get_global("False"),
            Value::Nil => universe.get_global("Nil"),
            Value::Array(_) => universe.get_global("Array"),
            Value::Block(_) => universe.get_global("Block"),
            Value::CompiledBlock(_) => universe.get_global("Block"),
            Value::Object(obj) => Some(Value::Class(obj.borrow().class.clone())),
            Value::Class(cls) => cls.borrow().class.as_ref().map(|mc| Value::Class(mc.clone())),
            Value::Method(_) => universe.get_global("Method"),
            Value::NativeHandle(_) => universe.get_global("Object"),
        };

        while let Some(Value::Class(cls)) = cls_opt {
            if cls.borrow().methods.contains_key(&selector) {
                return Ok(ReturnValue::Value(Value::Boolean(true)));
            }
            cls_opt = cls.borrow().super_class.as_ref().map(|s| Value::Class(s.clone()));
        }
    }
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn class_new(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        let instance = som_ref(SomObject {
            class: cls.clone(),
            fields: vec![Value::Nil; cls.borrow().instance_fields.len()],
        });
        Ok(ReturnValue::Value(Value::Object(instance)))
    } else {
        Err(anyhow!("new can only be sent to classes"))
    }
}

fn class_name(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        Ok(ReturnValue::Value(Value::Symbol(cls.borrow().name.clone())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn class_superclass(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        match &cls.borrow().super_class {
            Some(sc) => Ok(ReturnValue::Value(Value::Class(sc.clone()))),
            None => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn class_fields(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        let fields: Vec<Value> = cls.borrow().instance_fields.iter().map(|f| Value::Symbol(f.clone())).collect();
        Ok(ReturnValue::Value(Value::Array(som_ref(fields))))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn class_methods(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        let cls_ref = cls.borrow();
        let methods: Vec<Value> = cls_ref.method_order.iter().map(|name| Value::Method(cls_ref.methods.get(name).unwrap().clone())).collect();
        Ok(ReturnValue::Value(Value::Array(som_ref(methods))))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn class_has_method(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Class(cls), Some(arg)) = (self_val, args.get(0)) {
        let selector = match arg {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        Ok(ReturnValue::Value(Value::Boolean(cls.borrow().methods.contains_key(&selector))))
    } else {
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }
}

fn class_selectors(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Class(cls) = self_val {
        let selectors: Vec<Value> = cls.borrow().methods.keys().map(|s| Value::Symbol(s.clone())).collect();
        Ok(ReturnValue::Value(Value::Array(som_ref(selectors))))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn method_signature(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Method(m) = self_val {
        Ok(ReturnValue::Value(Value::Symbol(m.borrow().signature.clone())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn method_holder(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Method(m) = self_val {
        Ok(ReturnValue::Value(Value::Class(m.borrow().holder.clone())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn method_source(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Method(m) = self_val {
        match &m.borrow().source {
            Some(s) => Ok(ReturnValue::Value(Value::new_string(s.clone()))),
            None => Ok(ReturnValue::Value(Value::Nil)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn nil_as_string(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    Ok(ReturnValue::Value(Value::new_string("nil".to_string())))
}

fn symbol_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    match self_val {
        Value::String(s) => Ok(ReturnValue::Value(Value::new_string(s.borrow().clone()))),
        Value::Symbol(s) => Ok(ReturnValue::Value(Value::new_string(s.clone()))),
        _ => Ok(ReturnValue::Value(Value::Nil)),
    }
}

fn bool_if_true(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Boolean(true), Some(Value::Block(b))) = (self_val, args.get(0)) {
        interpreter.run_block(b.clone(), Vec::new())
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn bool_if_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Boolean(false), Some(Value::Block(b))) = (self_val, args.get(0)) {
        interpreter.run_block(b.clone(), Vec::new())
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn bool_if_true_if_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Boolean(b), Some(Value::Block(true_block)), Some(Value::Block(false_block))) = (self_val, args.get(0), args.get(1)) {
        if *b {
            interpreter.run_block(true_block.clone(), Vec::new())
        } else {
            interpreter.run_block(false_block.clone(), Vec::new())
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn true_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    Ok(ReturnValue::Value(Value::Boolean(false)))
}

fn false_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    Ok(ReturnValue::Value(Value::Boolean(true)))
}

fn block_while_true(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Block(cond), Some(Value::Block(body))) = (self_val, args.get(0)) {
        loop {
            match interpreter.run_block(cond.clone(), Vec::new())? {
                ReturnValue::Value(Value::Boolean(true)) => {
                    match interpreter.run_block(body.clone(), Vec::new())? {
                        ReturnValue::Restart => continue,
                        ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                        _ => {}
                    }
                }
                _ => break,
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn block_while_false(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Block(cond), Some(Value::Block(body))) = (self_val, args.get(0)) {
        loop {
            match interpreter.run_block(cond.clone(), Vec::new())? {
                ReturnValue::Value(Value::Boolean(false)) => {
                    match interpreter.run_block(body.clone(), Vec::new())? {
                        ReturnValue::Restart => continue,
                        ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                        _ => {}
                    }
                }
                _ => break,
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn block_restart(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    Ok(ReturnValue::Restart)
}

fn block_value(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
    if let Value::Block(b) = self_val {
        interpreter.run_block(b.clone(), args)
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}
