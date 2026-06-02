use crate::interpreter::{Interpreter, ReturnValue};
use crate::object::*;
use crate::universe::Universe;
use anyhow::Result;
use std::collections::HashMap;

pub fn register(prims: &mut HashMap<String, fn(&Value, Vec<Value>, &Universe, &Interpreter) -> Result<ReturnValue>>) {
    prims.insert("Debugger>>resume".to_string(), dbg_resume);
    prims.insert("Debugger>>stepInto".to_string(), dbg_step_into);
    prims.insert("Debugger>>currentFrames".to_string(), dbg_current_frames);
    prims.insert("Debugger class>>halt".to_string(), dbg_halt);
    prims.insert("Object>>halt".to_string(), dbg_halt);
}

pub fn dbg_resume(_: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    *universe.vm_state.borrow_mut() = crate::universe::VmState::Running;
    Ok(ReturnValue::Value(Value::Nil))
}

pub fn dbg_step_into(_: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    *universe.vm_state.borrow_mut() = crate::universe::VmState::Stepping;
    Ok(ReturnValue::Value(Value::Nil))
}

pub fn dbg_current_frames(_: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    let frames = universe.active_frames.borrow();
    let mut ret = Vec::new();
    for f in frames.iter().rev() {
        let name = f.borrow().method_name.clone();
        ret.push(Value::new_string(name));
    }
    Ok(ReturnValue::Value(Value::Array(crate::object::som_ref(ret))))
}

pub fn dbg_halt(_: &Value, _: Vec<Value>, universe: &Universe, _: &Interpreter) -> Result<ReturnValue> {
    *universe.vm_state.borrow_mut() = crate::universe::VmState::Stepping;
    Ok(ReturnValue::Value(Value::Nil))
}
