use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::Result;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

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
        let start_idx = start.to_usize().unwrap_or(1);
        let end_idx = end.to_usize().unwrap_or(0);
        if start_idx == 0 || end_idx > s.len() { return Ok(ReturnValue::Value(Value::Nil)); }
        if end_idx < start_idx { return Ok(ReturnValue::Value(Value::new_string("".to_string()))); }
        Ok(ReturnValue::Value(Value::new_string(s[start_idx-1..end_idx].to_string())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}

fn arr_new(_: &Value, args: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Some(Value::Integer(len)) = args.get(0) {
        let l = len.to_usize().unwrap_or(0);
        Ok(ReturnValue::Value(Value::Array(som_ref(vec![Value::Nil; l]))))
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
