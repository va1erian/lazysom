use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::{Result, anyhow};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Signed};
use std::collections::HashMap;

/// Safety cap for `Array new:` so a hostile/huge size request fails with a
/// clear error instead of trying to allocate hundreds of gigabytes and
/// aborting the process via an allocator failure.
const MAX_ARRAY_SIZE: usize = 100_000_000;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("String>>concatenate:".to_string(), str_concat);
    prims.insert("String>>length".to_string(), str_len);
    prims.insert("String>>=".to_string(), str_eq);
    prims.insert("String>>asSymbol".to_string(), str_as_symbol);
    prims.insert("String>>isWhiteSpace".to_string(), str_is_whitespace);
    prims.insert("String>>isLetters".to_string(), str_is_letters);
    prims.insert("String>>isDigits".to_string(), str_is_digits);
    prims.insert("String>>primSubstringFrom:to:".to_string(), str_substring);
    prims.insert("String>>hashcode".to_string(), crate::primitives::core::obj_hashcode);

    prims.insert("Array>>at:".to_string(), arr_at);
    prims.insert("Array>>at:put:".to_string(), arr_at_put);
    prims.insert("Array>>length".to_string(), arr_len);
    prims.insert("Array class>>new:".to_string(), arr_new);
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
    // Counted in characters (not bytes) so it stays consistent with
    // primSubstringFrom:to: below when the string contains multi-byte
    // UTF-8 characters.
    let len = match self_val {
        Value::String(s) => s.borrow().chars().count(),
        Value::Symbol(s) => s.chars().count(),
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

fn str_substring(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let s = match self_val {
        Value::String(s) => s.borrow().clone(),
        Value::Symbol(s) => s.clone(),
        _ => return Ok(ReturnValue::Value(Value::Nil)),
    };
    if let (Some(Value::Integer(start)), Some(Value::Integer(end))) = (args.get(0), args.get(1)) {
        // Index by character, not by byte, so multi-byte UTF-8 characters
        // (e.g. emoji) never get sliced on a non-char boundary, which would
        // otherwise panic.
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len() as i64;
        let start_idx = start.to_i64().unwrap_or(if start.is_negative() { i64::MIN } else { i64::MAX });
        let end_idx = end.to_i64().unwrap_or(if end.is_negative() { i64::MIN } else { i64::MAX });
        if start_idx < 1 || end_idx > len {
            return Ok(ReturnValue::Value(Value::Nil));
        }
        if end_idx < start_idx {
            return Ok(ReturnValue::Value(Value::new_string("".to_string())));
        }
        let result: String = chars[(start_idx as usize - 1)..(end_idx as usize)].iter().collect();
        Ok(ReturnValue::Value(Value::new_string(result)))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn arr_new(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(len)) = args.get(0) {
        if len.is_negative() {
            return Err(anyhow!("Array class>>new: size must not be negative (got {})", len));
        }
        match len.to_usize() {
            Some(l) if l <= MAX_ARRAY_SIZE => Ok(ReturnValue::Value(Value::Array(som_ref(vec![Value::Nil; l])))),
            _ => Err(anyhow!("Array class>>new: size {} exceeds maximum allowed size ({})", len, MAX_ARRAY_SIZE)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

/// Resolves a SOM (1-based) index against `len`, returning the 0-based Rust
/// index, or `None` if it is out of bounds (including negative/huge indices
/// that don't fit in an i64).
fn resolve_index(idx: &BigInt, len: usize) -> Option<usize> {
    let i = idx.to_i64()?;
    if i < 1 || (i as u64) > len as u64 {
        return None;
    }
    Some((i - 1) as usize)
}

fn arr_at(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Array(arr), Some(Value::Integer(idx))) = (self_val, args.get(0)) {
        let len = arr.borrow().len();
        match resolve_index(idx, len) {
            Some(i) => Ok(ReturnValue::Value(arr.borrow()[i].clone())),
            None => Err(anyhow!("Array>>at: index {} out of bounds (size {})", idx, len)),
        }
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn arr_at_put(self_val: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let (Value::Array(arr), Some(Value::Integer(idx)), Some(val)) = (self_val, args.get(0), args.get(1)) {
        let len = arr.borrow().len();
        match resolve_index(idx, len) {
            Some(i) => {
                arr.borrow_mut()[i] = val.clone();
                Ok(ReturnValue::Value(val.clone()))
            }
            None => Err(anyhow!("Array>>at:put: index {} out of bounds (size {})", idx, len)),
        }
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
