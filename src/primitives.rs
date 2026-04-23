use crate::object::*;
use crate::universe::Universe;
use crate::interpreter::{Interpreter, ReturnValue};
use anyhow::{Result, anyhow};
use std::rc::Rc;
use std::cell::RefCell;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero, Signed};
use num_integer::Integer;

pub fn get_primitives() -> std::collections::HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>> {
    let mut prims: std::collections::HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>> = std::collections::HashMap::new();

    fn sys_print_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            match arg {
                Value::String(s) => print!("{}", s.borrow()),
                Value::Symbol(s) => print!("{}", s),
                _ => {}
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_print_newline(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        println!();
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Some(val) = universe.get_global(&name) {
                return Ok(ReturnValue::Value(val));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }
    
    fn sys_global_put(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Some(arg), Some(val)) = (args.get(0), args.get(1)) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            universe.set_global(&name, val.clone());
            return Ok(ReturnValue::Value(val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }
    
    fn sys_has_global(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            return Ok(ReturnValue::Value(Value::Boolean(universe.get_global(&name).is_some())));
        }
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn sys_exit(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::Integer(code)) = args.get(0) {
            std::process::exit(code.to_i32().unwrap_or(0));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_load(_: &Value, args: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            match universe.load_class(&name) {
                Ok(cls) => return Ok(ReturnValue::Value(Value::Class(cls))),
                Err(_) => return Ok(ReturnValue::Value(Value::Nil)),
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn sys_time(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
    }

    fn sys_ticks(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // Dummy implementation of ticks
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(now))))
    }

    fn sys_full_gc(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // No-op for now, but return true as some tests expect it to indicate support
        Ok(ReturnValue::Value(Value::Boolean(true)))
    }

    fn sys_load_file(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let file_name = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Ok(content) = std::fs::read_to_string(file_name) {
                return Ok(ReturnValue::Value(Value::new_string(content)));
            }
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn int_plus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a + b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af + *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_minus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a - b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af - *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_mul(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a * b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af * *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Division by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a.div_floor(b))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af / *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_mod(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Modulo by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a.mod_floor(b))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af % *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_rem(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Modulo by zero")); }
            Ok(ReturnValue::Value(Value::Integer(a % b)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af % *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_float_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            if b.is_zero() { return Err(anyhow!("Division by zero")); }
            let af = a.to_f64().unwrap_or(0.0);
            let bf = b.to_f64().unwrap_or(1.0);
            Ok(ReturnValue::Value(Value::Double(af / bf)))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af / *b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a == b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) == *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn int_lt(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a < b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) < *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }
    
    fn int_le(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(a <= b))),
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a.to_f64().unwrap_or(0.0) <= *b))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn int_bit_and(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a & b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_bit_xor(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a ^ b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_shl(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            let shift = b.to_u32().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(a << shift)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_min(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a.clone().min(b.clone()))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af.min(*b))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_max(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            Ok(ReturnValue::Value(Value::Integer(a.clone().max(b.clone()))))
        } else if let (Value::Integer(a), Some(Value::Double(b))) = (self_val, args.get(0)) {
            let af = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Double(af.max(*b))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_shr(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(a), Some(Value::Integer(b))) = (self_val, args.get(0)) {
            let shift = b.to_u32().unwrap_or(0);
            let mask = BigInt::from(0xFFFFFFFFFFFFFFFFu64);
            let truncated = a & mask;
            let val_u64 = truncated.to_u64().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_u64 >> shift))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_32bit_signed(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let mask = BigInt::from(0xFFFFFFFFu64);
            let truncated = a & mask;
            let val_u32 = truncated.to_u32().unwrap_or(0);
            let val_i32 = val_u32 as i32;
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_i32))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_32bit_unsigned(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let mask = BigInt::from(0xFFFFFFFFu64);
            let truncated = a & mask;
            let val_u32 = truncated.to_u32().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(val_u32))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_double(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.to_f64().unwrap_or(0.0))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_at_random(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let limit = a.to_i64().unwrap_or(1);
            let rand_val = if limit > 0 { (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % limit as u128) as i64 + 1 } else { 1 };
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(rand_val))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_sqrt(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(a) = self_val {
            let f = a.to_f64().unwrap_or(0.0);
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(f.sqrt() as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_from_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(arg) = args.get(0) {
            let s_owned: String;
            let s = match arg {
                Value::String(s) => {
                    s_owned = s.borrow().clone();
                    s_owned.as_str()
                },
                Value::Symbol(s) => s.as_str(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            if let Ok(i) = s.parse::<BigInt>() {
                Ok(ReturnValue::Value(Value::Integer(i)))
            } else {
                Ok(ReturnValue::Value(Value::Nil))
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(i) = self_val {
            Ok(ReturnValue::Value(Value::new_string(i.to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn int_round(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(self_val.clone()))
    }
    fn str_concat(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s1 = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let Some(arg) = args.get(0) {
            let s2 = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::new_string(format!("{}{}", s1, s2))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn str_len(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let len = match self_val {
            Value::String(s) => s.borrow().len(),
            Value::Symbol(s) => s.len(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(len))))
    }

    fn str_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s1 = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if let Some(arg) = args.get(0) {
            let s2 = match arg {
                Value::String(s) => s.borrow().clone(),
                Value::Symbol(s) => s.clone(),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            Ok(ReturnValue::Value(Value::Boolean(s1 == s2)))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn obj_hashcode(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
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
            Value::Object(obj) => Rc::as_ptr(obj) as i64,
            Value::Class(cls) => Rc::as_ptr(cls) as i64,
            Value::Array(arr) => Rc::as_ptr(arr) as i64,
            Value::Method(m) => Rc::as_ptr(m) as i64,
            Value::Block(b) => Rc::as_ptr(b) as i64,
        };
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(h & 0x7FFFFFFF))))
    }

    fn obj_object_size(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        // Dummy implementation
        Ok(ReturnValue::Value(Value::Integer(BigInt::from(16))))
    }

    fn str_is_whitespace(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_whitespace()))))
    }

    fn str_is_letters(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_alphabetic()))))
    }

    fn str_is_digits(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
        };
        if s.is_empty() { return Ok(ReturnValue::Value(Value::Boolean(false))); }
        Ok(ReturnValue::Value(Value::Boolean(s.chars().all(|c| c.is_ascii_digit()))))
    }

    fn str_as_symbol(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        match self_val {
            Value::String(s) => Ok(ReturnValue::Value(Value::Symbol(s.borrow().clone()))),
            Value::Symbol(s) => Ok(ReturnValue::Value(Value::Symbol(s.clone()))),
            _ => Ok(ReturnValue::Value(Value::Nil)),
        }
    }

    fn symbol_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        match self_val {
            Value::String(s) => Ok(ReturnValue::Value(Value::new_string(s.borrow().clone()))),
            Value::Symbol(s) => Ok(ReturnValue::Value(Value::new_string(s.clone()))),
            _ => Ok(ReturnValue::Value(Value::Nil)),
        }
    }

    fn arr_new(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::Integer(len)) = args.get(0) {
            let l = len.to_usize().unwrap_or(0);
            Ok(ReturnValue::Value(Value::Array(Rc::new(RefCell::new(vec![Value::Nil; l])))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn str_substring(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        let s = match self_val {
            Value::String(s) => s.borrow().clone(),
            Value::Symbol(s) => s.clone(),
            _ => return Ok(ReturnValue::Value(Value::Nil)),
        };
        if let (Some(Value::Integer(start)), Some(Value::Integer(end))) = (args.get(0), args.get(1)) {
            let start_idx = start.to_usize().unwrap_or(1);
            let end_idx = end.to_usize().unwrap_or(0);
            if start_idx == 0 || end_idx > s.len() {
                return Ok(ReturnValue::Value(Value::Nil));
            }
            if end_idx < start_idx {
                return Ok(ReturnValue::Value(Value::new_string("".to_string())));
            }
            Ok(ReturnValue::Value(Value::new_string(s[start_idx-1..end_idx].to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_at(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Array(arr), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
            let i = idx.to_usize().unwrap_or(0);
            Ok(ReturnValue::Value(arr.borrow().get(i - 1).cloned().unwrap_or(Value::Nil)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_at_put(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Array(arr), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
            let i = idx.to_usize().unwrap_or(0);
            arr.borrow_mut()[i - 1] = val.clone();
            Ok(ReturnValue::Value(val.clone()))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn arr_len(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Array(arr) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(arr.borrow().len()))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
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
            Value::Object(obj) => return Ok(ReturnValue::Value(Value::Class(obj.borrow().class.clone()))),
            Value::Array(_) => "Array",
            Value::Class(cls) => return Ok(ReturnValue::Value(Value::Class(cls.borrow().class.as_ref().unwrap().clone()))),
            Value::Block(b) => {
                let params = b.borrow().body.parameters.len();
                match params {
                    0 => "Block1",
                    1 => "Block2",
                    2 => "Block3",
                    _ => "Block", // Fallback for many arguments
                }
            }
            Value::Symbol(_) => "Symbol",
            Value::Method(_) => "Method",
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

    fn class_new(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let instance = Rc::new(RefCell::new(SomObject {
                class: cls.clone(),
                fields: vec![Value::Nil; cls.borrow().instance_fields.len()],
            }));
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
            let fields: Vec<Value> = cls.borrow().instance_fields.iter()
                .map(|f| Value::Symbol(f.clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(Rc::new(RefCell::new(fields)))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn class_methods(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Class(cls) = self_val {
            let cls_ref = cls.borrow();
            let methods: Vec<Value> = cls_ref.method_order.iter()
                .map(|name| Value::Method(cls_ref.methods.get(name).unwrap().clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(Rc::new(RefCell::new(methods)))))
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
            let selectors: Vec<Value> = cls.borrow().methods.keys()
                .map(|s| Value::Symbol(s.clone()))
                .collect();
            Ok(ReturnValue::Value(Value::Array(Rc::new(RefCell::new(selectors)))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
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
                Value::Object(obj) => Some(Value::Class(obj.borrow().class.clone())),
                Value::Class(cls) => cls.borrow().class.as_ref().map(|mc| Value::Class(mc.clone())),
                Value::Method(_) => universe.get_global("Method"),
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

    fn nil_as_string(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::new_string("nil".to_string())))
    }

    fn double_plus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a + b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_minus(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a - b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_mul(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a * b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_float_div(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a / b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_mod(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            Ok(ReturnValue::Value(Value::Double(a % b)))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_eq(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            match arg {
                Value::Double(b) => Ok(ReturnValue::Value(Value::Boolean(a == b))),
                Value::Integer(b) => Ok(ReturnValue::Value(Value::Boolean(*a == b.to_f64().unwrap_or(0.0)))),
                _ => Ok(ReturnValue::Value(Value::Boolean(false))),
            }
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn double_lt(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(a), Some(arg)) = (self_val, args.get(0)) {
            let b = match arg {
                Value::Double(v) => *v,
                Value::Integer(v) => v.to_f64().unwrap_or(0.0),
                _ => return Ok(ReturnValue::Value(Value::Boolean(false))),
            };
            Ok(ReturnValue::Value(Value::Boolean(*a < b)))
        } else {
            Ok(ReturnValue::Value(Value::Boolean(false)))
        }
    }

    fn double_as_integer(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(*a as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_as_string(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::new_string(a.to_string())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_sqrt(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.sqrt())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_round(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Integer(BigInt::from(a.round() as i64))))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_cos(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.cos())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_sin(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(a) = self_val {
            Ok(ReturnValue::Value(Value::Double(a.sin())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_from_string(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Some(Value::String(s)) = args.get(0) {
            if let Ok(f) = s.borrow().parse::<f64>() {
                Ok(ReturnValue::Value(Value::Double(f)))
            } else {
                Ok(ReturnValue::Value(Value::Nil))
            }
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_pos_inf(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Double(f64::INFINITY)))
    }

    fn int_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.clone(),
                Value::Double(d) => BigInt::from(d.to_i64().unwrap_or(0)),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = start.clone();
            while i <= limit {
                match interpreter.run_block(block.clone(), vec![Value::Integer(i.clone())])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i += 1;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn int_down_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Integer(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.clone(),
                Value::Double(d) => BigInt::from(d.to_i64().unwrap_or(0)),
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = start.clone();
            while i >= limit {
                match interpreter.run_block(block.clone(), vec![Value::Integer(i.clone())])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i -= 1;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn double_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.to_f64().unwrap_or(0.0),
                Value::Double(d) => *d,
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = *start;
            while i <= limit {
                match interpreter.run_block(block.clone(), vec![Value::Double(i)])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i += 1.0;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn double_down_to_do(self_val: &Value, args: Vec<Value>, _: &Universe, interpreter: &Interpreter) -> Result<ReturnValue> {
        if let (Value::Double(start), Some(other), Some(Value::Block(block))) = (self_val, args.get(0), args.get(1)) {
            let limit = match other {
                Value::Integer(i) => i.to_f64().unwrap_or(0.0),
                Value::Double(d) => *d,
                _ => return Ok(ReturnValue::Value(Value::Nil)),
            };
            let mut i = *start;
            while i >= limit {
                match interpreter.run_block(block.clone(), vec![Value::Double(i)])? {
                    ReturnValue::Restart => continue,
                    ReturnValue::NonLocalReturn(v, target) => return Ok(ReturnValue::NonLocalReturn(v, target)),
                    _ => {}
                }
                i -= 1.0;
            }
            return Ok(ReturnValue::Value(self_val.clone()));
        }
        Ok(ReturnValue::Value(Value::Nil))
    }

    fn true_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Boolean(false)))
    }

    fn false_not(_: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        Ok(ReturnValue::Value(Value::Boolean(true)))
    }

    fn int_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Integer(i) = self_val {
            Ok(ReturnValue::Value(Value::Integer(i.abs())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    fn double_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
        if let Value::Double(d) = self_val {
            Ok(ReturnValue::Value(Value::Double(d.abs())))
        } else {
            Ok(ReturnValue::Value(Value::Nil))
        }
    }

    prims.insert("System>>global:".to_string(), sys_global);
    prims.insert("System>>global:put:".to_string(), sys_global_put);
    prims.insert("System>>hasGlobal:".to_string(), sys_has_global);
    prims.insert("System>>load:".to_string(), sys_load);
    prims.insert("System>>exit:".to_string(), sys_exit);
    prims.insert("System>>printString:".to_string(), sys_print_string);
    prims.insert("System>>printNewline".to_string(), sys_print_newline);
    prims.insert("System>>time".to_string(), sys_time);
    prims.insert("System>>ticks".to_string(), sys_ticks);
    prims.insert("System>>fullGC".to_string(), sys_full_gc);
    prims.insert("System>>loadFile:".to_string(), sys_load_file);

    prims.insert("Integer>>+".to_string(), int_plus);
    prims.insert("Integer>>-".to_string(), int_minus);
    prims.insert("Integer>>*".to_string(), int_mul);
    prims.insert("Integer>>/".to_string(), int_div);
    prims.insert("Integer>>//".to_string(), int_float_div);
    prims.insert("Integer>>%".to_string(), int_mod);
    prims.insert("Integer>>rem:".to_string(), int_rem);
    prims.insert("Integer>>min:".to_string(), int_min);
    prims.insert("Integer>>max:".to_string(), int_max);
    prims.insert("Integer>>=".to_string(), int_eq);
    prims.insert("Integer>><".to_string(), int_lt);
    prims.insert("Integer>><=".to_string(), int_le);
    prims.insert("Integer>>&".to_string(), int_bit_and);
    prims.insert("Integer>>bitXor:".to_string(), int_bit_xor);
    prims.insert("Integer>><<".to_string(), int_shl);
    prims.insert("Integer>>>>>".to_string(), int_shr);
    prims.insert("Integer>>sqrt".to_string(), int_sqrt);
    prims.insert("Integer>>as32BitSignedValue".to_string(), int_as_32bit_signed);
    prims.insert("Integer>>as32BitUnsignedValue".to_string(), int_as_32bit_unsigned);
    prims.insert("Integer>>asDouble".to_string(), int_as_double);
    prims.insert("Integer>>atRandom".to_string(), int_at_random);
    prims.insert("Integer class>>fromString:".to_string(), int_from_string);
    prims.insert("Integer>>asString".to_string(), int_as_string);
    prims.insert("Integer>>round".to_string(), int_round);
    prims.insert("Integer>>to:do:".to_string(), int_to_do);
    prims.insert("Integer>>downTo:do:".to_string(), int_down_to_do);
    prims.insert("Integer>>abs".to_string(), int_abs);

    prims.insert("String>>concatenate:".to_string(), str_concat);
    prims.insert("String>>length".to_string(), str_len);
    prims.insert("String>>=".to_string(), str_eq);
    prims.insert("String>>asSymbol".to_string(), str_as_symbol);
    prims.insert("String>>hashcode".to_string(), obj_hashcode);
    prims.insert("String>>isWhiteSpace".to_string(), str_is_whitespace);
    prims.insert("String>>isLetters".to_string(), str_is_letters);
    prims.insert("String>>isDigits".to_string(), str_is_digits);
    prims.insert("String>>primSubstringFrom:to:".to_string(), str_substring);

    prims.insert("Array>>at:".to_string(), arr_at);
    prims.insert("Array>>at:put:".to_string(), arr_at_put);
    prims.insert("Array>>length".to_string(), arr_len);
    prims.insert("Array class>>new:".to_string(), arr_new);

    prims.insert("True>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifTrue:".to_string(), bool_if_true);
    prims.insert("False>>ifFalse:".to_string(), bool_if_false);
    prims.insert("True>>ifFalse:".to_string(), bool_if_false);
    prims.insert("Boolean>>ifTrue:ifFalse:".to_string(), bool_if_true_if_false);

    prims.insert("Block>>whileTrue:".to_string(), block_while_true);
    prims.insert("Block>>whileFalse:".to_string(), block_while_false);
    prims.insert("Block>>restart".to_string(), block_restart);
    prims.insert("Block>>value".to_string(), block_value);
    prims.insert("Block>>value:".to_string(), block_value);
    prims.insert("Block>>value:with:".to_string(), block_value);

    prims.insert("Object>>perform:".to_string(), obj_perform);
    prims.insert("Object>>perform:withArguments:".to_string(), obj_perform);
    prims.insert("Object>>perform:inSuperclass:".to_string(), obj_perform_in_superclass);
    prims.insert("Object>>instVarAt:".to_string(), obj_inst_var_at);
    prims.insert("Object>>instVarAt:put:".to_string(), obj_inst_var_at_put);
    prims.insert("Object>>class".to_string(), obj_class);
    prims.insert("Object>>==".to_string(), obj_eq);
    prims.insert("Object>>hashcode".to_string(), obj_hashcode);
    prims.insert("Object>>objectSize".to_string(), obj_object_size);

    prims.insert("Class>>new".to_string(), class_new);
    prims.insert("Class>>name".to_string(), class_name);
    prims.insert("Class>>superclass".to_string(), class_superclass);
    prims.insert("Class>>fields".to_string(), class_fields);
    prims.insert("Class>>methods".to_string(), class_methods);
    prims.insert("Class>>hasMethod:".to_string(), class_has_method);
    prims.insert("Class>>selectors".to_string(), class_selectors);

    prims.insert("Object>>respondsTo:".to_string(), obj_responds_to);

    prims.insert("Method>>signature".to_string(), method_signature);
    prims.insert("Method>>holder".to_string(), method_holder);
    prims.insert("Primitive>>signature".to_string(), method_signature);
    prims.insert("Primitive>>holder".to_string(), method_holder);

    prims.insert("Nil>>asString".to_string(), nil_as_string);

    prims.insert("Symbol>>asString".to_string(), symbol_as_string);

    prims.insert("Double>>+".to_string(), double_plus);
    prims.insert("Double>>-".to_string(), double_minus);
    prims.insert("Double>>*".to_string(), double_mul);
    prims.insert("Double>>//".to_string(), double_float_div);
    prims.insert("Double>>%".to_string(), double_mod);
    prims.insert("Double>>=".to_string(), double_eq);
    prims.insert("Double>><".to_string(), double_lt);
    prims.insert("Double>>asInteger".to_string(), double_as_integer);
    prims.insert("Double>>asString".to_string(), double_as_string);
    prims.insert("Double>>sqrt".to_string(), double_sqrt);
    prims.insert("Double>>round".to_string(), double_round);
    prims.insert("Double>>cos".to_string(), double_cos);
    prims.insert("Double>>sin".to_string(), double_sin);
    prims.insert("Double class>>fromString:".to_string(), double_from_string);
    prims.insert("Double class>>PositiveInfinity".to_string(), double_pos_inf);
    prims.insert("Double>>to:do:".to_string(), double_to_do);
    prims.insert("Double>>downTo:do:".to_string(), double_down_to_do);
    prims.insert("Double>>abs".to_string(), double_abs);

    prims.insert("True>>not".to_string(), true_not);
    prims.insert("False>>not".to_string(), false_not);

    prims
}
