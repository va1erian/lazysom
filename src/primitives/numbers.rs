use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::{Result, anyhow};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero, Signed};
use std::collections::HashMap;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
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
        let rand_val = if limit > 0 {
            (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() % limit as u128) as i64 + 1
        } else {
            1
        };
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
            Value::String(s) => { s_owned = s.borrow().clone(); s_owned.as_str() },
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

fn int_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Integer(i) = self_val {
        Ok(ReturnValue::Value(Value::Integer(i.abs())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
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

fn double_abs(self_val: &Value, _: Vec<Value>, _: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    if let Value::Double(d) = self_val {
        Ok(ReturnValue::Value(Value::Double(d.abs())))
    } else {
        Ok(ReturnValue::Value(Value::Nil))
    }
}
